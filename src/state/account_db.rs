use crate::common::hash::{summary, RLP_NULL};
use cita_trie::CDB;
use ethereum_types::{Address, H256};
use std::io::Error;
use std::sync::Arc;

static NULL_RLP_STATIC: [u8; 1] = [0x80; 1];

fn combine_key(addr_hash: &[u8], key: &[u8]) -> Vec<u8> {
    let mut dst = key.to_owned().to_vec();
    {
        for (k, a) in dst[12..].iter_mut().zip(&addr_hash[12..]) {
            *k ^= *a
        }
    }
    dst
}

#[derive(Debug)]
pub struct AccountDB<B: CDB> {
    /// address means address's hash
    address_hash: H256,
    db: Arc<B>,
}

impl<B: CDB> AccountDB<B> {
    pub fn new(address: Address, db: Arc<B>) -> Self {
        let address_hash = H256::from_slice(summary(&address[..]).as_slice());
        AccountDB { address_hash, db }
    }
}

impl<B: CDB> cita_trie::DB for AccountDB<B> {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        if H256::from_slice(key) == RLP_NULL {
            return Ok(Some(NULL_RLP_STATIC.to_vec()));
        }

        let concatenated = combine_key(&self.address_hash.0[..], key);
        self.db.get(concatenated.as_slice())
    }

    fn insert(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), Error> {
        if H256::from_slice(key.as_slice()) == RLP_NULL {
            return Ok(());
        }
        let concatenated = combine_key(&self.address_hash.0[..], &key[..]);
        self.db.insert(concatenated, value)
    }

    fn contains(&self, key: &[u8]) -> Result<bool, Error> {
        if H256::from_slice(key) == RLP_NULL {
            return Ok(true);
        }
        let concatenated = combine_key(&self.address_hash.0[..], key);
        self.db.contains(concatenated.as_slice())
    }

    fn remove(&self, key: &[u8]) -> Result<(), Error> {
        if H256::from_slice(key) == RLP_NULL {
            return Ok(());
        }
        let concatenated = combine_key(&self.address_hash.0[..], key);
        self.db.remove(concatenated.as_slice())
    }

    fn flush(&self) -> Result<(), Error> {
        self.db.flush()
    }
}

#[cfg(test)]
mod test_account_db {
    use super::*;
    use cita_trie::MemoryDB;
    use cita_trie::DB;

    #[test]
    fn test_accdb_get() {
        let memdb = Arc::new(MemoryDB::new(false));
        let accdb = AccountDB::new(Address::zero(), memdb);
        accdb.insert(b"test-key".to_vec(), b"test-value".to_vec()).unwrap();
        let v = accdb.get(b"test-key").unwrap().unwrap();
        assert_eq!(v, b"test-value")
    }

    #[test]
    fn test_accdb_contains() {
        let memdb = Arc::new(MemoryDB::new(false));
        let accdb = AccountDB::new(Address::zero(), memdb);
        accdb.insert(b"test".to_vec(), b"test".to_vec()).unwrap();
        let contains = accdb.contains(b"test").unwrap();
        assert_eq!(contains, true)
    }

    #[test]
    fn test_accdb_remove() {
        let memdb = Arc::new(MemoryDB::new(true));
        let accdb = AccountDB::new(Address::zero(), memdb);
        accdb.insert(b"test".to_vec(), b"test".to_vec()).unwrap();
        accdb.remove(b"test").unwrap();
        let contains = accdb.contains(b"test").unwrap();
        assert_eq!(contains, false)
    }
}
