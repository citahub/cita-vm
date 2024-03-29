use std::collections::BTreeMap;
use std::sync::Arc;

use cita_trie::{PatriciaTrie, Trie, DB};
use ethereum_types::{BigEndianHash, H256, U256};
use hashbrown::HashMap;

use crate::common;
use crate::common::hash;
use crate::state::err::Error;

/// Single and pure account in the database. Usually, store it according to
/// the following structure:
/// Key: address -> Value: rlp::encode(account).
#[derive(Debug)]
pub struct Account {
    pub balance: U256,
    pub nonce: U256,
    pub storage_root: H256,
    pub code_hash: H256,
    pub abi_hash: H256,
}

/// Free to use rlp::encode().
impl rlp::Encodable for Account {
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        s.begin_list(5)
            .append(&self.nonce)
            .append(&self.balance)
            .append(&self.storage_root)
            .append(&self.code_hash)
            .append(&self.abi_hash);
    }
}

/// Free to use rlp::decode().
impl rlp::Decodable for Account {
    fn decode(data: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        Ok(Account {
            nonce: data.val_at(0)?,
            balance: data.val_at(1)?,
            storage_root: data.val_at(2)?,
            code_hash: data.val_at(3)?,
            abi_hash: data.val_at(4)?,
        })
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum CodeState {
    Clean,
    Dirty,
}

#[derive(Debug, Clone)]
pub struct StateObject {
    pub balance: U256,
    pub nonce: U256,
    pub storage_root: H256,
    pub code_hash: H256,
    pub code: Vec<u8>,
    pub code_size: usize,
    pub code_state: CodeState,
    pub abi_hash: H256,
    pub abi: Vec<u8>,
    pub abi_size: usize,
    pub abi_state: CodeState,
    pub storage_changes: HashMap<H256, H256>,
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
            abi_hash: account.abi_hash,
            abi: vec![],
            abi_size: 0,
            abi_state: CodeState::Clean,
            storage_changes: HashMap::new(),
        }
    }
}

//const CODE_PREFIX: &str = "C:";
//const ABI_PREFIX: &str = "ABI:";

impl StateObject {
    /// Create a new account.
    /// Note: If contract account generated, make sure you use `init_code` on
    /// this before `commit`ing.
    pub fn new(balance: U256, nonce: U256) -> StateObject {
        StateObject {
            balance,
            nonce,
            storage_root: common::hash::RLP_NULL,
            code_hash: common::hash::NIL_DATA,
            code: vec![],
            code_size: 0,
            code_state: CodeState::Clean,
            abi_hash: common::hash::NIL_DATA,
            abi: vec![],
            abi_size: 0,
            abi_state: CodeState::Clean,
            storage_changes: HashMap::new(),
        }
    }

    /// Create a new account from rlp bytes.
    /// Note: make sure you use `read_code` after this.
    pub fn from_rlp(data: &[u8]) -> Result<StateObject, Error> {
        let account: Account = rlp::decode(data)?;
        Ok(account.into())
    }

    /// Get the account from state object.
    pub fn account(&self) -> Account {
        Account {
            balance: self.balance,
            nonce: self.nonce,
            storage_root: self.storage_root,
            code_hash: self.code_hash,
            abi_hash: self.abi_hash,
        }
    }

    /// Get the rlp data.
    pub fn rlp(&self) -> Vec<u8> {
        rlp::encode(&self.account()).to_vec()
    }

    /// Function is_empty returns whether the given account is empty. Empty
    /// is defined according to EIP161 (balance = nonce = code = 0).
    pub fn is_empty(&self) -> bool {
        self.balance.is_zero()
            && self.nonce.is_zero()
            && self.code_hash == common::hash::NIL_DATA
            && self.storage_root == common::hash::RLP_NULL
    }

    /// Init the code by given data.
    pub fn init_code(&mut self, code: Vec<u8>) {
        self.code = code;
        self.code_size = self.code.len();
        self.code_hash = {
            if self.code_size > 0 {
                H256::from_slice(common::hash::summary(&self.code).as_slice())
            } else {
                common::hash::NIL_DATA
            }
        };
        self.code_state = CodeState::Dirty;
    }

    /// Init the abi by given data.
    pub fn init_abi(&mut self, abi: Vec<u8>) {
        self.abi = abi;
        self.abi_size = self.abi.len();
        self.abi_hash = {
            if self.abi_size > 0 {
                H256::from_slice(common::hash::summary(&self.abi).as_slice())
            } else {
                common::hash::NIL_DATA
            }
        };
        self.abi_state = CodeState::Dirty;
    }

