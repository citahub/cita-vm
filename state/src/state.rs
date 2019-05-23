use std::cell::RefCell;
use std::sync::Arc;

use cita_trie::db::DB;
use cita_trie::trie::{PatriciaTrie, Trie};
use ethereum_types::{Address, H256, U256};
use hashbrown::hash_map::Entry;
use hashbrown::{HashMap, HashSet};
use log::debug;
use rayon::prelude::*;

use super::account::StateObject;
use super::account_db::AccountDB;
use super::err::Error;
use super::hashlib;
use super::object_entry::{ObjectStatus, StateObjectEntry};

/// State is the one who managers all accounts and states in Ethereum's system.
pub struct State<B> {
    pub db: Arc<B>,
    pub root: H256,
    pub cache: RefCell<HashMap<Address, StateObjectEntry>>,
    /// Checkpoints are used to revert to history.
    pub checkpoints: RefCell<Vec<HashMap<Address, Option<StateObjectEntry>>>>,
}

impl<B: DB> State<B> {
    /// Creates empty state for test.
    pub fn new(db: Arc<B>) -> Result<State<B>, Error> {
        let mut trie = PatriciaTrie::new(Arc::clone(&db), hashlib::RLPNodeCodec::default());
        let root = trie.root()?;

        Ok(State {
            db,
            root: From::from(&root[..]),
            cache: RefCell::new(HashMap::new()),
            checkpoints: RefCell::new(Vec::new()),
        })
    }

    /// Creates new state with existing state root
    pub fn from_existing(db: Arc<B>, root: H256) -> Result<State<B>, Error> {
        if !db.contains(&root.0[..]).or_else(|e| Err(Error::DB(format!("{}", e))))? {
            return Err(Error::NotFound);
        }
        Ok(State {
            db,
            root,
            cache: RefCell::new(HashMap::new()),
            checkpoints: RefCell::new(Vec::new()),
        })
    }

    /// Create a contract account with code or not
    /// Overwrite the code if the contract already exists
    pub fn new_contract(&mut self, contract: &Address, balance: U256, nonce: U256, code: Vec<u8>) -> StateObject {
        debug!(
            "state.new_contract contract={:?} balance={:?}, nonce={:?} code={:?}",
            contract, balance, nonce, code
        );
        let mut state_object = StateObject::new(balance, nonce);
        state_object.init_code(code);

        self.insert_cache(contract, StateObjectEntry::new_dirty(Some(state_object.clone_dirty())));
        state_object
    }

    /// Kill a contract.
    pub fn kill_contract(&mut self, contract: &Address) {
        debug!("state.kill_contract contract={:?}", contract);
        self.insert_cache(contract, StateObjectEntry::new_dirty(None));
    }

    /// Remove any touched empty or dust accounts.
    pub fn kill_garbage(&mut self, inused: &HashSet<Address>) {
        for a in inused {
            if let Some(state_object_entry) = self.cache.borrow().get(a) {
                if state_object_entry.state_object.is_none() {
                    continue;
                }
            }
            if self.is_empty(a).unwrap_or(false) {
                self.kill_contract(a)
            }
        }
    }

    /// Clear cache
    /// Note that the cache is just a HashMap, so memory explosion will be
    /// happend if you never call `clear()`. You should decide for yourself
    /// when to call this function.
    pub fn clear(&mut self) {
        assert!(self.checkpoints.borrow().is_empty());
        self.cache.borrow_mut().clear();
    }

    /// Use a callback function to avoid clone data in caches.
    fn call_with_cached<F, U>(&self, address: &Address, f: F) -> Result<U, Error>
    where
        F: Fn(Option<&StateObject>) -> U,
    {
        if let Some(state_object_entry) = self.cache.borrow().get(address) {
            if let Some(state_object) = &state_object_entry.state_object {
                return Ok(f(Some(state_object)));
            } else {
                return Ok(f(None));
            }
        }
        let trie = PatriciaTrie::from(Arc::clone(&self.db), hashlib::RLPNodeCodec::default(), &self.root.0)?;
        match trie.get(hashlib::summary(&address[..]).as_slice())? {
            Some(rlp) => {
                let mut state_object = StateObject::from_rlp(&rlp)?;
                let accdb = Arc::new(AccountDB::new(*address, self.db.clone()));
                state_object.read_code(accdb)?;
                self.insert_cache(address, StateObjectEntry::new_clean(Some(state_object.clone_clean())));
                Ok(f(Some(&state_object)))
            }
            None => Ok(f(None)),
        }
    }

