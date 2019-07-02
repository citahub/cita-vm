use std::cell::RefCell;
use std::rc::Rc;

use bytes::Bytes;
use ckb_vm::machine::SupportMachine;

use crate::evm::DataProvider;
use crate::riscv;
use crate::{Context, InterpreterParams, InterpreterResult};

pub struct Interpreter {
    pub context: Context,
    pub iparams: InterpreterParams,
    pub data_provider: Rc<RefCell<DataProvider>>,
}

impl Interpreter {
    pub fn new(context: Context, iparams: InterpreterParams, data_provider: Rc<RefCell<DataProvider>>) -> Self {
        Self {
            context,
            iparams,
            data_provider,
        }
    }

    pub fn run(&mut self) -> Result<InterpreterResult, ckb_vm::Error> {
        let contract_code = Bytes::from(self.iparams.contract.code_data.clone());
        let contract_args: Vec<Bytes> = riscv::cutting_parameters(self.iparams.input.clone())
            .as_slice()
            .iter()
            .map(|e| Bytes::from(e.clone()))
            .collect();

        let code = contract_code.clone();
        let mut args = contract_args.clone();
        args.insert(0, Bytes::from("main"));

        let ret_data = Rc::new(RefCell::new(Vec::new()));
        let core_machine =
            ckb_vm::DefaultCoreMachine::<u64, ckb_vm::FlatMemory<u64>>::new_with_max_cycles(self.iparams.gas_limit);
        let mut machine =
            ckb_vm::DefaultMachineBuilder::<ckb_vm::DefaultCoreMachine<u64, ckb_vm::FlatMemory<u64>>>::new(
                core_machine,
            )
            .instruction_cycle_func(Box::new(riscv::cost_model::instruction_cycles))
            .syscall(Box::new(riscv::SyscallDebug::new("riscv:", std::io::stdout())))
            .syscall(Box::new(riscv::SyscallEnvironment::new(
                self.context.clone(),
                self.iparams.clone(),
                self.data_provider.clone(),
            )))
            .syscall(Box::new(riscv::SyscallRet::new(ret_data.clone())))
            .syscall(Box::new(riscv::SyscallStorage::new(
                self.iparams.address,
                self.data_provider.clone(),
            )))
            .build();

        machine.load_program(&code, &args[..]).unwrap();
        let exitcode = machine.run()?;
        let cycles = machine.cycles();
        if exitcode != 0x00 {
            Ok(InterpreterResult::Revert(
                ret_data.borrow().to_vec(),
                self.iparams.gas_limit - cycles,
            ))
        } else {
            Ok(InterpreterResult::Normal(
                ret_data.borrow().to_vec(),
                self.iparams.gas_limit - cycles,
                vec![],
            ))
        }
    }
}
