extern crate trie_root;
extern crate trie_db;
extern crate hash_db;
extern crate memory_db;
extern crate hex;

use trie_db::{DBValue};

pub mod hasher;
pub mod codec;

use self::codec::RLPNodeCodec;

pub type MemoryDB<H> = memory_db::MemoryDB<H, DBValue>;
pub type RLPTrieDBMut<'a, H> = trie_db::TrieDBMut<'a, H, RLPNodeCodec<H>>;
pub type RLPTrieDB<'a, H> = trie_db::TrieDB<'a, H, RLPNodeCodec<H>>;
pub type RLPSecTrieDBMut<'a, H> = trie_db::SecTrieDBMut<'a, H, RLPNodeCodec<H>>;

#[cfg(test)]
mod tests {
    use super::*;
    use trie_db::TrieMut;
    use super::hasher::{Sha3Hasher};

    #[test]
    fn test_empty_trie_shoule_be_true() {
        let mut m = MemoryDB::<Sha3Hasher>::default();
        let mut root = Default::default();
        let trie = RLPSecTrieDBMut::new(&mut m, &mut root);
        assert!(trie.is_empty(), true)
    }

    #[test]
    fn test_insert_leaft_node() {
        let mut m = MemoryDB::<Sha3Hasher>::default();
        let mut root = Default::default();
        let mut trie = RLPSecTrieDBMut::new(&mut m, &mut root);

        trie.insert("test-key".as_bytes(), "test-value".as_bytes()).unwrap();
        let value = trie.get("test-key".as_bytes()).unwrap().unwrap();
        assert_eq!(value, "test-value".as_bytes())
    }

    #[test]
    fn test_insert_branch_node() {
        let mut m = MemoryDB::<Sha3Hasher>::default();
        let mut root = Default::default();
        let mut trie = RLPSecTrieDBMut::new(&mut m, &mut root);

        trie.insert("test-key1".as_bytes(), "test-vlue".as_bytes()).unwrap();
        trie.insert("test-key2".as_bytes(), "test-value".as_bytes()).unwrap();
        trie.insert("test-key3".as_bytes(), "test-value".as_bytes()).unwrap();
        trie.insert("test-key4".as_bytes(), "test-value".as_bytes()).unwrap();

        trie.root();
    }

    #[test]
    fn test_insert_ext_node() {
        let mut m = MemoryDB::<Sha3Hasher>::default();
        let mut root = Default::default();
        let mut trie = RLPSecTrieDBMut::new(&mut m, &mut root);

        trie.insert("test-key1".as_bytes(), "test-vlue".as_bytes()).unwrap();
        trie.insert("test-key2".as_bytes(), "test-value".as_bytes()).unwrap();
        trie.insert("test-key11".as_bytes(), "test-value".as_bytes()).unwrap();
        trie.insert("test-key12".as_bytes(), "test-value".as_bytes()).unwrap();
        trie.insert("test-key13".as_bytes(), "test-value".as_bytes()).unwrap();

        trie.root();
    }

    #[test]
    fn test_remove_should_be_none() {
        let mut m = MemoryDB::<Sha3Hasher>::default();
        let mut root = Default::default();
        let mut trie = RLPSecTrieDBMut::new(&mut m, &mut root);

        trie.insert("test-key1".as_bytes(), "test-value".as_bytes()).unwrap();
        let old_value = trie.remove("test-key1".as_bytes()).unwrap().unwrap();
        assert_eq!(old_value, "test-value".as_bytes());
        
        let value = trie.get("test-key1".as_bytes()).unwrap().take();
        assert_eq!(value, None)
    }

    #[test]
    fn test_contains_should_be_false() {
        let mut m = MemoryDB::<Sha3Hasher>::default();
        let mut root = Default::default();
        let trie = RLPSecTrieDBMut::new(&mut m, &mut root);

        let contains = trie.contains("test-key1".as_bytes()).unwrap();
        assert_eq!(contains, false)
    }

    #[test]
    fn test_contains_should_be_true() {
        let mut m = MemoryDB::<Sha3Hasher>::default();
        let mut root = Default::default();
        let mut trie = RLPSecTrieDBMut::new(&mut m, &mut root);

        trie.insert("test-key1".as_bytes(), "test-value".as_bytes()).unwrap();
        let contains = trie.contains("test-key1".as_bytes()).unwrap();
        assert_eq!(contains, true)
    }

       #[test]
    fn test_trie_from_existing() {
        let mut m = MemoryDB::<Sha3Hasher>::default();
        let mut new_root = {
            let mut root = Default::default();
            let mut trie = RLPSecTrieDBMut::new(&mut m, &mut root);
            trie.insert("test-key1".as_bytes(), "test-value".as_bytes()).unwrap();
            trie.insert("test-key2".as_bytes(), "test-value".as_bytes()).unwrap();
            trie.insert("test-key11".as_bytes(), "test-value".as_bytes()).unwrap();
            trie.insert("test-key11".as_bytes(), "test-value".as_bytes()).unwrap();
            trie.get("test-key1".as_bytes()).unwrap().unwrap();
            trie.root().clone()
        };

        let _ = RLPSecTrieDBMut::from_existing(&mut m, &mut new_root).unwrap();
    }
}