    /// Get state object.
    pub fn get_state_object(&self, address: &Address) -> Result<Option<StateObject>, Error> {
        if let Some(state_object_entry) = self.cache.borrow().get(address) {
            if let Some(state_object) = &state_object_entry.state_object {
                return Ok(Some((*state_object).clone_dirty()));
            }
        }
        let trie = PatriciaTrie::from(Arc::clone(&self.db), hashlib::RLPNodeCodec::default(), &self.root.0)?;
        match trie.get(hashlib::summary(&address[..]).as_slice())? {
            Some(rlp) => {
                let mut state_object = StateObject::from_rlp(&rlp)?;
                let accdb = Arc::new(AccountDB::new(*address, self.db.clone()));
                state_object.read_code(accdb)?;
                self.insert_cache(address, StateObjectEntry::new_clean(Some(state_object.clone_clean())));
                Ok(Some(state_object))
            }
            None => Ok(None),
        }
    }

    /// Get state object. If not exists, create a fresh one.
    pub fn get_state_object_or_default(&mut self, address: &Address) -> Result<StateObject, Error> {
        match self.get_state_object(address)? {
            Some(state_object) => Ok(state_object),
            None => {
                let state_object = self.new_contract(address, U256::zero(), U256::zero(), vec![]);
                Ok(state_object)
            }
        }
    }

    /// Get the merkle proof for a given account.
    pub fn get_account_proof(&self, address: &Address) -> Result<Vec<Vec<u8>>, Error> {
        let trie = PatriciaTrie::from(Arc::clone(&self.db), hashlib::RLPNodeCodec::default(), &self.root.0)?;
        let proof = trie.get_proof(hashlib::summary(&address[..]).as_slice())?;
        Ok(proof)
    }

    /// Get the storage proof for given account and key.
    pub fn get_storage_proof(&self, address: &Address, key: &H256) -> Result<Vec<Vec<u8>>, Error> {
        self.call_with_cached(address, |a| match a {
            Some(data) => {
                let accdb = Arc::new(AccountDB::new(*address, self.db.clone()));
                data.get_storage_proof(accdb, key)
            }
            None => Ok(vec![]),
        })?
    }

    /// Check if an account exists.
    pub fn exist(&mut self, address: &Address) -> Result<bool, Error> {
        self.call_with_cached(address, |a| Ok(a.is_some()))?
    }

    /// Check if an account is empty. Empty is defined according to
    /// EIP161 (balance = nonce = code = 0).
    #[allow(clippy::wrong_self_convention)]
    pub fn is_empty(&mut self, address: &Address) -> Result<bool, Error> {
        self.call_with_cached(address, |a| match a {
            Some(data) => Ok(data.is_empty()),
            None => Ok(true),
        })?
    }

    /// Set (key, value) in storage cache.
    pub fn set_storage(&mut self, address: &Address, key: H256, value: H256) -> Result<(), Error> {
        debug!(
            "state.set_storage address={:?} key={:?} value={:?}",
            address, key, value
        );
        let state_object = self.get_state_object_or_default(address)?;
        let accdb = Arc::new(AccountDB::new(*address, self.db.clone()));
        if state_object.get_storage(accdb, &key)? == Some(value) {
            return Ok(());
        }

        self.add_checkpoint(address);
        if let Some(ref mut state_object_entry) = self.cache.borrow_mut().get_mut(address) {
            match state_object_entry.state_object {
                Some(ref mut state_object) => {
                    state_object.set_storage(key, value);
                    state_object_entry.status = ObjectStatus::Dirty;
                }
                None => panic!("state object always exist in cache."),
            }
        }
        Ok(())
    }

