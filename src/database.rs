use cita_trie::trie::Trie;
use cita_trie::db::DB;
use parity_rocksdb::rocksdb::Writable;

#[derive(Clone, Debug, PartialEq, Eq)]
enum DBError {
    Str(String),
}

impl From<String> for DBError {
    fn from(error: String) -> Self {
        DBError::Str(error)
    }
}

impl std::fmt::Display for DBError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DBError::Str(data) => write!(f, "{:?}", data),
            _ => unimplemented!(),
        }
    }
}

impl std::error::Error for DBError {}

struct RocksDB {
    raw: parity_rocksdb::DB,
}

impl RocksDB {
    fn new(path: &str) -> Result<Self, DBError> {
        let db = parity_rocksdb::DB::open_default(path)?;
        return Ok(RocksDB {
            raw: db,
        })
    }
}

impl std::fmt::Debug for RocksDB {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "RocksDB()")
    }
}

// TODO: Global cache?
impl cita_trie::db::DB for RocksDB {
    type Error = DBError;

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, DBError> {
        let a = self.raw.get(key)?;
        let b = match a {
            Some(data) => Some(data.to_vec()),
            None => None,
        };
        Ok(b)
    }

    fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<(), DBError> {
        self.raw.put(key, value)?;
        Ok(())
    }

    fn contains(&self, key: &[u8]) -> Result<bool, DBError> {
        let r =self.raw.get(key)?;
        Ok(r.is_some())
    }

    fn remove(&mut self, key: &[u8]) -> Result<(), DBError> {
        self.raw.delete(key)?;
        Ok(())
    }
}
