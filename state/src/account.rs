use super::err::Error;
use super::hashlib;
use cita_trie::db::DB;
use cita_trie::trie::{PatriciaTrie, Trie};
use ethereum_types::{Address, H256, U256};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Account {
    pub balance: U256,
    pub nonce: U256,
    pub storage_root: H256,
    pub code_hash: H256,
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
    pub address: Address,
    pub balance: U256,
    pub nonce: U256,
    pub storage_root: H256,
    pub code_hash: H256,
    pub code: Vec<u8>,
    pub code_size: usize,
    pub code_state: CodeState,
    storage_changes: HashMap<H256, H256>,
}

impl From<Account> for StateObject {
    fn from(account: Account) -> Self {
        StateObject {
            address: Address::zero(),
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
            address: Address::zero(),
            balance,
            nonce,
            storage_root: hashlib::RLP_NULL,
            code_hash: hashlib::NIL_DATA,
            code: vec![],
            code_size: 0,
            code_state: CodeState::Clean,
            storage_changes: HashMap::new(),
        }
    }

    /// Create a new account from rlp bytes.
    /// Note: make sure you use `read_code` after this.
    pub fn from_rlp(data: &[u8]) -> Result<StateObject, Error> {
        let account: Account = rlp::decode(data)?;
        Ok(account.into())
    }

    pub fn rlp(&self) -> Vec<u8> {
        rlp::encode(&self.account())
    }

    pub fn account(&self) -> Account {
        Account {
            balance: self.balance,
            nonce: self.nonce,
            storage_root: self.storage_root,
            code_hash: self.code_hash,
        }
    }

    pub fn set_address(&mut self, address: &Address) {
        self.address = *address;
    }

    // is_empty returns whether the given account is empty. Empty
    // is defined according to EIP161 (balance = nonce = code = 0).
    pub fn is_empty(&self) -> bool {
        self.balance.is_zero() && self.nonce.is_zero() && self.code_hash == hashlib::NIL_DATA
    }

    pub fn init_code(&mut self, code: Vec<u8>) {
        self.code = code;
        self.code_hash = From::from(&hashlib::summary(&self.code)[..]);
        self.code_size = self.code.len();
        self.code_state = CodeState::Dirty;
    }

    pub fn read_code<B: DB>(&mut self, db: &mut B) -> Result<(), Error> {
        if self.code_hash == hashlib::NIL_DATA {
            return Ok(());
        }
        let c = db
            .get(&self.code_hash)
            .or_else(|e| Err(Error::DB(format!("{}", e))))?
            .unwrap_or_else(|| vec![]);
        self.code = c;
        self.code_size = self.code.len();
        self.code_state = CodeState::Clean;
        Ok(())
    }

    pub fn balance(&self) -> U256 {
        self.balance
    }

    pub fn nonce(&self) -> U256 {
        self.nonce
    }

    pub fn code(&self) -> Vec<u8> {
        self.code.clone()
    }

    pub fn code_hash(&self) -> H256 {
        self.code_hash
    }

    pub fn code_size(&self) -> usize {
        self.code_size
    }

    pub fn inc_nonce(&mut self) {
        self.nonce += U256::from(1u8);
    }

