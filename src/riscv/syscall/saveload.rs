// Provides the ability to interact with state storage.
use std::cell::RefCell;
use std::cmp;
use std::rc::Rc;

use ckb_vm::instructions::Register;
use ckb_vm::Memory;
use ethereum_types::{Address, H256, U256};

use crate::common::hash;
use crate::evm::DataProvider;
use crate::riscv::syscall::common::get_arr;
use crate::riscv::syscall::convention::{SYSCODE_LOAD, SYSCODE_SAVE};

// Convert k/v of any length to H256 storage.
//
// Give an example shows below:
//
// k = {0x00, 0x01, 0x02}, len=3
// v = {0x00, 0x01, 0x02, ... , 0x39}; len=40
//
// let k_hash = keccek256(k); // f84a97f1f0a956e738abd85c2ea5026f8874e3ec09c8f12159dfeeaab2b156
// let v_size = v.len();      // 0x28
//
// slot0: 0xf84a97f...56 = 0x........0028 // store the size of v
// slot1: 0xf84a97f...57 = 0x000102..3031 // store the first 32 bytes of v
// slot2: 0xf84a97f...58 = 0x32...39...00 // store the rest 8 bytes of v, and fill in with 0x00.
fn gen_mapped(k: &[u8], v: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut r: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();

    let k_hash = hash::summary(k).to_vec();
    let k_u256 = U256::from(&k_hash[..]);

    let v_size = v.len();
    let v_size_u256 = U256::from(v_size);
    let mut v_size_byte = vec![0u8; 32];
    v_size_u256.to_big_endian(&mut v_size_byte);

    r.push((k_hash.clone(), v_size_byte.clone()));

    let mut i = k_u256.saturating_add(U256::one());
    let mut a = vec![];
    for e in v {
        a.push(*e);
        if a.len() == 32 {
            let mut i_hash = vec![0u8; 32];
            i.to_big_endian(&mut i_hash);

            r.push((i_hash, a.clone()));
            i = i.saturating_add(U256::one());
            a = vec![];
        }
    }
    if v_size % 32 != 0 {
        let mut i_hash = vec![0u8; 32];
        i.to_big_endian(&mut i_hash);
        for _ in 0..(32 - a.len()) {
            a.push(0x00);
        }
        r.push((i_hash, a.clone()));
    }
    r
}

pub struct SyscallStorage {
    address: Address,
    data: Rc<RefCell<DataProvider>>,
}

impl SyscallStorage {
    pub fn new(address: Address, data: Rc<RefCell<DataProvider>>) -> Self {
        Self { address, data }
    }
}

impl<Mac: ckb_vm::SupportMachine> ckb_vm::Syscalls<Mac> for SyscallStorage {
    fn initialize(&mut self, _machine: &mut Mac) -> Result<(), ckb_vm::Error> {
        Ok(())
    }

    fn ecall(&mut self, machine: &mut Mac) -> Result<bool, ckb_vm::Error> {
        let code = &machine.registers()[ckb_vm::registers::A7];

        // Set storage
        if code.to_i32() == SYSCODE_SAVE {
            let k_addr = machine.registers()[ckb_vm::registers::A0].to_usize();
            let k_size = machine.registers()[ckb_vm::registers::A1].to_usize();
            let v_addr = machine.registers()[ckb_vm::registers::A2].to_usize();
            let v_size = machine.registers()[ckb_vm::registers::A3].to_usize();
            let k = get_arr(machine, k_addr, k_size)?;
            let v = get_arr(machine, v_addr, v_size)?;

            for (rk, rv) in &gen_mapped(&k[..], &v[..]) {
                self.data
                    .borrow_mut()
                    .set_storage(&self.address, H256::from(&rk[..]), H256::from(&rv[..]));
            }

            machine.set_register(ckb_vm::registers::A0, Mac::REG::from_u8(0));
            return Ok(true);
        }

        // Get storage
        if code.to_i32() == SYSCODE_LOAD {
            let k_addr = machine.registers()[ckb_vm::registers::A0].to_usize();
            let k_size = machine.registers()[ckb_vm::registers::A1].to_usize();
            let v_addr = machine.registers()[ckb_vm::registers::A2].to_usize();
            let v_size = machine.registers()[ckb_vm::registers::A3].to_usize();
            let r_addr = machine.registers()[ckb_vm::registers::A4].to_usize();
            let k = get_arr(machine, k_addr, k_size)?;

            let k_hash = hash::summary(&k[..]).to_vec();
            let k_u256 = U256::from(&k_hash[..]);
            let size_h256 = self.data.borrow().get_storage(&self.address, &H256::from(&k_hash[..]));
            let size_u256 = U256::from(size_h256);
            let mut size_u64 = size_u256.as_u64();

            let mut i = k_u256.saturating_add(U256::one());
            let mut r: Vec<u8> = Vec::new();

            loop {
                let v_h256 = self.data.borrow().get_storage(&self.address, &H256::from(i));
                if size_u64 < 32 {
                    for e in &v_h256.0[0..size_u64 as usize] {
                        r.push(*e);
                    }
                    break;
                }
                size_u64 -= 32;
                for e in &v_h256.0 {
                    r.push(*e);
                }
                i = i.saturating_add(U256::one());
            }

            r.resize(v_size, 0u8);

            machine.memory_mut().store_bytes(v_addr, &r)?;
            machine
                .memory_mut()
                .store_bytes(r_addr, &cmp::min(size_u256.as_u64() as usize, v_size).to_le_bytes()[..])?;
            machine.set_register(ckb_vm::registers::A0, Mac::REG::from_u8(0));
            return Ok(true);
        }
        Ok(false)
    }
}
