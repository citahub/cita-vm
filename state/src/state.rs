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
use std::collections::HashMap;

pub struct State<B> {
    pub db: B,
    pub root: H256,
    pub cache: RefCell<HashMap<Address, StateObjectEntry>>,
    pub checkpoints: RefCell<Vec<HashMap<Address, Option<StateObjectEntry>>>>,
}

impl<B: DB> State<B> {
    /// Creates empty state for test.
    pub fn new(mut db: B) -> Result<State<B>, Error> {
        let mut trie = PatriciaTrie::new(&mut db, RLPNodeCodec::default());
        let root = trie.root()?;

        Ok(State {
            db,
            root: From::from(&root[..]),
            cache: RefCell::new(HashMap::new()),
            checkpoints: RefCell::new(Vec::new()),
        })
    }

    /// Creates new state with existing state root
    pub fn from_existing(db: B, root: H256) -> Result<State<B>, Error> {
        if !db
            .contains(&root.0[..])
            .or_else(|e| Err(Error::DB(format!("{}", e))))?
        {
            return Err(Error::KeyNotFound);
        }
        Ok(State {
            db,
            root,
            cache: RefCell::new(HashMap::new()),
            checkpoints: RefCell::new(Vec::new()),
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

    /// Get state object
    /// Firstly, search from cache. If not, get from trie.
    pub fn get_state_object(&mut self, address: &Address) -> Result<Option<StateObject>, Error> {
        if let Some(state_object_entry) = self.cache.borrow().get(address) {
            if let Some(state_object) = &state_object_entry.state_object {
                return Ok(Some((*state_object).clone_dirty()));
            }
        }
        let trie = PatriciaTrie::from(&mut self.db, RLPNodeCodec::default(), &self.root.0)?;
        match trie.get(&address)? {
            Some(rlp) => {
                let state_object = StateObject::from_rlp(&rlp)?;
                self.insert_cache(
                    address,
                    StateObjectEntry::new_clean(Some(state_object.clone_clean())),
                );
                Ok(Some(state_object))
            }
            None => Ok(None),
        }
    }

    pub fn exist(&mut self, a: &Address) -> Result<bool, Error> {
        Ok(self.get_state_object(a)?.is_some())
    }

    pub fn set_storage(&mut self, address: &Address, key: H256, value: H256) -> Result<(), Error> {
        if self.storage_at(address, &key)? != value {
            let contain_key = self.cache.borrow().contains_key(address);
            if !contain_key {
                let trie = PatriciaTrie::from(&mut self.db, RLPNodeCodec::default(), &self.root.0)?;
                match trie.get(&address)? {
                    Some(rlp) => {
                        let mut state_object = StateObject::from_rlp(&rlp)?;
                        state_object.set_storage(key, value);
                        self.insert_cache(address, StateObjectEntry::new_dirty(Some(state_object)));
                    }
                    None => {
                        return Err(Error::KeyNotFound);
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
                None => return Err(Error::NotInCache),
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
                state_object.commit_storage(&mut self.db)?;
                state_object.commit_code(&mut self.db)?;
            }
        }

        // secondly, update the whold state tree
        let mut trie = PatriciaTrie::from(&mut self.db, RLPNodeCodec::default(), &self.root.0)?;
        for (address, entry) in self
            .cache
            .borrow_mut()
            .iter_mut()
            .filter(|&(_, ref a)| a.is_dirty())
        {
            entry.status = ObjectStatus::Committed;
            match entry.state_object {
                Some(ref mut state_object) => {
                    trie.insert(address, &rlp::encode(&state_object.account()))?;
                }
                None => {
                    trie.remove(address)?;
                }
            }
        }
        self.root = From::from(&trie.root()?[..]);
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
    fn nonce(&mut self, a: &Address) -> Result<U256, Error>;

    fn balance(&mut self, a: &Address) -> Result<U256, Error>;

    fn storage_at(&mut self, a: &Address, key: &H256) -> Result<H256, Error>;

    fn code(&mut self, a: &Address) -> Result<Vec<u8>, Error>;

    fn set_code(&mut self, a: &Address, code: Vec<u8>) -> Result<(), Error>;

    fn code_hash(&mut self, a: &Address) -> Result<H256, Error>;

    fn code_size(&mut self, a: &Address) -> Result<usize, Error>;

    fn add_balance(&mut self, a: &Address, incr: U256) -> Result<(), Error>;

    fn sub_balance(&mut self, a: &Address, decr: U256) -> Result<(), Error>;

    fn transfer_balance(&mut self, from: &Address, to: &Address, by: U256) -> Result<(), Error>;

    fn inc_nonce(&mut self, a: &Address) -> Result<(), Error>;
}

impl<B: DB> StateObjectInfo for State<B> {
    fn nonce(&mut self, a: &Address) -> Result<U256, Error> {
        if let Some(state_object) = self.get_state_object(a)? {
            return Ok(state_object.nonce());
        }
        Ok(U256::from(0))
    }

    fn balance(&mut self, a: &Address) -> Result<U256, Error> {
        if let Some(state_object) = self.get_state_object(a)? {
            return Ok(state_object.balance());
        }
        Ok(U256::from(0))
    }

    fn storage_at(&mut self, a: &Address, key: &H256) -> Result<H256, Error> {
        match self.get_state_object(a)? {
            Some(mut state_object) => {
                if let Some(value) = state_object.get_storage_at_changes(key) {
                    return Ok(value);
                }
                if let Some(value) = state_object.get_storage_at_backend(&mut self.db, key)? {
                    return Ok(value);
                }
                Ok(H256::zero())
            }
            None => {
                // This account never exist, create one.
                self.new_contract(a, U256::from(0u64), U256::from(0u64), None);
                Ok(H256::zero())
            }
        }
    }

    fn code(&mut self, a: &Address) -> Result<Vec<u8>, Error> {
        if let Some(mut state_object) = self.get_state_object(a)? {
            return Ok(state_object.read_code(&mut self.db)?);
        }
        Ok(vec![])
    }

    fn set_code(&mut self, a: &Address, code: Vec<u8>) -> Result<(), Error> {
        match self.get_state_object(a)? {
            Some(mut state_object) => {
                self.add_checkpoint(&a);
                state_object.init_code(code.clone());
                self.insert_cache(a, StateObjectEntry::new_dirty(Some(state_object)));
                Ok(())
            }
            None => {
                self.new_contract(a, U256::from(0), U256::from(0), Some(code));
                Ok(())
            }
        }
    }

    fn code_hash(&mut self, a: &Address) -> Result<H256, Error> {
        if let Some(mut state_object) = self.get_state_object(a)? {
            let _ = state_object.read_code(&mut self.db)?;
            return Ok(state_object.code_hash());
        }
        Ok(H256::zero())
    }

    fn code_size(&mut self, a: &Address) -> Result<usize, Error> {
        if let Some(mut state_object) = self.get_state_object(a)? {
            let _ = state_object.read_code(&mut self.db)?;
            return Ok(state_object.code_size());
        }
        Ok(0)
    }

    fn add_balance(&mut self, a: &Address, incr: U256) -> Result<(), Error> {
        if incr.is_zero() {
            return Ok(());
        }
        match self.get_state_object(a)? {
            Some(mut state_object) => {
                state_object.add_balance(incr);
                self.insert_cache(a, StateObjectEntry::new_dirty(Some(state_object)));
            }
            None => {
                self.new_contract(a, incr, U256::from(0), None);
            }
        }
        Ok(())
    }

    fn sub_balance(&mut self, a: &Address, decr: U256) -> Result<(), Error> {
        if decr.is_zero() {
            return Ok(());
        }

        match self.get_state_object(a)? {
            Some(mut state_object) => {
                state_object.sub_balance(decr);
                self.insert_cache(a, StateObjectEntry::new_dirty(Some(state_object)));
            }
            None => return Err(Error::KeyNotFound),
        }
        Ok(())
    }

    fn transfer_balance(&mut self, from: &Address, to: &Address, by: U256) -> Result<(), Error> {
        self.sub_balance(from, by)?;
        self.add_balance(to, by)?;
        Ok(())
    }

    fn inc_nonce(&mut self, a: &Address) -> Result<(), Error> {
        match self.get_state_object(a)? {
            Some(mut state_object) => {
                state_object.inc_nonce();
                self.insert_cache(a, StateObjectEntry::new_dirty(Some(state_object)))
            }
            None => {
                self.new_contract(a, U256::from(0), U256::from(1), None);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use cita_trie::db::MemoryDB;

    fn get_temp_state() -> State<MemoryDB> {
        let db = MemoryDB::new();
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
            state.inc_nonce(&a).unwrap();
            state.add_balance(&a, U256::from(69u64)).unwrap();
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
            state.drop()
        };

        let (root, db) = {
            let mut state = State::from_existing(db, root).unwrap();
            assert_eq!(state.exist(&a).unwrap(), true);
            assert_eq!(state.balance(&a).unwrap(), U256::from(69u64));
            state.kill_contract(&a);
            state.commit().unwrap();
            assert_eq!(state.exist(&a).unwrap(), false);
            assert_eq!(state.balance(&a).unwrap(), U256::from(0u64));
            state.drop()
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
    fn checkpoint_revert_to_storage_at() {
        let mut state = get_temp_state();
        let a = Address::zero();
        let k = H256::from(U256::from(0));

        state.checkpoint();
        state.checkpoint();
        state.set_storage(&a, k, H256::from(1u64)).unwrap();
        assert_eq!(state.storage_at(&a, &k).unwrap(), H256::from(1u64));
        state.revert_checkpoint();
        assert!(state.storage_at(&a, &k).unwrap().is_zero());
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
        assert!(state.storage_at(&a, &k).unwrap().is_zero());
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
        state.add_balance(&a, U256::from(1)).unwrap();
        state.checkpoint(); // c2
        state.add_balance(&a, U256::from(1)).unwrap();
        state.discard_checkpoint(); // discard c2
        state.revert_checkpoint(); // revert to c1
        assert_eq!(state.exist(&a).unwrap(), false);
        state.commit().unwrap();
        assert_eq!(orig_root, state.drop().0);
    }
}
