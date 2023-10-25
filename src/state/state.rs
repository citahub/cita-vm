use crate::common;
use crate::common::hash;
use crate::state::account::StateObject;
use crate::state::account_db::AccountDB;
use crate::state::err::Error;
use crate::state::object_entry::{ObjectStatus, StateObjectEntry};
use cita_trie::{PatriciaTrie, Trie, CDB};
use ethereum_types::{Address, H256, U256};
use hashbrown::hash_map::Entry;
use hashbrown::{HashMap, HashSet};
use log::debug;
use rayon::prelude::{IntoParallelRefMutIterator, ParallelIterator};
use std::cell::RefCell;
use std::sync::Arc;

const PREFIX_LEN: usize = 12;
const LATEST_ERA_KEY: [u8; PREFIX_LEN] = [b'l', b'a', b's', b't', 0, 0, 0, 0, 0, 0, 0, 0];

/// State is the one who managers all accounts and states in Ethereum's system.
pub struct State<B> {
    pub db: Arc<B>,
    pub root: H256,
    pub cache: RefCell<HashMap<Address, StateObjectEntry>>,
    /// Checkpoints are used to revert to history
    pub checkpoints: RefCell<Vec<HashMap<Address, Option<StateObjectEntry>>>>,
}

impl<B: CDB> State<B> {
    /// Creates empty state for test.
    pub fn new(db: Arc<B>) -> Result<State<B>, Error> {
        let mut trie = PatriciaTrie::new(Arc::clone(&db), Arc::new(hash::get_hasher()));
        //let mut trie = PatriciaTrie::new(Arc::clone(&db), Arc::new(hash::HasherNull::new()));
        let root = trie.root()?;
        let mut date = [0; 32];
        date.copy_from_slice(root.as_slice());

        Ok(State {
            db,
            root: H256::from(date),
            cache: RefCell::new(HashMap::new()),
            checkpoints: RefCell::new(Vec::new()),
        })
    }

