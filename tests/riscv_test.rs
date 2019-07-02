use std::fs;

use cita_vm;
use ethereum_types::U256;

#[test]
fn test_riscv_exit_0() {
    let vm = cita_vm::FakeVM::new();

    let tx = cita_vm::Transaction {
        from: vm.account1,
        to: None,
        value: U256::from(10),
        nonce: U256::from(1),
        gas_limit: 1000000,
        gas_price: U256::from(1),
        input: fs::read("./build/tests/exit_0").unwrap(),
        itype: cita_vm::InterpreterType::RISCV,
    };
    let r = vm.executor.exec(cita_vm::Context::default(), tx).unwrap();
    let (_, _, _, addr) = match r {
        cita_vm::InterpreterResult::Normal(_, _, _) => unreachable!(),
        cita_vm::InterpreterResult::Revert(_, _) => unreachable!(),
        cita_vm::InterpreterResult::Create(a, b, c, d) => (a, b, c, d),
    };

    let tx = cita_vm::Transaction {
        from: vm.account1,
        to: Some(addr),
        value: U256::from(10),
        nonce: U256::from(2),
        gas_limit: 1000000,
        gas_price: U256::from(1),
        input: vec![],
        itype: cita_vm::InterpreterType::RISCV,
    };
    let r = vm.executor.exec(cita_vm::Context::default(), tx).unwrap();
    match r {
        cita_vm::InterpreterResult::Normal(_, _, _) => {}
        _ => panic!(""),
    }
}

#[test]
fn test_riscv_exit_1() {
    let vm = cita_vm::FakeVM::new();

    let tx = cita_vm::Transaction {
        from: vm.account1,
        to: None,
        value: U256::from(10),
        nonce: U256::from(1),
        gas_limit: 1000000,
        gas_price: U256::from(1),
        input: fs::read("./build/tests/exit_1").unwrap(),
        itype: cita_vm::InterpreterType::RISCV,
    };
    let r = vm.executor.exec(cita_vm::Context::default(), tx).unwrap();
    let (_, _, _, addr) = match r {
        cita_vm::InterpreterResult::Normal(_, _, _) => unreachable!(),
        cita_vm::InterpreterResult::Revert(_, _) => unreachable!(),
        cita_vm::InterpreterResult::Create(a, b, c, d) => (a, b, c, d),
    };

    let tx = cita_vm::Transaction {
        from: vm.account1,
        to: Some(addr),
        value: U256::from(10),
        nonce: U256::from(2),
        gas_limit: 1000000,
        gas_price: U256::from(1),
        input: vec![],
        itype: cita_vm::InterpreterType::RISCV,
    };
    let r = vm.executor.exec(cita_vm::Context::default(), tx).unwrap();
    match r {
        cita_vm::InterpreterResult::Revert(_, _) => {}
        _ => panic!(""),
    }
}