    /// Set code for an account.
    pub fn set_code(&mut self, address: &Address, code: Vec<u8>) -> Result<(), Error> {
        debug!("state.set_code address={:?} code={:?}", address, code);
        let mut state_object = self.get_state_object_or_default(address)?;
        state_object.init_code(code);
        self.insert_cache(address, StateObjectEntry::new_dirty(Some(state_object)));
        Ok(())
    }

    /// Add balance by incr for an account.
    pub fn add_balance(&mut self, address: &Address, incr: U256) -> Result<(), Error> {
        debug!("state.add_balance a={:?} incr={:?}", address, incr);
        if incr.is_zero() {
            return Ok(());
        }
        let mut state_object = self.get_state_object_or_default(address)?;
        if state_object.balance.overflowing_add(incr).1 {
            return Err(Error::BalanceError);
        }
        state_object.add_balance(incr);
        self.insert_cache(address, StateObjectEntry::new_dirty(Some(state_object)));
        Ok(())
    }

    /// Sub balance by decr for an account.
    pub fn sub_balance(&mut self, a: &Address, decr: U256) -> Result<(), Error> {
        debug!("state.sub_balance a={:?} decr={:?}", a, decr);
        if decr.is_zero() {
            return Ok(());
        }
        let mut state_object = self.get_state_object_or_default(a)?;
        if state_object.balance.overflowing_sub(decr).1 {
            return Err(Error::BalanceError);
        }
        state_object.sub_balance(decr);
        self.insert_cache(a, StateObjectEntry::new_dirty(Some(state_object)));
        Ok(())
    }

    /// Transfer balance from `from` to `to` by `by`.
    pub fn transfer_balance(&mut self, from: &Address, to: &Address, by: U256) -> Result<(), Error> {
        self.sub_balance(from, by)?;
        self.add_balance(to, by)?;
        Ok(())
    }

    /// Increase nonce for an account.
    pub fn inc_nonce(&mut self, address: &Address) -> Result<(), Error> {
        debug!("state.inc_nonce a={:?}", address);
        let mut state_object = self.get_state_object_or_default(address)?;
        state_object.inc_nonce();
        self.insert_cache(address, StateObjectEntry::new_dirty(Some(state_object)));
        Ok(())
    }

    /// Insert a state object entry into cache.
    fn insert_cache(&self, address: &Address, state_object_entry: StateObjectEntry) {
        let is_dirty = state_object_entry.is_dirty();
        let old_entry = self
            .cache
            .borrow_mut()
            .insert(*address, state_object_entry.clone_dirty());

        if is_dirty {
            if let Some(checkpoint) = self.checkpoints.borrow_mut().last_mut() {
                checkpoint.entry(*address).or_insert(old_entry);
            }
        }
    }

    /// Flush the data from cache to database.
    pub fn commit(&mut self) -> Result<(), Error> {
        assert!(self.checkpoints.borrow().is_empty());

        // Firstly, update account storage tree
        for (address, entry) in self.cache.borrow_mut().iter_mut().filter(|&(_, ref a)| a.is_dirty()) {
            if let Some(ref mut state_object) = entry.state_object {
                let accdb = Arc::new(AccountDB::new(*address, self.db.clone()));
                state_object.commit_storage(Arc::clone(&accdb))?;
                state_object.commit_code(Arc::clone(&accdb))?;
            }
        }

        // Secondly, update the world state tree
        let mut trie = PatriciaTrie::from(Arc::clone(&self.db), hashlib::RLPNodeCodec::default(), &self.root.0)?;

        let addresses: Vec<Address> = self
            .cache
            .borrow()
            .iter()
            .filter(|&(_, ref a)| a.is_dirty())
            .map(|(a, _)| *a)
            .collect();
        let addresses_hash: Vec<Vec<u8>> = addresses.par_iter().map(|a| hashlib::summary(&a[..])).collect();
        for (i, (_, entry)) in self
            .cache
            .borrow_mut()
            .iter_mut()
            .filter(|&(_, ref a)| a.is_dirty())
            .enumerate()
        {
            entry.status = ObjectStatus::Committed;
            match &entry.state_object {
                Some(state_object) => {
                    trie.insert(&addresses_hash[i][..], &rlp::encode(&state_object.account()))?;
                }
                None => {
                    trie.remove(&addresses_hash[i][..])?;
                }
            }
        }
        self.root = From::from(&trie.root()?[..]);
        self.db.flush().or_else(|e| Err(Error::DB(format!("{}", e))))
    }

