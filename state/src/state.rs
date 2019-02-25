use super::account::StateObject;
use super::errors::Error;
use super::object_entry::{ObjectStatus, StateObjectEntry};
use cita_trie::codec::RLPNodeCodec;
use cita_trie::db::DB;
use cita_trie::trie::PatriciaTrie;
use cita_trie::trie::Trie;
use ethereum_types::{Address, H256, U256};
use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};

pub struct State<B> {
    pub db: B,
    pub root: H256,
    pub cache: RefCell<HashMap<Address, StateObjectEntry>>,
    pub checkpoints: RefCell<Vec<HashMap<Address, Option<StateObjectEntry>>>>,
    pub refund: BTreeMap<Address, u64>,
}

impl<B: DB> State<B> {
    /// Creates empty state for test.
    pub fn new(mut db: B) -> State<B> {
        let mut trie = PatriciaTrie::new(&mut db, RLPNodeCodec::default());
        let root = trie.root().unwrap();

        State {
            db,
            root: From::from(&root[..]),
            cache: RefCell::new(HashMap::new()),
            checkpoints: RefCell::new(Vec::new()),
            refund: BTreeMap::new(),
        }
    }

    /// Creates new state with existing state root
    pub fn from_existing(db: B, root: H256) -> Result<State<B>, Error> {
        if !db.contains(&root.0[..]).or(Err(Error::InvalidStateRoot))? {
            return Err(Error::InvalidStateRoot);
        }
        Ok(State {
            db,
            root,
            cache: RefCell::new(HashMap::new()),
            checkpoints: RefCell::new(Vec::new()),
            refund: BTreeMap::new(),
        })
    }

    /// Create a contract account with code or not
    pub fn new_contract(
        &mut self,
        contract: &Address,
        balance: U256,
        nonce: U256,
        code: Option<Vec<u8>>,
    ) -> StateObject {
        let mut state_object = StateObject::new(balance, nonce);
        state_object.init_code(code.unwrap_or_default());

        self.insert_cache(
            contract,
            StateObjectEntry::new_dirty(Some(state_object.clone_dirty())),
        );
        state_object
    }

    pub fn kill_contract(&mut self, contract: &Address) {
        self.insert_cache(contract, StateObjectEntry::new_dirty(None));
    }

    pub fn drop(self) -> (H256, B) {
        (self.root, self.db)
    }

    pub fn exist(&mut self, a: &Address) -> bool {
        if let Ok(_state_object) = self.get_state_object(a) {
            return true;
        }
        false
    }

    /// Get state object
    /// Firstly, search from cache. If not, get from trie.
    pub fn get_state_object(&mut self, address: &Address) -> Result<StateObject, Error> {
        if let Some(state_object_entry) = self.cache.borrow().get(address) {
            if let Some(state_object) = &state_object_entry.state_object {
                return Ok((*state_object).clone_dirty());
            }
        }

        let trie = PatriciaTrie::from(&mut self.db, RLPNodeCodec::default(), &self.root.0)
            .or(Err(Error::TrieError))?;
        match trie.get(&address) {
            Ok(Some(rlp)) => {
                let state_object = StateObject::from_rlp(&rlp)?;
                self.insert_cache(
                    address,
                    StateObjectEntry::new_clean(Some(state_object.clone_clean())),
                );
                Ok(state_object)
            }
            Ok(None) => {
                // this state object is not exist in patriciaTrie, maybe you need to crate a new contract
                Err(Error::AccountNotExist)
            }
            Err(_) => Err(Error::TrieError),
        }
    }