    /// Creates new state with existing state root
    pub fn from_existing(db: Arc<B>, root: H256) -> Result<State<B>, Error> {
        if !db.contains(&root.0[..]).map_err(|e| Error::DB(format!("{}", e)))? {
            return Err(Error::NotFound);
        }
        // This for compatible with cita 0.x,no need to update it
        // For state test,this should be removed
        if root == common::hash::RLP_NULL {
            db.insert(LATEST_ERA_KEY.to_vec(), [0x80].to_vec()).unwrap();
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
        let mut state_object = StateObject::new(balance, nonce);
        state_object.init_code(code);

        self.insert_cache(contract, StateObjectEntry::new_dirty(Some(state_object.clone_dirty())));
        state_object
    }

    /// Kill a contract.
    pub fn kill_contract(&mut self, contract: &Address) {
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
        let trie = PatriciaTrie::from(Arc::clone(&self.db), Arc::new(hash::get_hasher()), &self.root.0)?;
        match trie.get(&address[..])? {
            Some(rlp) => {
                let mut state_object = StateObject::from_rlp(&rlp)?;
                let accdb = Arc::new(AccountDB::new(*address, self.db.clone()));
                state_object.read_code(accdb.clone())?;
                state_object.read_abi(accdb)?;
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
        let trie = PatriciaTrie::from(Arc::clone(&self.db), Arc::new(hash::get_hasher()), &self.root.0)?;

        //match trie.get(common::hash::summary(&address[..]).as_slice())? {
        match trie.get(&address[..])? {
            Some(rlp) => {
                let mut state_object = StateObject::from_rlp(&rlp)?;
                state_object.read_code(self.db.clone())?;
                state_object.read_abi(self.db.clone())?;
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
        let trie = PatriciaTrie::from(Arc::clone(&self.db), Arc::new(hash::get_hasher()), &self.root.0)?;
        //let trie = PatriciaTrie::from(Arc::clone(&self.db), Arc::new(hash::HasherNull::new()), &self.root.0)?;
        let proof = trie.get_proof(common::hash::summary(&address[..]).as_slice())?;
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
        let mut state_object = self.get_state_object_or_default(address)?;
        state_object.init_code(code);
        self.insert_cache(address, StateObjectEntry::new_dirty(Some(state_object)));
        Ok(())
    }

    /// Set abi for an account.
    pub fn set_abi(&mut self, address: &Address, abi: Vec<u8>) -> Result<(), Error> {
        let mut state_object = self.get_state_object_or_default(address)?;
        state_object.init_abi(abi);
        self.insert_cache(address, StateObjectEntry::new_dirty(Some(state_object)));
        Ok(())
    }

    /// Add balance by incr for an account.
    pub fn add_balance(&mut self, address: &Address, incr: U256) -> Result<(), Error> {
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
        self.cache
            .borrow_mut()
            .par_iter_mut()
            .map(|(address, entry)| {
                if !entry.is_dirty() {
                    return Ok(());
                }

                if let Some(ref mut state_object) = entry.state_object {
                    // When operate on account element, AccountDB should be used
                    let accdb = Arc::new(AccountDB::new(*address, self.db.clone()));
                    state_object.commit_storage(Arc::clone(&accdb))?;
                    state_object.commit_code(Arc::clone(&accdb))?;
                    state_object.commit_abi(Arc::clone(&accdb))?;
                }
                Ok(())
            })
            .collect::<Result<(), Error>>()?;

        // Secondly, update the world state tree
        let mut trie = PatriciaTrie::from(Arc::clone(&self.db), Arc::new(hash::get_hasher()), &self.root.0)?;

        let key_values = self
            .cache
            .borrow_mut()
            .par_iter_mut()
            .filter(|(_, a)| a.is_dirty())
            .map(|(address, entry)| {
                entry.status = ObjectStatus::Committed;
                match entry.state_object {
                    Some(ref mut state_object) => (address.0.to_vec(), rlp::encode(&state_object.account()).to_vec()),
                    None => (address.0.to_vec(), vec![]),
                }
            })
            .collect::<Vec<(Vec<u8>, Vec<u8>)>>();

        for (key, value) in key_values.into_iter() {
            trie.insert(key, value)?;
        }

        let mut date = [0; 32];
        date.copy_from_slice(trie.root()?.as_slice());
        self.root = H256::from(date);
        self.db.flush().map_err(|e| Error::DB(format!("{}", e)))
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

    fn abi(&mut self, a: &Address) -> Result<Vec<u8>, Error>;

    fn abi_hash(&mut self, a: &Address) -> Result<H256, Error>;

    fn abi_size(&mut self, a: &Address) -> Result<usize, Error>;
}

impl<B: CDB> StateObjectInfo for State<B> {
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

    fn abi(&mut self, address: &Address) -> Result<Vec<u8>, Error> {
        self.call_with_cached(address, |a| Ok(a.map_or(vec![], |e| e.abi.clone())))?
    }

    fn abi_hash(&mut self, address: &Address) -> Result<H256, Error> {
        self.call_with_cached(address, |a| Ok(a.map_or(H256::zero(), |e| e.abi_hash)))?
    }

    fn abi_size(&mut self, address: &Address) -> Result<usize, Error> {
        self.call_with_cached(address, |a| Ok(a.map_or(0, |e| e.abi_size)))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cita_trie::MemoryDB;
    use std::str::FromStr;
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
                H256::from_str("0xf1885eda54b7a053318cd41e2093220dab15d65381b1157a3633a83bfd5c9239").unwrap()
            );
            assert_eq!(state.code_size(&a).unwrap(), 3);
            state.commit().unwrap();
            assert_eq!(state.code(&a).unwrap(), vec![1, 2, 3]);
            assert_eq!(
                state.code_hash(&a).unwrap(),
                H256::from_str("0xf1885eda54b7a053318cd41e2093220dab15d65381b1157a3633a83bfd5c9239").unwrap()
            );
            assert_eq!(state.code_size(&a).unwrap(), 3);
            (state.root, state.db)
        };

        let mut state = State::from_existing(db, root).unwrap();
        assert_eq!(state.code(&a).unwrap(), vec![1, 2, 3]);
        assert_eq!(
            state.code_hash(&a).unwrap(),
            H256::from_str("0xf1885eda54b7a053318cd41e2093220dab15d65381b1157a3633a83bfd5c9239").unwrap()
        );
        assert_eq!(state.code_size(&a).unwrap(), 3);
    }

    #[test]
    fn test_abi_from_database() {
        let a = Address::zero();
        let (root, db) = {
            let mut state = get_temp_state();
            state.set_abi(&a, vec![1, 2, 3]).unwrap();
            assert_eq!(state.abi(&a).unwrap(), vec![1, 2, 3]);
            assert_eq!(
                state.abi_hash(&a).unwrap(),
                H256::from_str("0xf1885eda54b7a053318cd41e2093220dab15d65381b1157a3633a83bfd5c9239").unwrap()
            );
            assert_eq!(state.abi_size(&a).unwrap(), 3);
            state.commit().unwrap();
            assert_eq!(state.abi(&a).unwrap(), vec![1, 2, 3]);
            assert_eq!(
                state.abi_hash(&a).unwrap(),
                H256::from_str("0xf1885eda54b7a053318cd41e2093220dab15d65381b1157a3633a83bfd5c9239").unwrap()
            );
            assert_eq!(state.abi_size(&a).unwrap(), 3);
            (state.root, state.db)
        };

        let mut state = State::from_existing(db, root).unwrap();
        assert_eq!(state.abi(&a).unwrap(), vec![1, 2, 3]);
        assert_eq!(
            state.abi_hash(&a).unwrap(),
            H256::from_str("0xf1885eda54b7a053318cd41e2093220dab15d65381b1157a3633a83bfd5c9239").unwrap()
        );
        assert_eq!(state.abi_size(&a).unwrap(), 3);
    }

    #[test]
    fn get_storage_from_datebase() {
        let a = Address::zero();
        let (root, db) = {
            let mut state = get_temp_state();
            state
                .set_storage(&a, H256::from_low_u64_be(1), H256::from_low_u64_be(69))
                .unwrap();
            state.commit().unwrap();
            (state.root, state.db)
        };

        let mut state = State::from_existing(db, root).unwrap();
        assert_eq!(
            state.get_storage(&a, &H256::from_low_u64_be(1u64)).unwrap(),
            H256::from_low_u64_be(69u64)
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
        let b = Address::from_low_u64_be(1);

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
            H256::from_str("530acecc6ec873396bb3e90b6578161f9688ed7eeeb93d6fba5684895a93b78a").unwrap()
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
        let k = H256::zero();

        state.checkpoint();
        state.checkpoint();
        state.set_storage(&a, k, H256::from_low_u64_be(1)).unwrap();
        assert_eq!(state.get_storage(&a, &k).unwrap(), H256::from_low_u64_be(1));
        state.revert_checkpoint();
        assert!(state.get_storage(&a, &k).unwrap().is_zero());
    }

    #[test]
    fn checkpoint_kill_account() {
        let mut state = get_temp_state();
        let a = Address::zero();
        let k = H256::zero();
        state.checkpoint();
        state.set_storage(&a, k, H256::from_low_u64_be(1)).unwrap();
        state.checkpoint();
        state.kill_contract(&a);
        assert!(state.get_storage(&a, &k).unwrap().is_zero());
        state.revert_checkpoint();
        assert_eq!(state.get_storage(&a, &k).unwrap(), H256::from_low_u64_be(1));
    }

    #[test]
    fn checkpoint_create_contract_fail() {
        let mut state = get_temp_state();
        let orig_root = state.root;
        let a = Address::from_low_u64_be(1000);

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
        let a = Address::from_low_u64_be(1000);
        let k = H256::zero();

        state.set_storage(&a, k, H256::from_low_u64_be(0xffff)).unwrap();
        state.commit().unwrap();
        state.clear();

        let orig_root = state.root;
        assert_eq!(state.get_storage(&a, &k).unwrap(), H256::from_low_u64_be(0xffff));
        state.clear();

        state.checkpoint(); // c1
        state.new_contract(&a, U256::zero(), U256::zero(), vec![]);
        state.checkpoint(); // c2
        state.set_storage(&a, k, H256::from_low_u64_be(2)).unwrap();
        state.revert_checkpoint(); // revert to c2
        assert_eq!(state.get_storage(&a, &k).unwrap(), H256::from_low_u64_be(0));
        state.revert_checkpoint(); // revert to c1
        assert_eq!(state.get_storage(&a, &k).unwrap(), H256::from_low_u64_be(0xffff));

        state.commit().unwrap();
        assert_eq!(orig_root, state.root);
    }

    #[test]
    fn checkpoint_chores() {
        let mut state = get_temp_state();
        let a = Address::from_low_u64_be(1000);
        let b = Address::from_low_u64_be(2000);
        state.new_contract(&a, 5.into(), 0.into(), vec![10u8, 20, 30, 40, 50]);
        state.add_balance(&a, 5.into()).unwrap();
        state
            .set_storage(&a, H256::from_low_u64_be(10), H256::from_low_u64_be(10))
            .unwrap();
        assert_eq!(state.code(&a).unwrap(), vec![10u8, 20, 30, 40, 50]);
        assert_eq!(state.balance(&a).unwrap(), 10.into());
        assert_eq!(
            state.get_storage(&a, &H256::from_low_u64_be(10)).unwrap(),
            H256::from_low_u64_be(10)
        );
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
        state
            .set_storage(&a, H256::from_low_u64_be(20), H256::from_low_u64_be(20))
            .unwrap();
        assert_eq!(state.balance(&a).unwrap(), 8.into());
        assert_eq!(
            state.get_storage(&a, &H256::from_low_u64_be(10)).unwrap(),
            H256::from_low_u64_be(10)
        );
        assert_eq!(
            state.get_storage(&a, &H256::from_low_u64_be(20)).unwrap(),
            H256::from_low_u64_be(20)
        );

        state.checkpoint(); // c2
        state.new_contract(&b, 30.into(), 0.into(), vec![]);
        state
            .set_storage(&a, H256::from_low_u64_be(10), H256::from_low_u64_be(10))
            .unwrap();
        assert_eq!(state.balance(&b).unwrap(), 30.into());
        assert!(state.code(&b).unwrap().is_empty());

        state.revert_checkpoint(); // revert c2
        assert_eq!(state.balance(&a).unwrap(), 8.into());
        assert_eq!(
            state.get_storage(&a, &H256::from_low_u64_be(10)).unwrap(),
            H256::from_low_u64_be(10)
        );
        assert_eq!(
            state.get_storage(&a, &H256::from_low_u64_be(20)).unwrap(),
            H256::from_low_u64_be(20)
        );
        assert_eq!(state.balance(&b).unwrap(), 0.into());
        assert!(state.code(&b).unwrap().is_empty());
        assert_eq!(state.exist(&b).unwrap(), false);

        state.revert_checkpoint(); // revert c1
        assert_eq!(state.code(&a).unwrap(), vec![10u8, 20, 30, 40, 50]);
        assert_eq!(state.balance(&a).unwrap(), 10.into());
        assert_eq!(
            state.get_storage(&a, &H256::from_low_u64_be(10)).unwrap(),
            H256::from_low_u64_be(10)
        );

        state.commit().unwrap();
        assert_eq!(orig_root, state.root);
    }

    #[test]
    fn get_account_proof() {
        let mut state = get_temp_state();
        let a = Address::from_low_u64_be(1000);
        let b = Address::from_low_u64_be(2000);
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
        let a = Address::from_low_u64_be(1000);
        let b = Address::from_low_u64_be(2000);
        let c = Address::from_low_u64_be(3000);
        state.new_contract(&a, 5.into(), 0.into(), vec![10u8, 20, 30, 40, 50]);
        state
            .set_storage(&a, H256::from_low_u64_be(10), H256::from_low_u64_be(15))
            .unwrap();
        state.new_contract(&b, 5.into(), 0.into(), vec![10u8, 20, 30, 40, 50]);
        state.commit().unwrap();

        // account not exist
        let proof = state.get_storage_proof(&c, &H256::from_low_u64_be(10)).unwrap();
        assert_eq!(proof.len(), 0);

        // account who has empty storage trie
        let proof = state.get_storage_proof(&b, &H256::from_low_u64_be(10)).unwrap();
        assert_eq!(proof.len(), 0);

        // account and storage key exists
        let proof1 = state.get_storage_proof(&a, &H256::from_low_u64_be(10)).unwrap();
        assert_eq!(proof1.len(), 1);

        // account exists but storage key not exist
        let proof2 = state.get_storage_proof(&a, &H256::from_low_u64_be(20)).unwrap();
        assert_eq!(proof2.len(), 1);

        assert_eq!(proof1, proof2);
    }

    #[test]
    fn create_empty() {
        let mut state = get_temp_state();
        state.commit().unwrap();

        #[cfg(feature = "sha3hash")]
        let expected = "56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421";
        #[cfg(feature = "blake2bhash")]
        let expected = "c14af59107ef14003e4697a40ea912d865eb1463086a4649977c13ea69b0d9af";
        #[cfg(feature = "sm3hash")]
        let expected = "995b949869f80fa1465a9d8b6fa759ec65c3020d59c2624662bdff059bdf19b3";

        assert_eq!(state.root, H256::from_str(expected).unwrap());
    }
}
