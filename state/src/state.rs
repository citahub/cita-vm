use super::account::StateObject;
use super::state_object_entry::{ObjectStatus, StateObjectEntry};
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
    pub fn new(mut db: B) -> State<B> {
        let mut trie = PatriciaTrie::new(&mut db, RLPNodeCodec::default());
        let root = trie.root().unwrap();

        State {
            db,
            root: H256::from_slice(&root),
            cache: RefCell::new(HashMap::new()),
            checkpoints: RefCell::new(Vec::new()),
            refund: BTreeMap::new(),
        }
    }

    pub fn from_existing(db: B, root: H256) -> State<B> {
        State {
            db: db,
            root: H256::from_slice(&root),
            cache: RefCell::new(HashMap::new()),
            checkpoints: RefCell::new(Vec::new()),
            refund: BTreeMap::new(),
        }
    }

    pub fn new_contract(&mut self, contract: &Address, balance: U256, nonce: U256) {
        let original_storage_root = H256::default(); // fix me
        self.insert_cache(
            contract,
            StateObjectEntry::new_dirty_state_object(Some(StateObject::new_state_object(
                balance,
                nonce,
                original_storage_root,
            ))),
        );
    }

    pub fn kill_contract(&mut self, contract: &Address) {
        self.insert_cache(contract, StateObjectEntry::new_dirty_state_object(None));
    }

    pub fn is_empty(&mut self, address: &Address) -> bool {
        if let Some(state_object_entry) = self.cache.borrow().get(address) {
            if let Some(ref _state_object) = state_object_entry.state_object {
                return true;
            }
        }

        // let trie = PatriciaTrie::from(&mut self.db, RLPNodeCodec::default(), &self.root.0).unwrap();

        if let Some(ref _state_object) = self.db.get(&address).unwrap() {
            return true;
        }
        false
    }

    pub fn db(self) -> B {
        self.db
    }

    pub fn root(&self) -> &H256 {
        &self.root
    }

    pub fn add_refund(&mut self, address: &Address, n: u64) {
        match self.ensure_cached(address) {
            Some(mut state_object) => {
                state_object.add_balance(&U256::from(n));
                self.insert_cache(
                    address,
                    StateObjectEntry::new_dirty_state_object(Some(state_object)),
                )
            }
            None => {
                self.new_contract(address, U256::from(n), U256::from(0));
            }
        }

        self.refund
            .entry(*address)
            .and_modify(|v| *v += n)
            .or_insert(n);
    }

    pub fn sub_refund(&mut self, address: &Address, n: u64) {
        match self.ensure_cached(address) {
            Some(mut state_object) => {
                state_object.sub_balance(&U256::from(n));
                self.insert_cache(
                    address,
                    StateObjectEntry::new_dirty_state_object(Some(state_object)),
                )
            }
            None => {
                self.new_contract(address, U256::from(n), U256::from(0));
            }
        }

        self.refund
            .entry(*address)
            .and_modify(|v| *v -= n)
            .or_insert(n);
    }

    pub fn ensure_cached(&mut self, address: &Address) -> Option<StateObject> {
        if let Some(state_object_entry) = self.cache.borrow().get(address) {
            if let Some(state_object) = &state_object_entry.state_object {
                return Some((*state_object).clone_all());
            }
        }

        // let trie = PatriciaTrie::from(&mut self.db, RLPNodeCodec::default(), &self.root.0).unwrap();
        match self.db.get(&address) {
            Ok(Some(rlp)) => {
                let state_object = StateObject::from_rlp(&rlp);
                self.insert_cache(
                    address,
                    StateObjectEntry::new_clean_state_object(Some(
                        state_object.clone_basic_state_object(),
                    )),
                );
                return Some(state_object);
            }
            Ok(None) => {
                // TODO
            }
            Err(_) => {
                // TODO
            }
        }
        None
    }

    pub fn storage_at(&mut self, address: &Address, key: &H256) -> H256 {
        if let Some(mut state_object) = self.ensure_cached(address) {
            if let Some(value) = state_object.cached_storage_at(key) {
                return value;
            }
            if let Some(value) = state_object.trie_storage_at(&mut self.db, key) {
                return value;
            }
        }
        H256::from(0)
    }

    pub fn set_storage(&mut self, address: &Address, key: H256, value: H256) {
        if self.storage_at(address, &key) != value {
            let contain_key = self.cache.borrow().contains_key(address);
            if !contain_key {
                // let trie = PatriciaTrie::from(&mut self.db, RLPNodeCodec::default(), &self.root.0)
                //     .unwrap();
                match self.db.get(&address) {
                    Ok(rlp) => {
                        let mut state_object = StateObject::from_rlp(&rlp.unwrap());
                        state_object.set_storage(key, value);
                        self.insert_cache(
                            address,
                            StateObjectEntry::new_dirty_state_object(Some(state_object)),
                        );
                    }
                    Err(_) => panic!("this state object  is not exist in patriciaTrie."),
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
    }

    pub fn insert_cache(&self, address: &Address, state_object_entry: StateObjectEntry) {
        let is_dirty = state_object_entry.is_dirty();
        self.cache.borrow_mut().insert(
            *address,
            state_object_entry.clone_dirty_state_object_entry(),
        );

        if is_dirty {
            if let Some(checkpoint) = self.checkpoints.borrow_mut().last_mut() {
                checkpoint
                    .entry(*address)
                    .or_insert(Some(state_object_entry));
            }
        }
    }

    pub fn commit(&mut self) {
        assert!(self.checkpoints.borrow().is_empty());
        let mut trie =
            PatriciaTrie::from(&mut self.db, RLPNodeCodec::default(), &self.root.0).unwrap();

        for (address, a) in self
            .cache
            .borrow_mut()
            .iter_mut()
            .filter(|&(_, ref a)| a.is_dirty())
        {
            a.status = ObjectStatus::Committed;
            match a.state_object {
                Some(ref mut state_object) => {
                    trie.insert(address, &state_object.rlp());
                }
                None => {
                    trie.remove(address);
                }
            }
        }
    }

    pub fn checkpoint(&mut self) {
        self.checkpoints.borrow_mut().push(HashMap::new());
    }

    fn add_checkpoint(&self, address: &Address) {
        if let Some(ref mut checkpoint) = self.checkpoints.borrow_mut().last_mut() {
            checkpoint.entry(*address).or_insert_with(|| {
                self.cache
                    .borrow()
                    .get(address)
                    .map(StateObjectEntry::clone_dirty_state_object_entry)
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
                            e.get_mut().overwrite_with_state_object_entry(v);
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
