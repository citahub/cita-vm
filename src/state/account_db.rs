use std::cell::RefCell;
use std::rc::Rc;

use cita_trie::DB;
use ethereum_types::Address;

use crate::state::err::Error;

#[derive(Debug)]
pub struct AccountDB<B: DB> {
    address: Address,
    db: Rc<RefCell<B>>,
}

impl<B: DB> AccountDB<B> {
    pub fn new(address: Address, db: Rc<RefCell<B>>) -> Self {
        AccountDB { address, db }
    }
}

impl<B: DB> DB for AccountDB<B> {
    type Error = Error;

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        let concatenated = [&self.address.0[..], &key[..]].concat();
        self.db
            .borrow()
            .get(concatenated.as_slice())
            .or_else(|e| Err(Error::DB(format!("{}", e))))
    }

    fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<(), Self::Error> {
        let concatenated = [&self.address.0[..], &key[..]].concat();
        self.db
            .borrow_mut()
            .insert(concatenated, value)
            .or_else(|e| Err(Error::DB(format!("{}", e))))
    }

    fn contains(&self, key: &[u8]) -> Result<bool, Self::Error> {
        let concatenated = [&self.address.0[..], &key[..]].concat();
        self.db
            .borrow()
            .contains(concatenated.as_slice())
            .or_else(|e| Err(Error::DB(format!("{}", e))))
    }

    fn remove(&mut self, key: &[u8]) -> Result<(), Self::Error> {
        let concatenated = [&self.address.0[..], &key[..]].concat();
        self.db
            .borrow_mut()
            .remove(concatenated.as_slice())
            .or_else(|e| Err(Error::DB(format!("{}", e))))
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.db
            .borrow_mut()
            .flush()
            .or_else(|e| Err(Error::DB(format!("{}", e))))
    }
}

#[cfg(test)]
mod test_account_db {
    use super::*;
    use cita_trie::MemoryDB;

    #[test]
    fn test_accdb_get() {
        let memdb = Rc::new(RefCell::new(MemoryDB::new(false)));
        let accdb = AccountDB::new(Address::zero(), memdb);
        accdb.insert(b"test-key".to_vec(), b"test-value".to_vec()).unwrap();
        let v = accdb.get(b"test-key").unwrap().unwrap();
        assert_eq!(v, b"test-value")
    }

    #[test]
    fn test_accdb_contains() {
        let memdb = Rc::new(RefCell::new(MemoryDB::new(false)));
        let accdb = AccountDB::new(Address::zero(), memdb);
        accdb.insert(b"test".to_vec(), b"test".to_vec()).unwrap();
        let contains = accdb.contains(b"test").unwrap();
        assert_eq!(contains, true)
    }

    #[test]
    fn test_accdb_remove() {
        let memdb = Rc::new(RefCell::new(MemoryDB::new(false)));
        let accdb = AccountDB::new(Address::zero(), memdb);
        accdb.insert(b"test".to_vec(), b"test".to_vec()).unwrap();
        accdb.remove(b"test").unwrap();
        let contains = accdb.contains(b"test").unwrap();
        assert_eq!(contains, false)
    }
}