    /// Read the code from database by it's codehash.
    pub fn read_code<B: DB>(&mut self, db: Arc<B>) -> Result<(), Error> {
        if self.code_hash == common::hash::NIL_DATA {
            return Ok(());
        }
        let c = db
            .get(self.code_hash.as_bytes())
            .map_err(|e| Error::DB(format!("{}", e)))?
            .unwrap_or_default();
        self.code = c;
        self.code_size = self.code.len();
        self.code_state = CodeState::Clean;
        Ok(())
    }

    /// Read the abi from database by it's abihash.
    pub fn read_abi<B: DB>(&mut self, db: Arc<B>) -> Result<(), Error> {
        if self.abi_hash == common::hash::NIL_DATA {
            return Ok(());
        }
        let c = db
            .get(self.abi_hash.as_bytes())
            .map_err(|e| Error::DB(format!("{}", e)))?
            .unwrap_or_default();
        self.abi = c;
        self.abi_size = self.abi.len();
        self.abi_state = CodeState::Clean;
        Ok(())
    }

    /// Add nonce by 1.
    pub fn inc_nonce(&mut self) {
        self.nonce += U256::from(1u8);
    }

    /// Add balance.
    /// Note: overflowing is not allowed.
    pub fn add_balance(&mut self, x: U256) {
        let (a, b) = self.balance.overflowing_add(x);
        // overflow is not allowed at state_object.
        assert!(!b);
        self.balance = a;
    }

    /// Sub balance.
    /// Note: overflowing is not allowed.
    pub fn sub_balance(&mut self, x: U256) {
        self.balance = self.balance.saturating_sub(x);
    }

    /// Set (key, value) in storage cache.
    pub fn set_storage(&mut self, key: H256, value: H256) {
        self.storage_changes.insert(key, value);
    }

    /// Get value by key from database.
    pub fn get_storage_at_backend<B: DB>(&self, db: Arc<B>, key: &H256) -> Result<Option<H256>, Error> {
        if self.storage_root == common::hash::RLP_NULL {
            return Ok(None);
        }
        let trie = PatriciaTrie::from(db, Arc::new(hash::get_hasher()), &self.storage_root.0)?;
        if let Some(b) = trie.get(key.as_bytes())? {
            let u256_k: U256 = rlp::decode(&b)?;
            let h256_k = H256::from_uint(&u256_k);
            return Ok(Some(h256_k));
        }
        Ok(None)
    }

    /// Get value by key from storage cache.
    pub fn get_storage_at_changes(&self, key: &H256) -> Option<H256> {
        self.storage_changes.get(key).copied()
    }

    /// Get the storage changes
    pub fn get_storage_changes(&self) -> BTreeMap<String, String> {
        let mut result = BTreeMap::new();
        for (k, v) in self.storage_changes.iter() {
            let key = String::from("0x") + &hex::encode(*k);
            let value = String::from("0x") + &hex::encode(*v);
            result.insert(key.clone(), value.clone());
        }
        result
    }

    /// Get value by key.
    pub fn get_storage<B: DB>(&self, db: Arc<B>, key: &H256) -> Result<Option<H256>, Error> {
        if let Some(value) = self.get_storage_at_changes(key) {
            return Ok(Some(value));
        }
        if let Some(value) = self.get_storage_at_backend(db, key)? {
            return Ok(Some(value));
        }
        Ok(None)
    }

    /// Get storage proof
    pub fn get_storage_proof<B: DB>(&self, db: Arc<B>, key: &H256) -> Result<Vec<Vec<u8>>, Error> {
        let trie = PatriciaTrie::from(db, Arc::new(hash::get_hasher()), &self.storage_root.0)
            .map_err(|e| Error::DB(format!("StateObject::get_storage_proof: {}", e)))?;
        let proof = trie.get_proof(&key.0)?;
        Ok(proof)
    }

    /// Flush data in storage cache to database.
    pub fn commit_storage<B: DB>(&mut self, db: Arc<B>) -> Result<(), Error> {
        let mut trie = if self.storage_root == common::hash::RLP_NULL {
            PatriciaTrie::new(db, Arc::new(hash::get_hasher()))
        } else {
            PatriciaTrie::from(db, Arc::new(hash::get_hasher()), &self.storage_root.0)?
        };

        for (k, v) in self.storage_changes.drain() {
            if v.is_zero() {
                trie.remove(k.as_bytes())?;
            } else {
                trie.insert(k.0.to_vec(), rlp::encode(&U256::from(v.as_bytes())).to_vec())?;
            }
        }

        self.storage_root = H256::from_slice(trie.root()?.as_slice());
        Ok(())
    }

