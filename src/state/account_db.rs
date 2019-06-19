use cita_trie::DB;
use ethereum_types::Address;
use std::sync::Arc;

use crate::err::Error;

#[derive(Debug)]
pub struct AccountDB<B: DB> {
    address: Address,
    db: Arc<B>,
}

impl<B: DB> AccountDB<B> {
    pub fn new(address: Address, db: Arc<B>) -> Self {
        AccountDB { address, db }
    }
}

impl<B: DB> DB for AccountDB<B> {
    type Error = Error;

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        let concatenated = [&self.address.0[..], &key[..]].concat();
        self.db
            .get(concatenated.as_slice())
            .or_else(|e| Err(Error::DB(format!("{}", e))))
    }

    fn insert(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), Self::Error> {
        let concatenated = [&self.address.0[..], &key[..]].concat();
        self.db
            .insert(concatenated, value)
            .or_else(|e| Err(Error::DB(format!("{}", e))))
    }

    fn contains(&self, key: &[u8]) -> Result<bool, Self::Error> {
        let concatenated = [&self.address.0[..], &key[..]].concat();
        self.db
            .contains(concatenated.as_slice())
            .or_else(|e| Err(Error::DB(format!("{}", e))))
    }

    fn remove(&self, key: &[u8]) -> Result<(), Self::Error> {
        let concatenated = [&self.address.0[..], &key[..]].concat();
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
