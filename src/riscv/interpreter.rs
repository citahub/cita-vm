use std::cell::RefCell;
use std::io;
use std::rc::Rc;

use bytes::Bytes;
use ckb_vm::machine::asm::{AsmCoreMachine, AsmMachine};
use ckb_vm::{DefaultMachineBuilder, SupportMachine};

use crate::evm::DataProvider;
use crate::riscv;
use crate::{Context, InterpreterParams, InterpreterResult};

#[derive(Clone, Debug)]
pub enum MachineType {
    NativeRust,
    Asm,
}

#[derive(Clone, Debug)]
pub struct InterpreterConf {
    pub machine_type: MachineType,
    pub print_debug: bool,
}

impl Default for InterpreterConf {
    fn default() -> Self {
        InterpreterConf {
            machine_type: MachineType::Asm,
            print_debug: true,
        }
    }
}

pub struct Interpreter {
    pub context: Context,
    pub iparams: InterpreterParams,
    pub data_provider: Rc<RefCell<DataProvider>>,
    pub cfg: InterpreterConf,
}

impl Interpreter {
    pub fn new(
        context: Context,
        cfg: InterpreterConf,
        iparams: InterpreterParams,
        data_provider: Rc<RefCell<DataProvider>>,
    ) -> Self {
        Self {
            context,
            iparams,
            data_provider,
            cfg,
        }
    }

    pub fn run(&mut self) -> Result<InterpreterResult, ckb_vm::Error> {
        let output: Box<dyn io::Write> = if self.cfg.print_debug {
            Box::new(io::stdout())
        } else {
            Box::new(io::sink())
        };
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
        let (exitcode, cycles) = match self.cfg.machine_type {
            MachineType::NativeRust => {
                let core_machine = ckb_vm::DefaultCoreMachine::<u64, ckb_vm::FlatMemory<u64>>::new_with_max_cycles(
                    self.iparams.gas_limit,
                );
                let mut machine = ckb_vm::DefaultMachineBuilder::<
                    ckb_vm::DefaultCoreMachine<u64, ckb_vm::FlatMemory<u64>>,
                >::new(core_machine)
                .instruction_cycle_func(Box::new(riscv::cost_model::instruction_cycles))
                .syscall(Box::new(riscv::SyscallDebug::new("contract.log", output)))
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
                (exitcode, cycles)
            }
            MachineType::Asm => {
                let core_machine = AsmCoreMachine::new_with_max_cycles(self.iparams.gas_limit);
                let machine = DefaultMachineBuilder::<Box<AsmCoreMachine>>::new(core_machine)
                    .instruction_cycle_func(Box::new(riscv::cost_model::instruction_cycles))
                    .syscall(Box::new(riscv::SyscallDebug::new("contract.log", output)))
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
                let mut machine = AsmMachine::new(machine);
                machine.load_program(&code, &args[..]).unwrap();
                let exitcode = machine.run()?;
                let cycles = machine.machine.cycles();
                (exitcode, cycles)
            }
        };

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
