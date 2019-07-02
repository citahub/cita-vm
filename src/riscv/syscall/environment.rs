//! Environmental Information
use std::cell::RefCell;
use std::rc::Rc;

use ckb_vm::instructions::Register;
use ckb_vm::memory::Memory;
use ethereum_types::U256;

use crate::evm::DataProvider;
use crate::riscv::syscall::common::get_arr;
use crate::riscv::syscall::convention::{
    SYSCODE_ADDRESS, SYSCODE_BALANCE, SYSCODE_BLOCKHASH, SYSCODE_CALLER, SYSCODE_CALLVALUE, SYSCODE_COINBASE,
    SYSCODE_DIFFICULTY, SYSCODE_GASLIMIT, SYSCODE_NUMBER, SYSCODE_ORIGIN, SYSCODE_TIMESTAMP,
};
use crate::{Context, InterpreterParams};

pub struct SyscallEnvironment {
    context: Context,
    iparams: InterpreterParams,
    data: Rc<RefCell<DataProvider>>,
}

impl SyscallEnvironment {
    pub fn new(context: Context, iparams: InterpreterParams, data: Rc<RefCell<DataProvider>>) -> Self {
        Self { context, iparams, data }
    }
}

impl<Mac: ckb_vm::SupportMachine> ckb_vm::Syscalls<Mac> for SyscallEnvironment {
    fn initialize(&mut self, _machine: &mut Mac) -> Result<(), ckb_vm::Error> {
        Ok(())
    }

    fn ecall(&mut self, machine: &mut Mac) -> Result<bool, ckb_vm::Error> {
        let code = &machine.registers()[ckb_vm::registers::A7];
        match code.to_i32() {
            SYSCODE_ADDRESS => {
                let addr = machine.registers()[ckb_vm::registers::A0].to_usize();
                machine.memory_mut().store_bytes(addr, &self.iparams.address)?;
                machine.set_register(ckb_vm::registers::A0, Mac::REG::from_u8(0));
                Ok(true)
            }
            SYSCODE_BALANCE => {
                let addr_addr = machine.registers()[ckb_vm::registers::A0].to_usize();
                let v_addr = machine.registers()[ckb_vm::registers::A1].to_usize();

                let addr_byte = get_arr(machine, addr_addr, 20)?;
                let addr_h160 = ethereum_types::Address::from(&addr_byte[..]);

                let v_u256 = self.data.borrow().get_balance(&addr_h160);
                let mut v_byte = [0x00u8; 32];
                v_u256.to_big_endian(&mut v_byte);
                machine.memory_mut().store_bytes(v_addr, &v_byte)?;
                Ok(true)
            }
            SYSCODE_ORIGIN => {
                let addr = machine.registers()[ckb_vm::registers::A0].to_usize();
                machine.memory_mut().store_bytes(addr, &self.iparams.origin)?;
                machine.set_register(ckb_vm::registers::A0, Mac::REG::from_u8(0));
                Ok(true)
            }
            SYSCODE_CALLER => {
                let addr = machine.registers()[ckb_vm::registers::A0].to_usize();
                machine.memory_mut().store_bytes(addr, &self.iparams.sender)?;
                machine.set_register(ckb_vm::registers::A0, Mac::REG::from_u8(0));
                Ok(true)
            }
            SYSCODE_CALLVALUE => {
                let addr = machine.registers()[ckb_vm::registers::A0].to_usize();
                let mut v_byte = [0x00u8; 32];
                self.iparams.value.to_big_endian(&mut v_byte);
                machine.memory_mut().store_bytes(addr, &v_byte)?;
                machine.set_register(ckb_vm::registers::A0, Mac::REG::from_u8(0));
                Ok(true)
            }
            SYSCODE_BLOCKHASH => {
                let h = machine.registers()[ckb_vm::registers::A0].to_usize();
                let hash_addr = machine.registers()[ckb_vm::registers::A1].to_usize();
                let hash_byte = self.data.borrow().get_block_hash(&U256::from(h)).0;
                machine.memory_mut().store_bytes(hash_addr, &hash_byte)?;
                machine.set_register(ckb_vm::registers::A0, Mac::REG::from_u8(0));
                Ok(true)
            }
            SYSCODE_COINBASE => {
                let addr = machine.registers()[ckb_vm::registers::A0].to_usize();
                machine.memory_mut().store_bytes(addr, &self.context.coinbase)?;
                machine.set_register(ckb_vm::registers::A0, Mac::REG::from_u8(0));
                Ok(true)
            }
            SYSCODE_TIMESTAMP => {
                let addr = machine.registers()[ckb_vm::registers::A0].to_usize();
                let time_byte = self.context.timestamp.to_le_bytes();
                machine.memory_mut().store_bytes(addr, &time_byte)?;
                machine.set_register(ckb_vm::registers::A0, Mac::REG::from_u8(0));
                Ok(true)
            }
            SYSCODE_NUMBER => {
                let addr = machine.registers()[ckb_vm::registers::A0].to_usize();
                let mut v_byte = [0x00u8; 32];
                self.context.number.to_big_endian(&mut v_byte);
                machine.memory_mut().store_bytes(addr, &v_byte)?;
                machine.set_register(ckb_vm::registers::A0, Mac::REG::from_u8(0));
                Ok(true)
            }
            SYSCODE_DIFFICULTY => {
                let addr = machine.registers()[ckb_vm::registers::A0].to_usize();
                let mut v_byte = [0x00u8; 32];
                self.context.difficulty.to_big_endian(&mut v_byte);
                machine.memory_mut().store_bytes(addr, &v_byte)?;
                machine.set_register(ckb_vm::registers::A0, Mac::REG::from_u8(0));
                Ok(true)
            }
            SYSCODE_GASLIMIT => {
                let addr = machine.registers()[ckb_vm::registers::A0].to_usize();
                let gaslimit_byte = self.context.gas_limit.to_le_bytes();
                machine.memory_mut().store_bytes(addr, &gaslimit_byte)?;
                machine.set_register(ckb_vm::registers::A0, Mac::REG::from_u8(0));
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
