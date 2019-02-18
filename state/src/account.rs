use cita_trie::codec::RLPNodeCodec;
use cita_trie::db::DB;
use cita_trie::trie::{PatriciaTrie, Trie};
use ethereum_types::{H256, U256};
use keccak_hash::{keccak, KECCAK_EMPTY, KECCAK_NULL_RLP};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Account {
    balance: U256,
    nonce: U256,
    storage_root: H256,
    code_hash: H256,
}

impl rlp::Encodable for Account {
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        s.begin_list(4)
            .append(&self.nonce)
            .append(&self.balance)
            .append(&self.storage_root)
            .append(&self.code_hash);
    }
}

impl rlp::Decodable for Account {
    fn decode(data: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        Ok(Account {
            nonce: data.val_at(0)?,
            balance: data.val_at(1)?,
            storage_root: data.val_at(2)?,
            code_hash: data.val_at(3)?,
        })
    }
}

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
    code_hash: H256,
    code: Vec<u8>,
    code_size: usize,
    code_state: CodeState,
    storage_changes: HashMap<H256, H256>,
}


impl From<Account> for StateObject {
    fn from(account: Account) -> Self {
        StateObject {
            balance: account.balance,
            nonce: account.nonce,
            storage_root: account.storage_root,
            code_hash: account.code_hash,
            code: vec![],
            code_size: 0,
            code_state: CodeState::Clean,
            storage_changes: HashMap::new(),
        }
    }
}


impl StateObject {

    /// Create a new account.
    /// NOTE: If contract account generated, make sure you use `init_code` on
    /// this before `commit`ing.
    pub fn new(balance: U256, nonce: U256) -> StateObject {
        StateObject {
            balance: balance,
            nonce: nonce,
            storage_root: KECCAK_NULL_RLP,
            code_hash: KECCAK_EMPTY,
            code: vec![],
            code_size: 0,
            code_state: CodeState::Clean,
            storage_changes: HashMap::new(),
        }
    }

    /// Create a new account from rlp bytes.
    /// Note: make sure you use `read_code` after this.
    pub fn from_rlp(data: &[u8]) -> StateObject {
        let account: Account = rlp::decode(data).unwrap();
        account.into()
    }

    pub fn account(&self) -> Account {
        Account {
            balance: self.balance,
            nonce: self.nonce,
            storage_root: self.storage_root,
            code_hash: self.code_hash,
        }
    }

    pub fn init_code(&mut self, code: Vec<u8>) {
        self.code = code.clone();
        self.code_hash = keccak(&code);
        self.code_size = code.len();
        self.code_state = CodeState::Dirty;
    }


    pub fn read_code<B: DB>(&mut self, db: &mut B) -> Vec<u8> {
        if self.code_hash == KECCAK_EMPTY {
            return vec![]
        }
        if !self.code.is_empty() {
            return self.code.clone()
        }
        return db.get(&self.code_hash).unwrap().unwrap()
    }

    pub fn balance(&self) -> U256 {
        self.balance.clone()
    }

    pub fn nonce(&self) -> U256 {
        self.nonce.clone()
    }

    pub fn code(&self) -> Option<Vec<u8>> {
        if self.code.is_empty() {
            return None;
        }
        Some(self.code.clone())
    }

    pub fn code_hash(&self) -> H256 {
        self.code_hash.clone()
    }

    pub fn code_size(&self) -> usize {
        self.code_size
    }

    pub fn inc_nonce(&mut self) {
        self.nonce = self.nonce + U256::from(1u8);
    }

    pub fn add_balance(&mut self, x: U256) {
        let (a, b) = self.balance.overflowing_add(x);
        assert_eq!(b, false);
        self.balance = a;
    }

    pub fn sub_balance(&mut self, x: U256) {
        let (a, b) = self.balance.overflowing_sub(x);
        assert_eq!(b, false);
        self.balance = a;
    }

    pub fn set_storage(&mut self, key: H256, value: H256) {
        self.storage_changes.insert(key, value);
    }

    pub fn get_storage_at_backend<B: DB>(&mut self, db: &mut B, key: &H256) -> Option<H256> {
        let trie = PatriciaTrie::from(db, RLPNodeCodec::default(), &self.storage_root.0).unwrap();
        if let Ok(a) =  trie.get(key) {
            if let Some(b) = a {
                return Some( From::from(&b[..]) )
            }
        }
        return None
    }

    pub fn get_storage_at_changes(&self, key: &H256) -> Option<H256> {
        if let Some(value) = self.storage_changes.get(key) {
            return Some(*value);
        }
        None
    }

    pub fn get_storage<B: DB>(&mut self, key: &H256, db: &mut B) -> Option<H256> {
        if let Some(value) = self.get_storage_at_changes(key) {
            return Some(value);
        }
        if let Some(value) = self.get_storage_at_backend(db, key) {
            return Some(value);
        }
        None
    }

    pub fn commit_storage<B: DB>(&mut self, db: &mut B) {
        let mut trie = PatriciaTrie::from(db, RLPNodeCodec::default(), &self.storage_root.0).unwrap();
        for (k, v) in self.storage_changes.drain() {
            if v.is_zero() {
                trie.remove(&k).unwrap();
            } else {
                trie.insert(&k, &v).unwrap();
            }
        }
    }

    pub fn commit_code<B: DB>(&mut self, db: &mut B){
        match (self.code_state == CodeState::Dirty, self.code.is_empty()) {
            (true, true) => {
                self.code_size = 0;
                self.code_state = CodeState::Clean;
            }
            (true, false) => {
                db.insert(&self.code_hash.clone(), &self.code).unwrap();
                self.code_size = self.code.len();
                self.code_state = CodeState::Clean;
            }
            (false, _) => {}
        }
    }

    pub fn clone_clean(&self) -> StateObject {
        StateObject {
            balance: self.balance.clone(),
            nonce: self.nonce.clone(),
            storage_root: self.storage_root.clone(),
            code: self.code.clone(),
            code_hash: self.code_hash.clone(),
            code_size: self.code_size.clone(),
            code_state: self.code_state.clone(),
            storage_changes: HashMap::new(),
        }
    }

    pub fn clone_dirty(&self) -> StateObject {
        let mut state_object = self.clone_clean();
        state_object.storage_changes = self.storage_changes.clone();
        state_object
    }

    pub fn merge(&mut self, other: StateObject) {
        self.balance = other.balance;
        self.nonce = other.nonce;
        self.storage_root = other.storage_root;
        self.code_hash = other.code_hash;
        self.code_state = other.code_state;
        self.code = other.code;
        self.code_size = other.code_size;
        self.storage_changes = other.storage_changes;
    }
}
