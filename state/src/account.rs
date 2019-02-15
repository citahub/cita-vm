use cita_trie::db::DB;
use ethereum_types::{H256, U256};
use keccak_hash::{keccak, KECCAK_EMPTY, KECCAK_NULL_RLP};
use lru_cache::LruCache;
use rlp::*;
use std::cell::RefCell;
use std::collections::HashMap;

const STORAGE_CACHE_ITEMS: usize = 8192;
type Bytes = Vec<u8>;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum CodeState {
    Clean,
    Dirty,
}

#[derive(Debug)]
pub struct StateObject {
    balance: U256,
    nonce: U256,
    storage_root: H256,
    code: Bytes,
    code_hash: H256,
    code_size: Option<usize>,
    code_state: CodeState,
    storage_changes: HashMap<H256, H256>,
    storage_cache: RefCell<LruCache<H256, H256>>,
    original_storage_cache: Option<(H256, RefCell<LruCache<H256, H256>>)>,
}

#[derive(Debug)]
pub struct Account {
    balance: U256,
    nonce: U256,
    storage_root: H256,
    code_hash: H256,
}

impl rlp::Encodable for Account {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(4)
            .append(&self.nonce)
            .append(&self.balance)
            .append(&self.storage_root)
            .append(&self.code_hash);
    }
}

impl rlp::Decodable for Account {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        Ok(Account {
            nonce: rlp.val_at(0)?,
            balance: rlp.val_at(1)?,
            storage_root: rlp.val_at(2)?,
            code_hash: rlp.val_at(3)?,
        })
    }
}

impl From<Account> for StateObject {
    fn from(account: Account) -> Self {
        StateObject {
            balance: account.balance,
            nonce: account.nonce,
            storage_root: account.storage_root,
            code: vec![],
            code_hash: account.code_hash,
            code_size: None,
            code_state: CodeState::Clean,
            storage_changes: HashMap::new(),
            storage_cache: Self::empty_storage_cache(),
            original_storage_cache: None,
        }
    }
}

impl StateObject {
    pub fn new(
        balance: U256,
        nonce: U256,
        storage: HashMap<H256, H256>,
        code: Bytes,
    ) -> StateObject {
        StateObject {
            balance,
            nonce,
            storage_root: KECCAK_NULL_RLP,
            code: code.clone(),
            code_hash: keccak(&code),
            code_size: Some(code.len()),
            code_state: CodeState::Dirty,
            storage_changes: storage,
            storage_cache: Self::empty_storage_cache(),
            original_storage_cache: None,
        }
    }

    pub fn new_state_object(
        balance: U256,
        nonce: U256,
        original_storage_root: H256,
    ) -> StateObject {
        StateObject {
            balance,
            nonce,
            storage_root: KECCAK_NULL_RLP, // why not original_storage_root ?
            code: vec![],
            code_hash: KECCAK_EMPTY,
            code_size: None,
            code_state: CodeState::Clean,
            storage_changes: HashMap::new(),
            storage_cache: RefCell::new(LruCache::new(STORAGE_CACHE_ITEMS)),
            original_storage_cache: if original_storage_root == KECCAK_NULL_RLP {
                None
            } else {
                Some((original_storage_root, Self::empty_storage_cache()))
            },
        }
    }

    fn empty_storage_cache() -> RefCell<LruCache<H256, H256>> {
        RefCell::new(LruCache::new(STORAGE_CACHE_ITEMS))
    }

    pub fn set_code(&mut self, code: Bytes) {
        self.code = code.clone();
        self.code_hash = keccak(&code);
        self.code_size = Some(code.len());
        self.code_state = CodeState::Dirty;
    }

    pub fn balance(&self) -> &U256 {
        &self.balance
    }

    pub fn nonce(&self) -> &U256 {
        &self.nonce
    }

    pub fn code(&self) -> Option<Bytes> {
        if self.code.is_empty() {
            return None;
        }
        Some(self.code.clone())
    }

    pub fn code_hash(&self) -> H256 {
        self.code_hash.clone()
    }

    pub fn code_size(&self) -> Option<usize> {
        self.code_size.clone()
    }

    pub fn is_code_cached(&self) -> bool {
        !self.code.is_empty() // Consider code hash or not ?
    }

