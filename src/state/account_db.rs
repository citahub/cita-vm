use std::sync::Arc;

use crate::common::hash::{summary, RLP_NULL};
use cita_trie::DB;
use ethereum_types::{Address, H256};

use crate::state::err::Error;

static NULL_RLP_STATIC: [u8; 1] = [0x80; 1];

fn combine_key<'a>(addr_hash: &[u8], key: &'a [u8]) -> Vec<u8> {
    let mut dst = key.clone().to_vec();
    {
        for (k, a) in dst[12..].iter_mut().zip(&addr_hash[12..]) {
            *k ^= *a
        }
    }
    dst
}

#[derive(Debug)]
pub struct AccountDB<B: DB> {
    /// address means address's hash
    address_hash: H256,
    db: Arc<B>,
}

impl<B: DB> AccountDB<B> {
    pub fn new(address: Address, db: Arc<B>) -> Self {
        let address_hash = summary(&address[..]).as_slice().into();
        AccountDB { address_hash, db }
    }
}

impl<B: DB> DB for AccountDB<B> {
    type Error = Error;

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        if H256::from(key) == RLP_NULL {
            return Ok(Some(NULL_RLP_STATIC.to_vec()));
        }

        let concatenated = combine_key(&self.address_hash.0[..], &key[..]);
        self.db
            .get(concatenated.as_slice())
            .or_else(|e| Err(Error::DB(format!("{}", e))))
    }

    fn insert(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), Self::Error> {
        if H256::from(key.as_slice()) == RLP_NULL {
            return Ok(());
        }
        let concatenated = combine_key(&self.address_hash.0[..], &key[..]);
        self.db
            .insert(concatenated, value)
            .or_else(|e| Err(Error::DB(format!("{}", e))))
    }

    fn contains(&self, key: &[u8]) -> Result<bool, Self::Error> {
        if H256::from(key) == RLP_NULL {
            return Ok(true);
        }
        let concatenated = combine_key(&self.address_hash.0[..], &key[..]);
        self.db
            .contains(concatenated.as_slice())
            .or_else(|e| Err(Error::DB(format!("{}", e))))
    }

    fn remove(&self, key: &[u8]) -> Result<(), Self::Error> {
        if H256::from(key) == RLP_NULL {
            return Ok(());
        }
        let concatenated = combine_key(&self.address_hash.0[..], &key[..]);
        self.db
            .remove(concatenated.as_slice())
            .or_else(|e| Err(Error::DB(format!("{}", e))))
    }

    fn flush(&self) -> Result<(), Self::Error> {
        self.db.flush().or_else(|e| Err(Error::DB(format!("{}", e))))
    }
}

#[cfg(test)]
mod test_account_db {
    use super::*;
    use cita_trie::MemoryDB;

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
