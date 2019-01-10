use cita_trie::codec::RLPNodeCodec;
use cita_trie::db::MemoryDB;
use cita_trie::trie::{PatriciaTrie, Trie};
use ethereum_types::{H256, U256};
use lru_cache::LruCache;
use rlp::*;
use std::cell::RefCell;
use std::collections::HashMap;

const STORAGE_CACHE_ITEMS: usize = 8192;

type Bytes = Vec<u8>;

#[derive(Debug)]
pub struct Account {
    balance: U256,
    nonce: U256,
    code: Bytes,
    storage_root: H256,
    storage_cache: RefCell<LruCache<H256, H256>>,
    storage_changes: HashMap<H256, H256>,
}

#[derive(Debug)]
pub struct BasicAccount {
    balance: U256,
    nonce: U256,
    // code: Arc<Bytes>,
    storage_root: H256,
}

impl rlp::Encodable for BasicAccount {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(4)
            .append(&self.balance)
            .append(&self.nonce)
            // .append(&self.code)
            .append(&self.storage_root);
    }
}

impl rlp::Decodable for BasicAccount {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        Ok(BasicAccount {
            balance: rlp.val_at(0)?,
            nonce: rlp.val_at(1)?,
            // code: rlp.val_at(2)?,
            storage_root: rlp.val_at(3)?,
        })
    }
}

impl From<BasicAccount> for Account {
    fn from(basic: BasicAccount) -> Self {
        Account {
            balance: basic.balance,
            nonce: basic.nonce,
            code: vec![],
            storage_root: basic.storage_root,
            storage_cache: RefCell::new(LruCache::new(STORAGE_CACHE_ITEMS)),
            storage_changes: HashMap::new(),
        }
    }
}

impl Account {
    pub fn new(balance: U256, nonce: U256, storage: HashMap<H256, H256>, code: Bytes) -> Account {
        Account {
            balance,
            nonce,
            code: code,
            storage_root: H256::default(),
            storage_cache: RefCell::new(LruCache::new(STORAGE_CACHE_ITEMS)),
            storage_changes: storage,
        }
    }

    pub fn new_contract(balance: U256, nonce: U256) -> Account {
        Account {
            balance,
            nonce,
            code: vec![],
            storage_root: H256::default(),
            storage_cache: RefCell::new(LruCache::new(STORAGE_CACHE_ITEMS)),
            storage_changes: HashMap::new(),
        }
    }

    pub fn init_code(&mut self, code: Bytes) {
        self.code = code;
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

    pub fn storage_root(&self) -> Option<&H256> {
        if self.storage_changes_is_null() {
            Some(&self.storage_root)
        } else {
            None
        }
    }

    pub fn get_storage_changes(&self) -> &HashMap<H256, H256> {
        &self.storage_changes
    }

    pub fn set_storage(&mut self, key: H256, value: H256) {
        self.storage_changes.insert(key, value);
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

    pub fn trie_storage_at(&mut self, db: &mut MemoryDB, key: &H256) -> Option<H256> {
        let trie = PatriciaTrie::from(db, RLPNodeCodec::default(), &self.storage_root.0).unwrap();
        let value = trie.get(key).unwrap().unwrap();

        self.storage_cache
            .borrow_mut()
            .insert(*key, H256::from_slice(&value));
        Some(H256::from_slice(&value))
    }

    pub fn commit_storage(&mut self, db: &mut MemoryDB) {
        let mut trie =
            PatriciaTrie::from(db, RLPNodeCodec::default(), &self.storage_root.0).unwrap();
        for (k, v) in self.storage_changes.drain() {
            if v.is_zero() {
                trie.remove(&k);
            } else {
                trie.insert(&k, &v);
            }
            self.storage_cache.borrow_mut().insert(k, k);
        }
    }

    pub fn rlp(&self) -> Vec<u8> {
        let mut stream = RlpStream::new_list(4);
        stream.append(&self.balance);
        stream.append(&self.nonce);
        // stream.append(&self.code);
        stream.append(&self.storage_root);
        stream.out()
    }

    pub fn from_rlp(rlp: &[u8]) -> Account {
        let basic_account: BasicAccount = decode(rlp).unwrap();
        basic_account.into()
    }

    pub fn clone_basic(&self) -> Account {
        Account {
            balance: self.balance.clone(),
            nonce: self.nonce.clone(),
            code: self.code.clone(),
            storage_root: self.storage_root,
            storage_cache: RefCell::new(LruCache::new(STORAGE_CACHE_ITEMS)),
            storage_changes: HashMap::new(),
        }
    }

    pub fn clone_dirty_account(&self) -> Account {
        let mut account = self.clone_basic();
        account.storage_changes = self.storage_changes.clone();
        account
    }

    pub fn clone_all(&self) -> Account {
        let mut account = self.clone_dirty_account();
        account.storage_cache = self.storage_cache.clone();
        account
    }

    pub fn overwrite_with_account(&mut self, other: Account) {
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