    /// Flush code to database if necessary.
    pub fn commit_code<B: DB>(&mut self, db: Arc<B>) -> Result<(), Error> {
        match (self.code_state == CodeState::Dirty, self.code.is_empty()) {
            (true, true) => {
                self.code_size = 0;
                self.code_state = CodeState::Clean;
            }
            (true, false) => {
                db.insert(self.code_hash.0.to_vec(), self.code.clone())
                    .map_err(|e| Error::DB(format!("{}", e)))?;
                self.code_size = self.code.len();
                self.code_state = CodeState::Clean;
            }
            (false, _) => {}
        }
        Ok(())
    }

    /// Flush abi to database if necessary.
    pub fn commit_abi<B: DB>(&mut self, db: Arc<B>) -> Result<(), Error> {
        match (self.abi_state == CodeState::Dirty, self.abi.is_empty()) {
            (true, true) => {
                self.abi_size = 0;
                self.abi_state = CodeState::Clean;
            }
            (true, false) => {
                db.insert(self.abi_hash.0.to_vec(), self.abi.clone())
                    .map_err(|e| Error::DB(format!("{}", e)))?;
                self.abi_size = self.abi.len();
                self.abi_state = CodeState::Clean;
            }
            (false, _) => {}
        }
        Ok(())
    }

    /// Clone without storage changes.
    pub fn clone_clean(&self) -> StateObject {
        StateObject {
            balance: self.balance,
            nonce: self.nonce,
            storage_root: self.storage_root,
            code: self.code.clone(),
            code_hash: self.code_hash,
            code_size: self.code_size,
            code_state: self.code_state,
            abi: self.abi.clone(),
            abi_hash: self.abi_hash,
            abi_size: self.abi_size,
            abi_state: self.abi_state,
            storage_changes: HashMap::new(),
        }
    }

    /// Clone with storage changes.
    pub fn clone_dirty(&self) -> StateObject {
        let mut state_object = self.clone_clean();
        state_object.storage_changes = self.storage_changes.clone();
        state_object
    }

