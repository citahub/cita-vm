use cita_trie::db::DB;
use ethereum_types::Address;

#[derive(Debug)]
pub struct AccountDB<'a, B: cita_trie::db::DB> {
    address: Address,
    db: &'a mut B,
}

impl<'a, B: cita_trie::db::DB> AccountDB<'a, B> {
    pub fn new(address: Address, db: &'a mut B) -> Self {
        AccountDB { address, db }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AccountDBError {
    Unknown,
}
impl std::error::Error for AccountDBError {}
impl std::fmt::Display for AccountDBError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AccountDBError::Unknown => write!(f, "Unknown"),
        }
    }
}

impl<'a, B: cita_trie::db::DB> DB for AccountDB<'a, B> {
    type Error = AccountDBError;

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        let concatenated = [&self.address.0[..], &key[..]].concat();
        self.db.get(concatenated.as_slice()).or(Err(AccountDBError::Unknown))
    }

    fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<(), Self::Error> {
        let concatenated = [&self.address.0[..], &key[..]].concat();
        self.db
            .insert(concatenated.as_slice(), value)
            .or(Err(AccountDBError::Unknown))
    }

    fn contains(&self, key: &[u8]) -> Result<bool, Self::Error> {
        let concatenated = [&self.address.0[..], &key[..]].concat();
        self.db
            .contains(concatenated.as_slice())
            .or(Err(AccountDBError::Unknown))
    }

    fn remove(&mut self, key: &[u8]) -> Result<(), Self::Error> {
        let concatenated = [&self.address.0[..], &key[..]].concat();
        self.db.remove(concatenated.as_slice()).or(Err(AccountDBError::Unknown))
    }
}

#[cfg(test)]
mod test_account_db {
    use super::*;
    use cita_trie::db::MemoryDB;

    #[test]
    fn test_accdb_get() {
        let mut memdb = MemoryDB::new();
        let mut accdb = AccountDB::new(Address::zero(), &mut memdb);
        accdb.insert(b"test-key", b"test-value").unwrap();
        let v = accdb.get(b"test-key").unwrap().unwrap();
        assert_eq!(v, b"test-value")
    }

    #[test]
    fn test_accdb_contains() {
        let mut memdb = MemoryDB::new();
        let mut accdb = AccountDB::new(Address::zero(), &mut memdb);
        accdb.insert(b"test", b"test").unwrap();
        let contains = accdb.contains(b"test").unwrap();
        assert_eq!(contains, true)
    }

    #[test]
    fn test_accdb_remove() {
        let mut memdb = MemoryDB::new();
        let mut accdb = AccountDB::new(Address::zero(), &mut memdb);
        accdb.insert(b"test", b"test").unwrap();
        accdb.remove(b"test").unwrap();
        let contains = accdb.contains(b"test").unwrap();
        assert_eq!(contains, false)
    }
}
