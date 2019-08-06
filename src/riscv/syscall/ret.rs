// Since ckb-vm can only return 0 or 1 as exit code, We must find another way to return string, u64...
use std::cell::RefCell;
use std::rc::Rc;

use ckb_vm::instructions::Register;

use crate::riscv::syscall::common::get_arr;
use crate::riscv::syscall::convention::SYSCODE_RET;

pub struct SyscallRet {
    data: Rc<RefCell<Vec<u8>>>,
}

impl SyscallRet {
    pub fn new(data: Rc<RefCell<Vec<u8>>>) -> Self {
        Self { data }
    }
}

impl<Mac: ckb_vm::SupportMachine> ckb_vm::Syscalls<Mac> for SyscallRet {
    fn initialize(&mut self, _machine: &mut Mac) -> Result<(), ckb_vm::Error> {
        Ok(())
    }

    fn ecall(&mut self, machine: &mut Mac) -> Result<bool, ckb_vm::Error> {
        let code = &machine.registers()[ckb_vm::registers::A7];
        if code.to_i32() != SYSCODE_RET {
            return Ok(false);
        }
        let addr = machine.registers()[ckb_vm::registers::A0].to_usize();
        let size = machine.registers()[ckb_vm::registers::A1].to_usize();
        let buffer = get_arr(machine, addr, size)?;
        self.data.borrow_mut().clear();
        self.data.borrow_mut().extend_from_slice(&buffer[..]);
        machine.set_register(ckb_vm::registers::A0, Mac::REG::from_u8(0));
        Ok(true)
    }
}
