extern crate cita_evm;

use cita_evm::interpreter::memory::*;

#[test]
fn test_memory_set_get() {
    let mut mem = Memory::new();
    mem.resize(128);
    assert_eq!(mem.len(), 128);
    let r = mem.get(8, 2);
    assert_eq!(r[0], 0x00);
    assert_eq!(r[1], 0x00);
    mem.set(8, &vec![0x01, 0x02]);
    let r = mem.get(8, 2);
    assert_eq!(r[0], 0x01);
    assert_eq!(r[1], 0x02);
}

#[test]
fn test_memory_resize() {
    let mut mem = Memory::new();
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
