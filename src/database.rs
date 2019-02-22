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
    lru: lru_cache::LruCache<Vec<u8>, Vec<u8>>,
}

impl RocksDB {
    fn new(path: &str) -> Result<Self, DBError> {
        let db = parity_rocksdb::DB::open_default(path)?;
        return Ok(RocksDB {
            raw: db,
            lru: lru_cache::LruCache::new(65536),
        });
    }
}

impl std::fmt::Debug for RocksDB {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "RocksDB()")
    }
}

impl cita_trie::db::DB for RocksDB {
    type Error = DBError;

    fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, DBError> {
        if let Some(data) = self.lru.get_mut(&key.to_vec()) {
            return Ok(Some(data.to_vec()));
        }

        let a = self.raw.get(key)?;
        match a {
            Some(data) => {
                self.lru.insert(key.to_vec(), data.to_vec());
                return Ok(Some(data.to_vec()));
            }
            None => return Ok(None),
        };
    }

    fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<(), DBError> {
        self.raw.put(key, value)?;
        self.lru.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn contains(&mut self, key: &[u8]) -> Result<bool, DBError> {
        Ok(self.get(key)?.is_some())
    }

    fn remove(&mut self, key: &[u8]) -> Result<(), DBError> {
        self.raw.delete(key)?;
        self.lru.remove(&key.to_vec());
        Ok(())
    }
}
