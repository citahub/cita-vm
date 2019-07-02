use std::fs;

use ethereum_types::U256;

fn main() {
    let vm = cita_vm::FakeVM::new();

    let tx = cita_vm::Transaction {
        from: vm.account1.clone(),
        to: None,
        value: U256::from(0),
        nonce: U256::from(1),
        gas_limit: 1_000_000,
        gas_price: U256::from(1),
        input: fs::read("./build/riscv_c_simplestorage").unwrap(),
        itype: cita_vm::InterpreterType::RISCV,
    };
    let r = vm.executor.exec(cita_vm::Context::default(), tx).unwrap();
    println!("{:?}", r);
    let contract_address = match r {
        cita_vm::InterpreterResult::Create(_, _, _, a) => a,
        _ => unreachable!(),
    };

    let tx = cita_vm::Transaction {
        from: vm.account1.clone(),
        to: Some(contract_address),
        value: U256::from(0),
        nonce: U256::from(2),
        gas_limit: 1_000_000,
        gas_price: U256::from(1),
        input: cita_vm::riscv::combine_parameters(vec!["set".into(), "everything".into(), "42".into()]),
        itype: cita_vm::InterpreterType::RISCV,
    };
    let r = vm.executor.exec(cita_vm::Context::default(), tx).unwrap();
    println!("{:?}", r);

    let tx = cita_vm::Transaction {
        from: vm.account1.clone(),
        to: Some(contract_address),
        value: U256::from(0),
        nonce: U256::from(3),
        gas_limit: 1_000_000,
        gas_price: U256::from(1),
        input: cita_vm::riscv::combine_parameters(vec!["get".into(), "everything".into()]),
        itype: cita_vm::InterpreterType::RISCV,
    };
    let r = vm.executor.exec(cita_vm::Context::default(), tx).unwrap();
    println!("{:?}", r);
}