    /// Create a recoverable checkpoint of this state. Return the checkpoint index.
    pub fn checkpoint(&mut self) -> usize {
        debug!("state.checkpoint");
        let mut checkpoints = self.checkpoints.borrow_mut();
        let index = checkpoints.len();
        checkpoints.push(HashMap::new());
        index
    }

    fn add_checkpoint(&self, address: &Address) {
        if let Some(ref mut checkpoint) = self.checkpoints.borrow_mut().last_mut() {
            checkpoint
                .entry(*address)
                .or_insert_with(|| self.cache.borrow().get(address).map(StateObjectEntry::clone_dirty));
        }
    }

    /// Merge last checkpoint with previous.
    pub fn discard_checkpoint(&mut self) {
        debug!("state.discard_checkpoint");
        let last = self.checkpoints.borrow_mut().pop();
        if let Some(mut checkpoint) = last {
            if let Some(prev) = self.checkpoints.borrow_mut().last_mut() {
                if prev.is_empty() {
                    *prev = checkpoint;
                } else {
                    for (k, v) in checkpoint.drain() {
                        prev.entry(k).or_insert(v);
                    }
                }
            }
        }
    }

    /// Revert to the last checkpoint and discard it.
    pub fn revert_checkpoint(&mut self) {
        debug!("state.revert_checkpoint");
        if let Some(mut last) = self.checkpoints.borrow_mut().pop() {
            for (k, v) in last.drain() {
                match v {
                    Some(v) => match self.cache.borrow_mut().entry(k) {
                        Entry::Occupied(mut e) => {
                            // Merge checkpointed changes back into the main account
                            // storage preserving the cache.
                            e.get_mut().merge(v);
                        }
                        Entry::Vacant(e) => {
                            e.insert(v);
                        }
                    },
                    None => {
                        if let Entry::Occupied(e) = self.cache.borrow_mut().entry(k) {
                            if e.get().is_dirty() {
                                e.remove();
                            }
                        }
                    }
                }
            }
        }
    }
}

pub trait StateObjectInfo {
    fn nonce(&mut self, a: &Address) -> Result<U256, Error>;

    fn balance(&mut self, a: &Address) -> Result<U256, Error>;

    fn get_storage(&mut self, a: &Address, key: &H256) -> Result<H256, Error>;

    fn code(&mut self, a: &Address) -> Result<Vec<u8>, Error>;

    fn code_hash(&mut self, a: &Address) -> Result<H256, Error>;

    fn code_size(&mut self, a: &Address) -> Result<usize, Error>;
}

impl<B: DB> StateObjectInfo for State<B> {
    fn nonce(&mut self, address: &Address) -> Result<U256, Error> {
        self.call_with_cached(address, |a| Ok(a.map_or(U256::zero(), |e| e.nonce)))?
    }

    fn balance(&mut self, address: &Address) -> Result<U256, Error> {
        self.call_with_cached(address, |a| Ok(a.map_or(U256::zero(), |e| e.balance)))?
    }

    fn get_storage(&mut self, address: &Address, key: &H256) -> Result<H256, Error> {
        self.call_with_cached(address, |a| match a {
            Some(state_object) => {
                let accdb = Arc::new(AccountDB::new(*address, self.db.clone()));
                match state_object.get_storage(accdb, key)? {
                    Some(v) => Ok(v),
                    None => Ok(H256::zero()),
                }
            }
            None => Ok(H256::zero()),
        })?
    }

    fn code(&mut self, address: &Address) -> Result<Vec<u8>, Error> {
        self.call_with_cached(address, |a| Ok(a.map_or(vec![], |e| e.code.clone())))?
    }