    pub fn set_storage(&mut self, address: &Address, key: H256, value: H256) -> Result<(), Error> {
        if self.storage_at(address, &key) != Some(value) {
            let contain_key = self.cache.borrow().contains_key(address);
            if !contain_key {
                let trie = PatriciaTrie::from(&mut self.db, RLPNodeCodec::default(), &self.root.0)
                    .or(Err(Error::TrieError))?;
                match trie.get(&address) {
                    Ok(Some(rlp)) => {
                        let mut state_object = StateObject::from_rlp(&rlp)?;
                        state_object.set_storage(key, value);
                        self.insert_cache(address, StateObjectEntry::new_dirty(Some(state_object)));
                    }
                    Ok(None) => {
                        // this state object is not exist in patriciaTrie, maybe you need to crate a new contract
                        return Err(Error::AccountNotExist);
                    }
                    Err(_err) => {
                        return Err(Error::TrieError);
                    }
                }
            }
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

    pub fn insert_cache(&self, address: &Address, state_object_entry: StateObjectEntry) {
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

    pub fn commit(&mut self) -> Result<(), Error> {
        assert!(self.checkpoints.borrow().is_empty());

        // firstly, update account storage tree
        for (_address, entry) in self
            .cache
            .borrow_mut()
            .iter_mut()
            .filter(|&(_, ref a)| a.is_dirty())
        {
            if let Some(ref mut state_object) = entry.state_object {
                state_object
                    .commit_storage(&mut self.db)
                    .or(Err(Error::DBError))?;
                state_object
                    .commit_code(&mut self.db)
                    .or(Err(Error::DBError))?;
            }
        }

        // secondly, update the whold state tree
        let mut trie = PatriciaTrie::from(&mut self.db, RLPNodeCodec::default(), &self.root.0)
            .or(Err(Error::TrieError))?;

        for (address, entry) in self
            .cache
            .borrow_mut()
            .iter_mut()
            .filter(|&(_, ref a)| a.is_dirty())
        {
            entry.status = ObjectStatus::Committed;
            match entry.state_object {
                Some(ref mut state_object) => {
                    trie.insert(address, &rlp::encode(&state_object.account()))
                        .or(Err(Error::TrieReConstructFailed))?;
                }
                None => {
                    trie.remove(address).or(Err(Error::TrieError))?;
                }
            }
        }
        self.root = From::from(&trie.root().or(Err(Error::TrieError))?[..]);
        Ok(())
    }

    pub fn checkpoint(&mut self) -> usize {
        let mut checkpoints = self.checkpoints.borrow_mut();
        let index = checkpoints.len();
        checkpoints.push(HashMap::new());
        index
    }

    fn add_checkpoint(&self, address: &Address) {
        if let Some(ref mut checkpoint) = self.checkpoints.borrow_mut().last_mut() {
            checkpoint.entry(*address).or_insert_with(|| {
                self.cache
                    .borrow()
                    .get(address)
                    .map(StateObjectEntry::clone_dirty)
            });
        }
    }

    // If the transaction if executed successfully
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

    // If the transaction fails to execute
    pub fn revert_checkpoint(&mut self) {
        if let Some(mut last) = self.checkpoints.borrow_mut().pop() {
            for (k, v) in last.drain() {
                match v {
                    Some(v) => match self.cache.get_mut().entry(k) {
                        Entry::Occupied(mut e) => {
                            e.get_mut().merge(v);
                        }
                        Entry::Vacant(e) => {
                            e.insert(v);
                        }
                    },
                    None => {
                        if let Entry::Occupied(e) = self.cache.get_mut().entry(k) {
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
    fn nonce(&mut self, a: &Address) -> Option<U256>;

    fn balance(&mut self, a: &Address) -> Option<U256>;

    fn storage_at(&mut self, a: &Address, key: &H256) -> Option<H256>;

    fn code(&mut self, a: &Address) -> Option<Vec<u8>>;

    fn set_code(&mut self, a: &Address, code: Vec<u8>);

    fn code_hash(&mut self, a: &Address) -> Option<H256>;

    fn code_size(&mut self, a: &Address) -> Option<usize>;

    fn add_balance(&mut self, a: &Address, incr: U256);

    fn sub_balance(&mut self, a: &Address, decr: U256);

    fn transfer_balance(&mut self, from: &Address, to: &Address, by: U256);

    fn inc_nonce(&mut self, a: &Address);

    fn add_refund(&mut self, address: &Address, n: u64);

    fn sub_refund(&mut self, address: &Address, n: u64);
}

impl<B: DB> StateObjectInfo for State<B> {
    fn nonce(&mut self, a: &Address) -> Option<U256> {
        if let Ok(state_object) = self.get_state_object(a) {
            return Some(state_object.nonce());
        }
        Some(U256::from(0))
    }

    fn balance(&mut self, a: &Address) -> Option<U256> {
        if let Ok(state_object) = self.get_state_object(a) {
            return Some(state_object.balance());
        }
        Some(U256::from(0))
    }

    fn storage_at(&mut self, a: &Address, key: &H256) -> Option<H256> {
        match self.get_state_object(a) {
            Ok(mut state_object) => {
                if let Some(value) = state_object.get_storage_at_changes(key) {
                    return Some(value);
                }
                if let Ok(Some(value)) = state_object.get_storage_at_backend(&mut self.db, key) {
                    return Some(value);
                }
                return None;
            }
            Err(_) => {
                // This account never exist, create one.
                self.new_contract(a, U256::from(0u64), U256::from(0u64), None);
            }
        }
        None
    }

    fn code(&mut self, a: &Address) -> Option<Vec<u8>> {
        if let Ok(mut state_object) = self.get_state_object(a) {
            if let Ok(code) = state_object.read_code(&mut self.db) {
                return Some(code);
            }
        }
        None
    }

    fn set_code(&mut self, a: &Address, code: Vec<u8>) {
        match self.get_state_object(a) {
            Ok(mut state_object) => {
                self.add_checkpoint(&a);
                state_object.init_code(code.clone());
                self.insert_cache(a, StateObjectEntry::new_dirty(Some(state_object)))
            }
            Err(_) => {
                // This account never exist, create one.
                self.new_contract(a, U256::from(0), U256::from(0), Some(code));
            }
        }
    }

    fn code_hash(&mut self, a: &Address) -> Option<H256> {
        if let Ok(mut state_object) = self.get_state_object(a) {
            if let Ok(_code) = state_object.read_code(&mut self.db) {
                return Some(state_object.code_hash());
            }
        }
        None
    }

    fn code_size(&mut self, a: &Address) -> Option<usize> {
        if let Ok(mut state_object) = self.get_state_object(a) {
            if let Ok(_code) = state_object.read_code(&mut self.db) {
                return Some(state_object.code_size());
            }
        }
        None
    }

    fn add_balance(&mut self, a: &Address, incr: U256) {
        if let false = incr.is_zero() {
            match self.get_state_object(a) {
                Ok(mut state_object) => {
                    state_object.add_balance(incr);
                    self.insert_cache(a, StateObjectEntry::new_dirty(Some(state_object)));
                }
                Err(_) => {
                    // This account never exist, create one.
                    self.new_contract(a, incr, U256::from(0), None);
                }
            }
        }
    }

    fn sub_balance(&mut self, a: &Address, decr: U256) {
        if let false = decr.is_zero() {
            match self.get_state_object(a) {
                Ok(mut state_object) => {
                    state_object.sub_balance(decr);
                    self.insert_cache(a, StateObjectEntry::new_dirty(Some(state_object)));
                }
                Err(_err) => {
                    unimplemented!();
                }
            }
        }
    }

    fn transfer_balance(&mut self, from: &Address, to: &Address, by: U256) {
        self.sub_balance(from, by);
        self.add_balance(to, by);
    }

    fn add_refund(&mut self, address: &Address, n: u64) {
        match self.get_state_object(address) {
            Ok(mut state_object) => {
                state_object.add_balance(U256::from(n));
                self.insert_cache(address, StateObjectEntry::new_dirty(Some(state_object)))
            }
            Err(_) => {
                // This account never exist, create one.
                self.new_contract(address, U256::from(n), U256::from(0), None);
            }
        }

        self.refund
            .entry(*address)
            .and_modify(|v| *v += n)
            .or_insert(n);
    }

    fn sub_refund(&mut self, address: &Address, n: u64) {
        match self.get_state_object(address) {
            Ok(mut state_object) => {
                state_object.sub_balance(U256::from(n));
                self.insert_cache(address, StateObjectEntry::new_dirty(Some(state_object)))
            }
            Err(_) => {
                // This account never exist, create one.
                self.new_contract(address, U256::from(n), U256::from(0), None);
            }
        }

        self.refund
            .entry(*address)
            .and_modify(|v| *v -= n)
            .or_insert(n);
    }

    fn inc_nonce(&mut self, a: &Address) {
        match self.get_state_object(a) {
            Ok(mut state_object) => {
                state_object.inc_nonce();
                self.insert_cache(a, StateObjectEntry::new_dirty(Some(state_object)))
            }
            Err(_) => {
                // This account never exist, create one.
                self.new_contract(a, U256::from(0), U256::from(1), None);
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use cita_trie::db::MemoryDB;

    fn get_temp_state() -> State<MemoryDB> {
        let db = MemoryDB::new();
        State::new(db)
    }

    #[test]
    fn test_code_from_database() {
        let a = Address::zero();
        let (root, db) = {
            let mut state = get_temp_state();
            state.set_code(&a, vec![1, 2, 3]);
            assert_eq!(state.code(&a).unwrap(), vec![1, 2, 3]);
            assert_eq!(
                state.code_hash(&a).unwrap(),
                "0xfd1780a6fc9ee0dab26ceb4b3941ab03e66ccd970d1db91612c66df4515b0a0a".into()
            );
            assert_eq!(state.code_size(&a).unwrap(), 3);
            state.commit().unwrap();
            assert_eq!(state.code(&a).unwrap(), vec![1, 2, 3]);
            assert_eq!(
                state.code_hash(&a).unwrap(),
                "0xfd1780a6fc9ee0dab26ceb4b3941ab03e66ccd970d1db91612c66df4515b0a0a".into()
            );
            assert_eq!(state.code_size(&a).unwrap(), 3);
            state.drop()
        };

        let mut state = State::from_existing(db, root).unwrap();
        assert_eq!(state.code(&a).unwrap(), vec![1, 2, 3]);
        assert_eq!(
            state.code_hash(&a).unwrap(),
            "0xfd1780a6fc9ee0dab26ceb4b3941ab03e66ccd970d1db91612c66df4515b0a0a".into()
        );
        assert_eq!(state.code_size(&a).unwrap(), 3);
    }

    #[test]
    fn storage_at_from_datebase() {
        let a = Address::zero();
        let (root, db) = {
            let mut state = get_temp_state();
            state
                .set_storage(
                    &a,
                    H256::from(&U256::from(1u64)),
                    H256::from(&U256::from(69u64)),
                )
                .unwrap();
            state.commit().unwrap();
            state.drop()
        };

        let mut state = State::from_existing(db, root).unwrap();
        assert_eq!(
            state
                .storage_at(&a, &H256::from(&U256::from(1u64)))
                .unwrap(),
            H256::from(&U256::from(69u64))
        );
    }

    #[test]
    fn get_from_database() {
        let a = Address::zero();
        let (root, db) = {
            let mut state = get_temp_state();
            state.inc_nonce(&a);
            state.add_balance(&a, U256::from(69u64));
            state.commit().unwrap();
            assert_eq!(state.balance(&a).unwrap(), U256::from(69u64));
            assert_eq!(state.nonce(&a).unwrap(), U256::from(1u64));
            state.drop()
        };

        let mut state = State::from_existing(db, root).unwrap();
        assert_eq!(state.balance(&a).unwrap(), U256::from(69u64));
        assert_eq!(state.nonce(&a).unwrap(), U256::from(1u64));
    }

    #[test]
    fn remove() {
        let a = Address::zero();
        let mut state = get_temp_state();
        assert_eq!(state.exist(&a), false);
        state.inc_nonce(&a);
        assert_eq!(state.exist(&a), true);
        assert_eq!(state.nonce(&a).unwrap(), U256::from(1u64));
        state.kill_contract(&a);
        assert_eq!(state.exist(&a), false);
        assert_eq!(state.nonce(&a).unwrap(), U256::from(0u64));
    }

    #[test]
    fn remove_from_database() {
        let a = Address::zero();
        let (root, db) = {
            let mut state = get_temp_state();
            state.add_balance(&a, U256::from(69u64));
            state.commit().unwrap();
            state.drop()
        };

        let (root, db) = {
            let mut state = State::from_existing(db, root).unwrap();
            assert_eq!(state.exist(&a), true);
            assert_eq!(state.balance(&a).unwrap(), U256::from(69u64));
            state.kill_contract(&a);
            state.commit().unwrap();
            assert_eq!(state.exist(&a), false);
            assert_eq!(state.balance(&a).unwrap(), U256::from(0u64));
            state.drop()
        };

        let mut state = State::from_existing(db, root).unwrap();
        assert_eq!(state.exist(&a), false);
        assert_eq!(state.balance(&a).unwrap(), U256::from(0u64));
    }

    #[test]
    fn alter_balance() {
        let mut state = get_temp_state();
        let a = Address::zero();
        let b: Address = 1u64.into();

        state.add_balance(&a, U256::from(69u64));
        assert_eq!(state.balance(&a).unwrap(), U256::from(69u64));
        state.commit().unwrap();
        assert_eq!(state.balance(&a).unwrap(), U256::from(69u64));

        state.sub_balance(&a, U256::from(42u64));
        assert_eq!(state.balance(&a).unwrap(), U256::from(27u64));
        state.commit().unwrap();
        assert_eq!(state.balance(&a).unwrap(), U256::from(27u64));

        state.transfer_balance(&a, &b, U256::from(18));
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
        state.inc_nonce(&a);
        assert_eq!(state.nonce(&a).unwrap(), U256::from(1u64));
        state.inc_nonce(&a);
        assert_eq!(state.nonce(&a).unwrap(), U256::from(2u64));
        state.commit().unwrap();
        assert_eq!(state.nonce(&a).unwrap(), U256::from(2u64));
        state.inc_nonce(&a);
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
        state.new_contract(&a, U256::from(0u64), U256::from(0u64), None);
        state.commit().unwrap();
        let (root, _db) = state.drop();
        assert_eq!(
            root,
            "3d019704df60561fb4ead78a6464021016353c761f2699851994e729ab95ef80".into()
        );
    }

    #[test]
    fn checkpoint_basic() {
        let mut state = get_temp_state();
        let a = Address::zero();

        state.checkpoint();
        state.add_balance(&a, U256::from(69u64));
        assert_eq!(state.balance(&a).unwrap(), U256::from(69u64));
        state.discard_checkpoint();
        assert_eq!(state.balance(&a).unwrap(), U256::from(69u64));

        state.checkpoint();
        state.add_balance(&a, U256::from(1u64));
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
        state.add_balance(&a, U256::from(69u64));
        assert_eq!(state.balance(&a).unwrap(), U256::from(69u64));
        state.discard_checkpoint();
        assert_eq!(state.balance(&a).unwrap(), U256::from(69u64));
        state.revert_checkpoint();
        assert_eq!(state.balance(&a).unwrap(), U256::from(0));
    }

    #[test]
    fn checkpoint_revert_to_storage_at() {
        let mut state = get_temp_state();
        let a = Address::zero();
        let k = H256::from(U256::from(0));

        state.checkpoint();
        state.checkpoint();
        state.set_storage(&a, k, H256::from(1u64)).unwrap();
        assert_eq!(state.storage_at(&a, &k).unwrap(), H256::from(1u64));
        state.revert_checkpoint();
        assert!(state.storage_at(&a, &k).is_none());
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
        assert!(state.storage_at(&a, &k).is_none());
        state.revert_checkpoint();
        assert_eq!(state.storage_at(&a, &k).unwrap(), H256::from(U256::from(1)));
    }

    #[test]
    fn checkpoint_create_contract_fail() {
        let state = get_temp_state();
        let orig_root = state.drop().0;
        let a: Address = 1000.into();

        let mut state = get_temp_state();
        state.checkpoint(); // c1
        state.new_contract(&a, U256::zero(), U256::zero(), None);
        state.add_balance(&a, U256::from(1));
        state.checkpoint(); // c2
        state.add_balance(&a, U256::from(1));
        state.discard_checkpoint(); // discard c2
        state.revert_checkpoint(); // revert to c1
        assert_eq!(state.exist(&a), false);
        state.commit().unwrap();
        assert_eq!(orig_root, state.drop().0);
    }
}
