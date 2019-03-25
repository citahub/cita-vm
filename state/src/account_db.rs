use super::err::Error;
use cita_trie::db::DB;
use ethereum_types::Address;

#[derive(Debug)]
pub struct AccountDB<B: DB> {
    address: Address,
    db: B,
}

impl<B: DB> AccountDB<B> {
    pub fn new(address: Address, db: B) -> Self {
        AccountDB { address, db }
    }
}

impl<B: DB> Clone for AccountDB<B> {
    fn clone(&self) -> Self {
        AccountDB {
            address: self.address,
            db: self.db.clone(),
        }
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

    fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<(), Self::Error> {
        let concatenated = [&self.address.0[..], &key[..]].concat();
        self.db
            .insert(concatenated.as_slice(), value)
            .or_else(|e| Err(Error::DB(format!("{}", e))))
    }

    fn contains(&self, key: &[u8]) -> Result<bool, Self::Error> {
        let concatenated = [&self.address.0[..], &key[..]].concat();
        self.db
            .contains(concatenated.as_slice())
            .or_else(|e| Err(Error::DB(format!("{}", e))))
    }

    fn remove(&mut self, key: &[u8]) -> Result<(), Self::Error> {
        let concatenated = [&self.address.0[..], &key[..]].concat();
        self.db
            .remove(concatenated.as_slice())
            .or_else(|e| Err(Error::DB(format!("{}", e))))
    }
}

#[cfg(test)]
mod test_account_db {
    use super::*;
    use cita_trie::db::MemoryDB;

    #[test]
    fn test_accdb_get() {
        let mut memdb = MemoryDB::new();
        let mut accdb = AccountDB::new(Address::zero(), memdb);
        accdb.insert(b"test-key", b"test-value").unwrap();
        let v = accdb.get(b"test-key").unwrap().unwrap();
        assert_eq!(v, b"test-value")
    }

    #[test]
    fn test_accdb_contains() {
        let mut memdb = MemoryDB::new();
        let mut accdb = AccountDB::new(Address::zero(), memdb);
        accdb.insert(b"test", b"test").unwrap();
        let contains = accdb.contains(b"test").unwrap();
        assert_eq!(contains, true)
    }

    #[test]
    fn test_accdb_remove() {
        let mut memdb = MemoryDB::new();
        let mut accdb = AccountDB::new(Address::zero(), memdb);
        accdb.insert(b"test", b"test").unwrap();
        accdb.remove(b"test").unwrap();
        let contains = accdb.contains(b"test").unwrap();
        assert_eq!(contains, false)
    }
}
