use parity_rocksdb::rocksdb::Writable;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DBError {
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
        }
    }
}

impl std::error::Error for DBError {}

pub struct RocksDB {
    raw: parity_rocksdb::DB,
    lru: std::sync::Mutex<std::cell::RefCell<lru_cache::LruCache<Vec<u8>, Vec<u8>>>>,
}

impl RocksDB {
    pub fn new(path: &str) -> Result<Self, DBError> {
        let db = parity_rocksdb::DB::open_default(path)?;
        return Ok(RocksDB {
            raw: db,
            lru: std::sync::Mutex::new(std::cell::RefCell::new(lru_cache::LruCache::new(65536))),
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

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, DBError> {
        let cache = self.lru.lock().unwrap();
        if let Some(data) = cache.borrow_mut().get_mut(&key.to_vec()) {
            return Ok(Some(data.to_vec()));
        }

        let a = self.raw.get(key)?;
        match a {
            Some(data) => {
                cache.borrow_mut().insert(key.to_vec(), data.to_vec());
                return Ok(Some(data.to_vec()));
            }
            None => return Ok(None),
        };
    }

    fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<(), DBError> {
        let cache = self.lru.lock().unwrap();
        self.raw.put(key, value)?;
        cache.borrow_mut().insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn contains(&self, key: &[u8]) -> Result<bool, DBError> {
        Ok(self.get(key)?.is_some())
    }

    fn remove(&mut self, key: &[u8]) -> Result<(), DBError> {
        let cache = self.lru.lock().unwrap();
        self.raw.delete(key)?;
        cache.borrow_mut().remove(&key.to_vec());
        Ok(())
    }
}
