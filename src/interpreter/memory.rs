pub struct Memory {
    store: Vec<u8>,
}

impl Memory {
    pub fn new() -> Memory {
        Memory { store: vec![] }
    }

    pub fn set(&mut self, offset: usize, val: &[u8]) {
        if offset + 32 > self.store.len() {
            panic!("invalid memory: store empty")
        }

        self.store[offset..offset + val.len()].copy_from_slice(val)
    }

    pub fn get(&self, offset: usize, size: usize) -> &[u8] {
        &self.store[offset..offset + size]
    }

    pub fn resize(&mut self, size: usize) {
        self.store.resize_default(size)
    }

    pub fn len(&self) -> usize {
        self.store.len()
    }

    pub fn data(&self) -> &[u8] {
        &self.store
    }
}
