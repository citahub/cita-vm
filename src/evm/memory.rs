use crate::evm::common;

#[derive(Default)]
pub struct Memory {
    store: Vec<u8>,
}

impl Memory {
    pub fn set(&mut self, offset: usize, val: &[u8]) {
        if offset + val.len() > self.store.len() {
            panic!("invalid memory: store empty")
        }

        self.store[offset..offset + val.len()].copy_from_slice(val)
    }

    pub fn get(&self, offset: usize, size: usize) -> &[u8] {
        if size == 0 {
            return &[];
        }
        &self.store[offset..offset + size]
    }

    pub fn resize(&mut self, size: usize) {
        self.store.resize(size, u8::default())
    }

    pub fn expand(&mut self, size: usize) {
        if size > self.len() {
            self.resize(common::to_word_size(size as u64) as usize * 32)
        }
    }

    pub fn len(&self) -> usize {
        self.store.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_set_get() {
        let mut mem = Memory::default();
        mem.resize(128);
        assert_eq!(mem.len(), 128);
        let r = mem.get(8, 2);
        assert_eq!(r[0], 0x00);
        assert_eq!(r[1], 0x00);
        mem.set(8, &[0x01, 0x02]);
        let r = mem.get(8, 2);
        assert_eq!(r[0], 0x01);
        assert_eq!(r[1], 0x02);
    }

    #[test]
    fn test_memory_resize() {
        let mut mem = Memory::default();
        mem.resize(128);
        let r: Vec<u8> = (0..128).map(|_| 0xFF).collect();
        mem.set(0, &r);

        mem.resize(256);
        assert_eq!(mem.len(), 256);
        assert_eq!(mem.get(128, 1)[0], 0x00);

        mem.resize(64);
        assert_eq!(mem.len(), 64);
        assert_eq!(mem.get(32, 1)[0], 0xFF);
    }

    #[test]
    fn test_memory_set8() {
        let mut mem = Memory::default();
        mem.resize(8);
        let r: Vec<u8> = (0..8).map(|_| 0xFF).collect();
        mem.set(0, &r);
        let r = mem.get(0, 8);
        assert_eq!(0xFF, r[0]);
        assert_eq!(0xFF, r[7]);
    }
}
