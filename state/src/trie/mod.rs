extern crate hash_db;
extern crate hex;
extern crate memory_db;
extern crate trie_db;
extern crate trie_root;

use trie_db::DBValue;

pub mod codec;
pub mod hasher;

use self::codec::RLPNodeCodec;

pub type MemoryDB<H> = memory_db::MemoryDB<H, DBValue>;
pub type RLPTrieDBMut<'a, H> = trie_db::TrieDBMut<'a, H, RLPNodeCodec<H>>;
pub type RLPTrieDB<'a, H> = trie_db::TrieDB<'a, H, RLPNodeCodec<H>>;
pub type RLPSecTrieDBMut<'a, H> = trie_db::SecTrieDBMut<'a, H, RLPNodeCodec<H>>;

#[cfg(test)]
mod tests {
    use super::hasher::Sha3Hasher;
    use super::*;
    use trie_db::TrieMut;

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

        trie.insert(b"test-key", b"test-value").unwrap();
        let value = trie.get(b"test-key").unwrap().unwrap();
        assert_eq!(value.into_vec(), b"test-value")
    }

    #[test]
    fn test_insert_branch_node() {
        let mut m = MemoryDB::<Sha3Hasher>::default();
        let mut root = Default::default();
        let mut trie = RLPSecTrieDBMut::new(&mut m, &mut root);

        trie.insert(b"test-key1", b"test-vlue").unwrap();
        trie.insert(b"test-key2", b"test-value").unwrap();
        trie.insert(b"test-key3", b"test-value").unwrap();
        trie.insert(b"test-key4", b"test-value").unwrap();

        trie.root();
    }

    #[test]
    fn test_insert_ext_node() {
        let mut m = MemoryDB::<Sha3Hasher>::default();
        let mut root = Default::default();
        let mut trie = RLPSecTrieDBMut::new(&mut m, &mut root);

        trie.insert(b"test-key1", b"test-vlue").unwrap();
        trie.insert(b"test-key2", b"test-value").unwrap();
        trie.insert(b"test-key11", b"test-value").unwrap();
        trie.insert(b"test-key12", b"test-value").unwrap();
        trie.insert(b"test-key13", b"test-value").unwrap();

        trie.root();
    }

    #[test]
    fn test_remove_should_be_none() {
        let mut m = MemoryDB::<Sha3Hasher>::default();
        let mut root = Default::default();
        let mut trie = RLPSecTrieDBMut::new(&mut m, &mut root);

        trie.insert(b"test-key1", b"test-value").unwrap();
        let old_value = trie.remove(b"test-key1").unwrap().unwrap();
        assert_eq!(old_value.into_vec(), b"test-value");

        let value = trie.get(b"test-key1").unwrap().take();
        assert_eq!(value, None)
    }

    #[test]
    fn test_contains_should_be_false() {
        let mut m = MemoryDB::<Sha3Hasher>::default();
        let mut root = Default::default();
        let trie = RLPSecTrieDBMut::new(&mut m, &mut root);

        let contains = trie.contains(b"test-key1").unwrap();
        assert_eq!(contains, false)
    }

    #[test]
    fn test_contains_should_be_true() {
        let mut m = MemoryDB::<Sha3Hasher>::default();
        let mut root = Default::default();
        let mut trie = RLPSecTrieDBMut::new(&mut m, &mut root);

        trie.insert(b"test-key1", b"test-value").unwrap();
        let contains = trie.contains(b"test-key1").unwrap();
        assert_eq!(contains, true)
    }

    #[test]
    fn test_trie_from_existing() {
        let mut m = MemoryDB::<Sha3Hasher>::default();
        let mut new_root = {
            let mut root = Default::default();
            let mut trie = RLPSecTrieDBMut::new(&mut m, &mut root);
            trie.insert(b"test-key1", b"test-value").unwrap();
            trie.insert(b"test-key2", b"test-value").unwrap();
            trie.insert(b"test-key11", b"test-value").unwrap();
            trie.insert(b"test-key11", b"test-value").unwrap();
            trie.get(b"test-key1").unwrap().unwrap();
            *trie.root()
        };

        let _ = RLPSecTrieDBMut::from_existing(&mut m, &mut new_root).unwrap();
    }
}