    pub fn add_balance(&mut self, x: U256) {
        let (a, b) = self.balance.overflowing_add(x);
        // overflow is not allowed at state_object.
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

    pub fn get_storage_at_backend<B: DB>(&mut self, db: &mut B, key: &H256) -> Result<Option<H256>, Error> {
        if self.storage_root == hashlib::RLP_NULL {
            return Ok(None);
        }
        let trie = PatriciaTrie::from(db, hashlib::RLPNodeCodec::default(), &self.storage_root.0)
            .or_else(|e| Err(Error::DB(format!("{}", e))))?;
        if let Some(b) = trie.get(&hashlib::encodek(key))? {
            let u256_k: U256 = From::from(&hashlib::decodev(&b)?[..]);
            let h256_k: H256 = u256_k.into();
            return Ok(Some(h256_k));
        }
        Ok(None)
    }

    pub fn get_storage_at_changes(&self, key: &H256) -> Option<H256> {
        self.storage_changes.get(key).and_then(|e| Some(*e))
    }

    pub fn get_storage<B: DB>(&mut self, db: &mut B, key: &H256) -> Result<Option<H256>, Error> {
        if let Some(value) = self.get_storage_at_changes(key) {
            return Ok(Some(value));
        }
        if let Some(value) = self.get_storage_at_backend(db, key)? {
            return Ok(Some(value));
        }
        Ok(None)
    }

    pub fn commit_storage<B: DB>(&mut self, db: &mut B) -> Result<(), Error> {
        let mut trie = if self.storage_root == hashlib::RLP_NULL {
            PatriciaTrie::new(db, hashlib::RLPNodeCodec::default())
        } else {
            PatriciaTrie::from(db, hashlib::RLPNodeCodec::default(), &self.storage_root.0)?
        };
        for (k, v) in self.storage_changes.drain() {
            // Parity uses `SecTrie` to remove or insert data. Only diffirence
            // between is that parity use keccak(k) as k.
            if v.is_zero() {
                trie.remove(&hashlib::encodek(&k))?;
            } else {
                trie.insert(&hashlib::encodek(&k), &hashlib::encodev(&U256::from(v)))?;
            }
        }
        self.storage_root = trie.root()?.into();
        Ok(())
    }

    pub fn commit_code<B: DB>(&mut self, db: &mut B) -> Result<(), Error> {
        match (self.code_state == CodeState::Dirty, self.code.is_empty()) {
            (true, true) => {
                self.code_size = 0;
                self.code_state = CodeState::Clean;
            }
            (true, false) => {
                db.insert(&self.code_hash, &self.code)
                    .or_else(|e| Err(Error::DB(format!("{}", e))))?;
                self.code_size = self.code.len();
                self.code_state = CodeState::Clean;
            }
            (false, _) => {}
        }
        Ok(())
    }

    pub fn clone_clean(&self) -> StateObject {
        StateObject {
            address: self.address,
            balance: self.balance,
            nonce: self.nonce,
            storage_root: self.storage_root,
            code: self.code.clone(),
            code_hash: self.code_hash,
            code_size: self.code_size,
            code_state: self.code_state,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_object_new() {
        let o = StateObject::new(69u8.into(), 0u8.into());
        assert_eq!(o.balance(), 69u8.into());
        assert_eq!(o.nonce(), 0u8.into());
        assert_eq!(o.code_hash(), hashlib::NIL_DATA);
        assert_eq!(o.storage_root, hashlib::RLP_NULL);
        assert_eq!(hex::encode(rlp::encode(&o.account())), "f8448045a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a0c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470");
    }

    #[test]
    fn state_object_rlp() {
        let a = StateObject::new(69u8.into(), 0u8.into());
        let b = StateObject::from_rlp(&rlp::encode(&a.account())[..]).unwrap();
        assert_eq!(a.balance(), b.balance());
        assert_eq!(a.nonce(), b.nonce());
        assert_eq!(a.code_hash(), b.code_hash());
        assert_eq!(a.storage_root, b.storage_root);
    }

    #[test]
    fn state_object_code() {
        let mut a = StateObject::new(69u8.into(), 0.into());
        let mut db = cita_trie::db::MemoryDB::new();
        a.init_code(vec![0x55, 0x44, 0xffu8]);
        assert_eq!(a.code_state, CodeState::Dirty);
        assert_eq!(a.code_size, 3);
        a.commit_code(&mut db).unwrap();
        assert_eq!(a.code_state, CodeState::Clean);
        assert_eq!(
            a.code_hash,
            "af231e631776a517ca23125370d542873eca1fb4d613ed9b5d5335a46ae5b7eb".into()
        );
        assert_eq!(db.get(&a.code_hash()).unwrap().unwrap(), vec![0x55, 0x44, 0xffu8]);
        a.init_code(vec![0x55]);
        assert_eq!(a.code_state, CodeState::Dirty);
        assert_eq!(a.code_size, 1);
        a.commit_code(&mut db).unwrap();
        assert_eq!(
            a.code_hash,
            "37bf2238b11b68cdc8382cece82651b59d3c3988873b6e0f33d79694aa45f1be".into()
        );
        assert_eq!(db.get(&a.code_hash()).unwrap().unwrap(), vec![0x55]);
    }

    #[test]
    fn state_object_storage_1() {
        let mut a = StateObject::new(69u8.into(), 0.into());
        let mut db = cita_trie::db::MemoryDB::new();
        a.set_storage(0.into(), 0x1234.into());
        a.commit_storage(&mut db).unwrap();
        assert_eq!(
            a.storage_root,
            "c57e1afb758b07f8d2c8f13a3b6e44fa5ff94ab266facc5a4fd3f062426e50b2".into()
        );
    }

    #[test]
    fn state_object_storage_2() {
        let mut a = StateObject::new(69u8.into(), 0.into());
        let mut db = cita_trie::db::MemoryDB::new();
        a.set_storage(0.into(), 0x1234.into());
        a.commit_storage(&mut db).unwrap();
        assert_eq!(
            a.storage_root,
            "c57e1afb758b07f8d2c8f13a3b6e44fa5ff94ab266facc5a4fd3f062426e50b2".into()
        );
        a.set_storage(1.into(), 0x1234.into());
        a.commit_storage(&mut db).unwrap();
        assert_eq!(
            a.storage_root,
            "4e49574efd650366d071855e0a3975123ea9d64cc945e8f5de8c8c517e1b4ca5".into()
        );
        a.set_storage(1.into(), 0.into());
        a.commit_storage(&mut db).unwrap();
        assert_eq!(
            a.storage_root,
            "c57e1afb758b07f8d2c8f13a3b6e44fa5ff94ab266facc5a4fd3f062426e50b2".into()
        );
    }

    #[test]
    fn state_object_storage_3() {
        let mut a = StateObject::new(69u8.into(), 0.into());
        let mut db = cita_trie::db::MemoryDB::new();
        let a_rlp = {
            a.set_storage(0x00u64.into(), 0x1234u64.into());
            a.commit_storage(&mut db).unwrap();
            a.init_code(vec![]);
            a.commit_code(&mut db).unwrap();
            rlp::encode(&a.account())
        };
        a = StateObject::from_rlp(&a_rlp[..]).unwrap();
        assert_eq!(
            a.storage_root,
            "c57e1afb758b07f8d2c8f13a3b6e44fa5ff94ab266facc5a4fd3f062426e50b2".into()
        );
        assert_eq!(
            a.get_storage(&mut db, &0x00u64.into()).unwrap().unwrap(),
            0x1234u64.into()
        );
        assert_eq!(a.get_storage(&mut db, &0x01u64.into()).unwrap(), None);
    }

    #[test]
    fn state_object_note_code() {
        let mut a = StateObject::new(69u8.into(), 0.into());
        let mut db = cita_trie::db::MemoryDB::new();
        let a_rlp = {
            a.init_code(vec![0x55, 0x44, 0xffu8]);
            a.commit_code(&mut db).unwrap();
            a.rlp()
        };
        a = StateObject::from_rlp(&a_rlp[..]).unwrap();
        a.read_code(&mut db).unwrap();
        assert_eq!(a.code, vec![0x55, 0x44, 0xffu8]);
    }
}