    fn code_hash(&mut self, address: &Address) -> Result<H256, Error> {
        self.call_with_cached(address, |a| Ok(a.map_or(H256::zero(), |e| e.code_hash)))?
    }

    fn code_size(&mut self, address: &Address) -> Result<usize, Error> {
        self.call_with_cached(address, |a| Ok(a.map_or(0, |e| e.code_size)))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cita_trie::db::MemoryDB;
    use std::sync::Arc;

    fn get_temp_state() -> State<MemoryDB> {
        let db = Arc::new(MemoryDB::new(false));
        State::new(db).unwrap()
    }

    #[test]
    fn test_code_from_database() {
        let a = Address::zero();
        let (root, db) = {
            let mut state = get_temp_state();
            state.set_code(&a, vec![1, 2, 3]).unwrap();
            assert_eq!(state.code(&a).unwrap(), vec![1, 2, 3]);
            assert_eq!(
                state.code_hash(&a).unwrap(),
                "0xf1885eda54b7a053318cd41e2093220dab15d65381b1157a3633a83bfd5c9239".into()
            );
            assert_eq!(state.code_size(&a).unwrap(), 3);
            state.commit().unwrap();
            assert_eq!(state.code(&a).unwrap(), vec![1, 2, 3]);
            assert_eq!(
                state.code_hash(&a).unwrap(),
                "0xf1885eda54b7a053318cd41e2093220dab15d65381b1157a3633a83bfd5c9239".into()
            );
            assert_eq!(state.code_size(&a).unwrap(), 3);
            (state.root, state.db)
        };

        let mut state = State::from_existing(db, root).unwrap();
        assert_eq!(state.code(&a).unwrap(), vec![1, 2, 3]);
        assert_eq!(
            state.code_hash(&a).unwrap(),
            "0xf1885eda54b7a053318cd41e2093220dab15d65381b1157a3633a83bfd5c9239".into()
        );
        assert_eq!(state.code_size(&a).unwrap(), 3);
    }

    #[test]
    fn get_storage_from_datebase() {
        let a = Address::zero();
        let (root, db) = {
            let mut state = get_temp_state();
            state
                .set_storage(&a, H256::from(&U256::from(1u64)), H256::from(&U256::from(69u64)))
                .unwrap();
            state.commit().unwrap();
            (state.root, state.db)
        };

        let mut state = State::from_existing(db, root).unwrap();
        assert_eq!(
            state.get_storage(&a, &H256::from(&U256::from(1u64))).unwrap(),
            H256::from(&U256::from(69u64))
        );
    }

    #[test]
    fn get_from_database() {
        let a = Address::zero();
        let (root, db) = {
            let mut state = get_temp_state();
            state.inc_nonce(&a).unwrap();
            state.add_balance(&a, U256::from(69u64)).unwrap();
            state.commit().unwrap();
            assert_eq!(state.balance(&a).unwrap(), U256::from(69u64));
            assert_eq!(state.nonce(&a).unwrap(), U256::from(1u64));
            (state.root, state.db)
        };

        let mut state = State::from_existing(db, root).unwrap();
        assert_eq!(state.balance(&a).unwrap(), U256::from(69u64));
        assert_eq!(state.nonce(&a).unwrap(), U256::from(1u64));
    }

    #[test]
    fn remove() {
        let a = Address::zero();
        let mut state = get_temp_state();
        assert_eq!(state.exist(&a).unwrap(), false);
        state.inc_nonce(&a).unwrap();
        assert_eq!(state.exist(&a).unwrap(), true);
        assert_eq!(state.nonce(&a).unwrap(), U256::from(1u64));
        state.kill_contract(&a);
        assert_eq!(state.exist(&a).unwrap(), false);
        assert_eq!(state.nonce(&a).unwrap(), U256::from(0u64));
    }

    #[test]
    fn remove_from_database() {
        let a = Address::zero();
        let (root, db) = {
            let mut state = get_temp_state();
            state.add_balance(&a, U256::from(69u64)).unwrap();
            state.commit().unwrap();
            (state.root, state.db)
        };

        let (root, db) = {
            let mut state = State::from_existing(db, root).unwrap();
            assert_eq!(state.exist(&a).unwrap(), true);
            assert_eq!(state.balance(&a).unwrap(), U256::from(69u64));
            state.kill_contract(&a);
            state.commit().unwrap();
            assert_eq!(state.exist(&a).unwrap(), false);
            assert_eq!(state.balance(&a).unwrap(), U256::from(0u64));
            (state.root, state.db)
        };

        let mut state = State::from_existing(db, root).unwrap();
        assert_eq!(state.exist(&a).unwrap(), false);
        assert_eq!(state.balance(&a).unwrap(), U256::from(0u64));
    }

    #[test]
    fn alter_balance() {
        let mut state = get_temp_state();
        let a = Address::zero();
        let b: Address = 1u64.into();

        state.add_balance(&a, U256::from(69u64)).unwrap();
        assert_eq!(state.balance(&a).unwrap(), U256::from(69u64));
        state.commit().unwrap();
        assert_eq!(state.balance(&a).unwrap(), U256::from(69u64));

        state.sub_balance(&a, U256::from(42u64)).unwrap();
        assert_eq!(state.balance(&a).unwrap(), U256::from(27u64));
        state.commit().unwrap();
        assert_eq!(state.balance(&a).unwrap(), U256::from(27u64));

        state.transfer_balance(&a, &b, U256::from(18)).unwrap();
        assert_eq!(state.balance(&a).unwrap(), U256::from(9u64));
        assert_eq!(state.balance(&b).unwrap(), U256::from(18u64));
        state.commit().unwrap();
        assert_eq!(state.balance(&a).unwrap(), U256::from(9u64));
        assert_eq!(state.balance(&b).unwrap(), U256::from(18u64));
    }

    #[test]
    fn alter_nonce() {
        let mut state = get_temp_state();
        let a = Address::zero();
        state.inc_nonce(&a).unwrap();
        assert_eq!(state.nonce(&a).unwrap(), U256::from(1u64));
        state.inc_nonce(&a).unwrap();
        assert_eq!(state.nonce(&a).unwrap(), U256::from(2u64));
        state.commit().unwrap();
        assert_eq!(state.nonce(&a).unwrap(), U256::from(2u64));
        state.inc_nonce(&a).unwrap();
        assert_eq!(state.nonce(&a).unwrap(), U256::from(3u64));
        state.commit().unwrap();
        assert_eq!(state.nonce(&a).unwrap(), U256::from(3u64));
    }

    #[test]
    fn balance_nonce() {
        let mut state = get_temp_state();
        let a = Address::zero();
        assert_eq!(state.balance(&a).unwrap(), U256::from(0u64));
        assert_eq!(state.nonce(&a).unwrap(), U256::from(0u64));
        state.commit().unwrap();
        assert_eq!(state.balance(&a).unwrap(), U256::from(0u64));
        assert_eq!(state.nonce(&a).unwrap(), U256::from(0u64));
    }

    #[test]
    fn ensure_cached() {
        let mut state = get_temp_state();
        let a = Address::zero();
        state.new_contract(&a, U256::from(0u64), U256::from(0u64), vec![]);
        state.commit().unwrap();
        assert_eq!(
            state.root,
            "0ce23f3c809de377b008a4a3ee94a0834aac8bec1f86e28ffe4fdb5a15b0c785".into()
        );
    }

    #[test]
    fn checkpoint_basic() {
        let mut state = get_temp_state();
        let a = Address::zero();

        state.checkpoint();
        state.add_balance(&a, U256::from(69u64)).unwrap();
        assert_eq!(state.balance(&a).unwrap(), U256::from(69u64));
        state.discard_checkpoint();
        assert_eq!(state.balance(&a).unwrap(), U256::from(69u64));

        state.checkpoint();
        state.add_balance(&a, U256::from(1u64)).unwrap();
        assert_eq!(state.balance(&a).unwrap(), U256::from(70u64));
        state.revert_checkpoint();
        assert_eq!(state.balance(&a).unwrap(), U256::from(69u64));
    }

    #[test]
    fn checkpoint_nested() {
        let mut state = get_temp_state();
        let a = Address::zero();
        state.checkpoint();
        state.checkpoint();
        state.add_balance(&a, U256::from(69u64)).unwrap();
        assert_eq!(state.balance(&a).unwrap(), U256::from(69u64));
        state.discard_checkpoint();
        assert_eq!(state.balance(&a).unwrap(), U256::from(69u64));
        state.revert_checkpoint();
        assert_eq!(state.balance(&a).unwrap(), U256::from(0));
    }

    #[test]
    fn checkpoint_revert_to_get_storage() {
        let mut state = get_temp_state();
        let a = Address::zero();
        let k = H256::from(U256::from(0));

        state.checkpoint();
        state.checkpoint();
        state.set_storage(&a, k, H256::from(1u64)).unwrap();
        assert_eq!(state.get_storage(&a, &k).unwrap(), H256::from(1u64));
        state.revert_checkpoint();
        assert!(state.get_storage(&a, &k).unwrap().is_zero());
    }

    #[test]
    fn checkpoint_kill_account() {
        let mut state = get_temp_state();
        let a = Address::zero();
        let k = H256::from(U256::from(0));
        state.checkpoint();
        state.set_storage(&a, k, H256::from(U256::from(1))).unwrap();
        state.checkpoint();
        state.kill_contract(&a);
        assert!(state.get_storage(&a, &k).unwrap().is_zero());
        state.revert_checkpoint();
        assert_eq!(state.get_storage(&a, &k).unwrap(), H256::from(U256::from(1)));
    }

    #[test]
    fn checkpoint_create_contract_fail() {
        let mut state = get_temp_state();
        let orig_root = state.root;
        let a: Address = 1000.into();

        state.checkpoint(); // c1
        state.new_contract(&a, U256::zero(), U256::zero(), vec![]);
        state.add_balance(&a, U256::from(1)).unwrap();
        state.checkpoint(); // c2
        state.add_balance(&a, U256::from(1)).unwrap();
        state.discard_checkpoint(); // discard c2
        state.revert_checkpoint(); // revert to c1
        assert_eq!(state.exist(&a).unwrap(), false);
        state.commit().unwrap();
        assert_eq!(orig_root, state.root);
    }

    #[test]
    fn create_contract_fail_previous_storage() {
        let mut state = get_temp_state();
        let a: Address = 1000.into();
        let k = H256::from(U256::from(0));

        state.set_storage(&a, k, H256::from(U256::from(0xffff))).unwrap();
        state.commit().unwrap();
        state.clear();

        let orig_root = state.root;
        assert_eq!(state.get_storage(&a, &k).unwrap(), H256::from(U256::from(0xffff)));
        state.clear();

        state.checkpoint(); // c1
        state.new_contract(&a, U256::zero(), U256::zero(), vec![]);
        state.checkpoint(); // c2
        state.set_storage(&a, k, H256::from(U256::from(2))).unwrap();
        state.revert_checkpoint(); // revert to c2
        assert_eq!(state.get_storage(&a, &k).unwrap(), H256::from(U256::from(0)));
        state.revert_checkpoint(); // revert to c1
        assert_eq!(state.get_storage(&a, &k).unwrap(), H256::from(U256::from(0xffff)));

        state.commit().unwrap();
        assert_eq!(orig_root, state.root);
    }

    #[test]
    fn checkpoint_chores() {
        let mut state = get_temp_state();
        let a: Address = 1000.into();
        let b: Address = 2000.into();
        state.new_contract(&a, 5.into(), 0.into(), vec![10u8, 20, 30, 40, 50]);
        state.add_balance(&a, 5.into()).unwrap();
        state.set_storage(&a, 10.into(), 10.into()).unwrap();
        assert_eq!(state.code(&a).unwrap(), vec![10u8, 20, 30, 40, 50]);
        assert_eq!(state.balance(&a).unwrap(), 10.into());
        assert_eq!(state.get_storage(&a, &10.into()).unwrap(), 10.into());
        state.commit().unwrap();
        let orig_root = state.root;

        // Top         => account_a: balance=8, nonce=0, code=[10, 20, 30, 40, 50],
        //             |      stroage = { 10=15, 20=20 }
        //             |  account_b: balance=30, nonce=0, code=[]
        //             |      storage = { 55=55 }
        //
        //
        // Checkpoint2 => account_a: balance=8, nonce=0, code=[10, 20, 30, 40, 50],
        //             |      stroage = { 10=10, 20=20 }
        //             |  account_b: None
        //
        // Checkpoint1 => account_a: balance=10, nonce=0, code=[10, 20, 30, 40, 50],
        //             |      storage = { 10=10 }
        //             |  account_b: None

        state.checkpoint(); // c1
        state.sub_balance(&a, 2.into()).unwrap();
        state.set_storage(&a, 20.into(), 20.into()).unwrap();
        assert_eq!(state.balance(&a).unwrap(), 8.into());
        assert_eq!(state.get_storage(&a, &10.into()).unwrap(), 10.into());
        assert_eq!(state.get_storage(&a, &20.into()).unwrap(), 20.into());

        state.checkpoint(); // c2
        state.new_contract(&b, 30.into(), 0.into(), vec![]);
        state.set_storage(&a, 10.into(), 15.into()).unwrap();
        assert_eq!(state.balance(&b).unwrap(), 30.into());
        assert_eq!(state.code(&b).unwrap(), vec![]);

        state.revert_checkpoint(); // revert c2
        assert_eq!(state.balance(&a).unwrap(), 8.into());
        assert_eq!(state.get_storage(&a, &10.into()).unwrap(), 10.into());
        assert_eq!(state.get_storage(&a, &20.into()).unwrap(), 20.into());
        assert_eq!(state.balance(&b).unwrap(), 0.into());
        assert_eq!(state.code(&b).unwrap(), vec![]);
        assert_eq!(state.exist(&b).unwrap(), false);

        state.revert_checkpoint(); // revert c1
        assert_eq!(state.code(&a).unwrap(), vec![10u8, 20, 30, 40, 50]);
        assert_eq!(state.balance(&a).unwrap(), 10.into());
        assert_eq!(state.get_storage(&a, &10.into()).unwrap(), 10.into());

        state.commit().unwrap();
        assert_eq!(orig_root, state.root);
    }

    #[test]
    fn get_account_proof() {
        let mut state = get_temp_state();
        let a: Address = 1000.into();
        let b: Address = 2000.into();
        state.new_contract(&a, 5.into(), 0.into(), vec![10u8, 20, 30, 40, 50]);
        state.commit().unwrap();

        // The state only contains one account, should be a single leaf node, therefore the proof
        // length is 1
        let proof1 = state.get_account_proof(&a).unwrap();
        assert_eq!(proof1.len(), 1);

        // account not in state should also have non-empty proof, the proof is the longest common
        // prefix node
        let proof2 = state.get_account_proof(&b).unwrap();
        assert_eq!(proof2.len(), 1);

        assert_eq!(proof1, proof2);
    }

    #[test]
    fn get_storage_proof() {
        let mut state = get_temp_state();
        let a: Address = 1000.into();
        let b: Address = 2000.into();
        let c: Address = 3000.into();
        state.new_contract(&a, 5.into(), 0.into(), vec![10u8, 20, 30, 40, 50]);
        state.set_storage(&a, 10.into(), 10.into()).unwrap();
        state.new_contract(&b, 5.into(), 0.into(), vec![10u8, 20, 30, 40, 50]);
        state.commit().unwrap();

        // account not exist
        let proof = state.get_storage_proof(&c, &10.into()).unwrap();
        assert_eq!(proof.len(), 0);

        // account who has empty storage trie
        let proof = state.get_storage_proof(&b, &10.into()).unwrap();
        assert_eq!(proof.len(), 0);

        // account and storage key exists
        let proof1 = state.get_storage_proof(&a, &10.into()).unwrap();
        assert_eq!(proof1.len(), 1);

        // account exists but storage key not exist
        let proof2 = state.get_storage_proof(&a, &20.into()).unwrap();
        assert_eq!(proof2.len(), 1);

        assert_eq!(proof1, proof2);
    }
}
