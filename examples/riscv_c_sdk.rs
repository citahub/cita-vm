use std::cell::RefCell;
use std::fs;
use std::rc::Rc;

use bytes::Bytes;
use cita_vm;
use ckb_vm;
use ckb_vm::machine::SupportMachine;
use hashbrown::HashMap;

fn main() {
    // Load binary
    let buffer = Bytes::from(fs::read("./build/riscv_c_sdk").unwrap());

    // Initialize ret data
    let ret_data = Rc::new(RefCell::new(Vec::new()));

    // Initialize params
    let mut vm_params = cita_vm::InterpreterParams::default();
    vm_params.address = ethereum_types::Address::from("0x0000000000000000000000000000000000000001");
    vm_params.origin = ethereum_types::Address::from("0x0000000000000000000000000000000000000002");
    vm_params.sender = ethereum_types::Address::from("0x0000000000000000000000000000000000000003");
    vm_params.value = ethereum_types::U256::from(5);

    // Initialize context
    let mut vm_context = cita_vm::Context::default();
    vm_context.number = ethereum_types::U256::from(6);
    vm_context.coinbase = ethereum_types::Address::from("0x0000000000000000000000000000000000000008");
    vm_context.timestamp = 9;
    vm_context.difficulty = ethereum_types::U256::from(0x0a);

    // Initialize storage
    let state = Rc::new(RefCell::new(cita_vm::evm::extmock::DataProviderMock::default()));
    let acc1 = ethereum_types::Address::from("0x0000000000000000000000000000000000000001");
    state.borrow_mut().db.insert(
        acc1,
        cita_vm::evm::extmock::Account {
            balance: ethereum_types::U256::from(10),
            code: vec![],
            nonce: ethereum_types::U256::from(0),
            storage: HashMap::new(),
        },
    );

    let mut machine =
        ckb_vm::DefaultMachineBuilder::<ckb_vm::DefaultCoreMachine<u64, ckb_vm::SparseMemory<u64>>>::default()
            .instruction_cycle_func(Box::new(cita_vm::riscv::instruction_cycles))
            .syscall(Box::new(cita_vm::riscv::SyscallDebug::new("riscv:", std::io::stdout())))
            .syscall(Box::new(cita_vm::riscv::SyscallEnvironment::new(
                vm_context.clone(),
                vm_params.clone(),
                state.clone(),
            )))
            .syscall(Box::new(cita_vm::riscv::SyscallRet::new(ret_data.clone())))
            .syscall(Box::new(cita_vm::riscv::SyscallStorage::new(
                vm_params.address.clone(),
                state.clone(),
            )))
            .build();

    machine.load_program(&buffer, &vec!["riscv_c_main".into()]).unwrap();
    let result = machine.run().unwrap();
    println!(
        "exit={:#02x} ret={:?} cycles={:?}",
        result,
        ret_data.borrow(),
        machine.cycles()
    );
}