    pub fn cache_code<B: DB>(&mut self, db: &mut B) -> Option<Bytes> {
        if self.is_code_cached() {
            return Some(self.code.clone());
        }

        match db.get(&self.code_hash) {
            Ok(x) => {
                self.code = x.clone().unwrap();
                self.code_size = Some(x.unwrap().len());
                Some(self.code.clone())
            }
            Err(_) => unimplemented!(),
        }
    }

    pub fn cache_given_code(&mut self, code: Bytes) {
        self.code = code.clone();
        self.code_size = Some(code.len());
    }

    pub fn storage_changes_is_null(&self) -> bool {
        self.storage_changes.is_empty()
    }

    pub fn increase_nonce(&mut self) {
        self.nonce = self.nonce + U256::from(1u8);
    }

    pub fn add_balance(&mut self, x: &U256) {
        self.balance = self.balance.saturating_add(*x);
    }

    pub fn sub_balance(&mut self, x: &U256) {
        self.balance = self.balance.saturating_sub(*x);
    }

    pub fn storage_root(&self) -> Option<H256> {
        if self.storage_changes_is_null() {
            Some(self.storage_root)
        } else {
            None
        }
    }

    pub fn set_storage(&mut self, key: H256, value: H256) {
        self.storage_changes.insert(key, value);
    }

    pub fn get_storage_changes(&self) -> &HashMap<H256, H256> {
        &self.storage_changes
    }

    pub fn cached_storage_at(&self, key: &H256) -> Option<H256> {
        if let Some(value) = self.storage_changes.get(key) {
            return Some(*value);
        }

        if let Some(value) = self.storage_cache.borrow_mut().get_mut(key) {
            return Some(*value);
        }
        None
    }

    pub fn trie_storage_at<B: DB>(&mut self, db: &mut B, key: &H256) -> Option<H256> {
        let value = db.get(key).unwrap().unwrap();

        self.storage_cache
            .borrow_mut()
            .insert(*key, H256::from_slice(&value));
        Some(H256::from_slice(&value))
    }

    pub fn commit_storage<B: DB>(&mut self, db: &mut B) {
        for (k, v) in self.storage_changes.drain() {
            if v.is_zero() {
                db.remove(&k);
            } else {
                db.insert(&k, &v);
            }
            self.storage_cache.borrow_mut().insert(k, k);
        }
    }

    pub fn commit_code<B: DB>(&mut self, db: &mut B) {
        match (self.code_state == CodeState::Dirty, self.code.is_empty()) {
            (true, true) => {
                self.code_size = Some(0);
                self.code_state = CodeState::Clean;
            }
            (true, false) => {
                db.insert(&self.code_hash.clone(), &self.code);
                self.code_size = Some(self.code.len());
                self.code_state = CodeState::Clean;
            }
            (false, _) => {}
        }
    }

    pub fn rlp(&self) -> Vec<u8> {
        let mut stream = RlpStream::new_list(4);
        stream.append(&self.balance);
        stream.append(&self.nonce);
        stream.append(&self.storage_root);
        stream.append(&self.code_hash);
        stream.out()
    }

    pub fn from_rlp(rlp: &[u8]) -> StateObject {
        let account: Account = decode(rlp).unwrap();
        account.into()
    }

    pub fn clone_basic_state_object(&self) -> StateObject {
        StateObject {
            balance: self.balance.clone(),
            nonce: self.nonce.clone(),
            storage_root: self.storage_root.clone(),
            code: self.code.clone(),
            code_hash: self.code_hash.clone(),
            code_size: self.code_size.clone(),
            code_state: self.code_state.clone(),
            storage_changes: HashMap::new(),
            storage_cache: Self::empty_storage_cache(),
            original_storage_cache: None, // FIX ME!
        }
    }

    pub fn clone_dirty_state_object(&self) -> StateObject {
        let mut state_object = self.clone_basic_state_object();
        state_object.storage_changes = self.storage_changes.clone();
        state_object
    }

    pub fn clone_all(&self) -> StateObject {
        let mut state_object = self.clone_dirty_state_object();
        state_object.storage_cache = self.storage_cache.clone();
        state_object
    }

    pub fn overwrite_with_state_object(&mut self, other: StateObject) {
        self.balance = other.balance;
        self.nonce = other.nonce;
        self.code = other.code;
        self.storage_root = other.storage_root;
        self.storage_changes = other.storage_changes;
        for (k, v) in other.storage_cache.into_inner() {
            self.storage_cache.borrow_mut().insert(k, v);
        }
    }
}