    /// Merge with others.
    pub fn merge(&mut self, other: StateObject) {
        self.balance = other.balance;
        self.nonce = other.nonce;
        self.storage_root = other.storage_root;
        self.code_hash = other.code_hash;
        self.code_state = other.code_state;
        self.code = other.code;
        self.code_size = other.code_size;
        self.abi_hash = other.abi_hash;
        self.abi_state = other.abi_state;
        self.abi = other.abi;
        self.abi_size = other.abi_size;
        self.storage_changes = other.storage_changes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn state_object_new() {
        let o = StateObject::new(69u8.into(), 0u8.into());
        assert_eq!(o.balance, 69u8.into());
        assert_eq!(o.nonce, 0u8.into());
        assert_eq!(o.code_hash, common::hash::NIL_DATA);
        assert_eq!(o.abi_hash, common::hash::NIL_DATA);
        assert_eq!(o.storage_root, common::hash::RLP_NULL);
        assert_eq!(hex::encode(rlp::encode(&o.account())), "f8658045a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a0c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470a0c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470");
    }

    #[test]
    fn state_object_rlp() {
        let a = StateObject::new(69u8.into(), 0u8.into());
        let b = StateObject::from_rlp(&rlp::encode(&a.account())[..]).unwrap();
        assert_eq!(a.balance, b.balance);
        assert_eq!(a.nonce, b.nonce);
        assert_eq!(a.code_hash, b.code_hash);
        assert_eq!(a.storage_root, b.storage_root);
    }

    #[test]
    fn state_object_code() {
        let mut a = StateObject::new(69u8.into(), 0.into());
        let db = Arc::new(cita_trie::MemoryDB::new(false));
        a.init_code(vec![0x55, 0x44, 0xffu8]);
        assert_eq!(a.code_state, CodeState::Dirty);
        assert_eq!(a.code_size, 3);
        a.commit_code(Arc::clone(&db)).unwrap();
        assert_eq!(a.code_state, CodeState::Clean);
        assert_eq!(
            a.code_hash,
            H256::from_str("af231e631776a517ca23125370d542873eca1fb4d613ed9b5d5335a46ae5b7eb").unwrap()
        );

        let k = a.code_hash.0.to_vec();
        assert_eq!(db.get(&k).unwrap().unwrap(), vec![0x55, 0x44, 0xffu8]);
        a.init_code(vec![0x55]);
        assert_eq!(a.code_state, CodeState::Dirty);
        assert_eq!(a.code_size, 1);
        a.commit_code(Arc::clone(&db)).unwrap();
        assert_eq!(
            a.code_hash,
            H256::from_str("37bf2238b11b68cdc8382cece82651b59d3c3988873b6e0f33d79694aa45f1be").unwrap()
        );

        let k = a.code_hash.0.to_vec();
        assert_eq!(db.get(&k).unwrap().unwrap(), vec![0x55]);
    }

    #[test]
    fn state_object_storage_1() {
        let mut a = StateObject::new(69u8.into(), 0.into());
        let db = Arc::new(cita_trie::MemoryDB::new(false));
        a.set_storage(H256::zero(), H256::from_low_u64_be(0x1234));
        a.commit_storage(Arc::clone(&db)).unwrap();
        assert_eq!(
            a.storage_root,
            H256::from_str("71623f5ec821de33ad5aa81f8c82f0916c6f60de0a536f8c466d440c56715bd5").unwrap()
        );
    }

    #[test]
    fn state_object_storage_2() {
        let mut a = StateObject::new(69u8.into(), 0.into());
        let db = Arc::new(cita_trie::MemoryDB::new(false));
        a.set_storage(H256::zero(), H256::from_low_u64_be(0x1234));
        a.commit_storage(Arc::clone(&db)).unwrap();
        assert_eq!(
            a.storage_root,
            //"c57e1afb758b07f8d2c8f13a3b6e44fa5ff94ab266facc5a4fd3f062426e50b2".into()
            H256::from_str("71623f5ec821de33ad5aa81f8c82f0916c6f60de0a536f8c466d440c56715bd5").unwrap()
        );
        a.set_storage(H256::from_low_u64_be(1), H256::from_low_u64_be(0x1234));
        a.commit_storage(Arc::clone(&db)).unwrap();
        assert_eq!(
            a.storage_root,
            //"4e49574efd650366d071855e0a3975123ea9d64cc945e8f5de8c8c517e1b4ca5".into()
            H256::from_str("a3db671bd0653a641fb031dccb869982da390eade9e6f993802ed09c4f6b7b2a").unwrap()
        );
        a.set_storage(H256::from_low_u64_be(1), H256::zero());
        a.commit_storage(Arc::clone(&db)).unwrap();
        assert_eq!(
            a.storage_root,
            //"c57e1afb758b07f8d2c8f13a3b6e44fa5ff94ab266facc5a4fd3f062426e50b2".into()
            H256::from_str("71623f5ec821de33ad5aa81f8c82f0916c6f60de0a536f8c466d440c56715bd5").unwrap()
        );
    }

    #[test]
    fn state_object_storage_3() {
        let mut a = StateObject::new(69u8.into(), 0.into());
        let db = Arc::new(cita_trie::MemoryDB::new(false));
        let a_rlp = {
            a.set_storage(H256::zero(), H256::from_low_u64_be(0x1234));
            a.commit_storage(Arc::clone(&db)).unwrap();
            a.init_code(vec![]);
            a.commit_code(Arc::clone(&db)).unwrap();
            rlp::encode(&a.account())
        };
        a = StateObject::from_rlp(&a_rlp[..]).unwrap();
        assert_eq!(
            a.storage_root,
            //"c57e1afb758b07f8d2c8f13a3b6e44fa5ff94ab266facc5a4fd3f062426e50b2".into()
            H256::from_str("71623f5ec821de33ad5aa81f8c82f0916c6f60de0a536f8c466d440c56715bd5").unwrap()
        );
        assert_eq!(
            a.get_storage(Arc::clone(&db), &H256::zero()).unwrap().unwrap(),
            H256::from_low_u64_be(0x1234)
        );
        assert_eq!(a.get_storage(Arc::clone(&db), &H256::from_low_u64_be(1)).unwrap(), None);
    }

    #[test]
    fn state_object_note_code() {
        let mut a = StateObject::new(69u8.into(), 0.into());
        let db = Arc::new(cita_trie::MemoryDB::new(false));
        let a_rlp = {
            a.init_code(vec![0x55, 0x44, 0xffu8]);
            a.commit_code(Arc::clone(&db)).unwrap();
            a.rlp()
        };
        a = StateObject::from_rlp(&a_rlp[..]).unwrap();
        a.read_code(Arc::clone(&db)).unwrap();
        assert_eq!(a.code, vec![0x55, 0x44, 0xffu8]);
    }
}
