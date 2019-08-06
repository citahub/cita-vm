use std::fs;

use cita_vm;
use ethereum_types::U256;

fn main() {
    let vm = cita_vm::FakeVM::new();

    let tx = cita_vm::Transaction {
        from: vm.account1,
        to: None,
        value: U256::from(10),
        nonce: U256::from(1),
        gas_limit: 1_000_000,
        gas_price: U256::from(1),
        input: fs::read("./build/riscv_c_fibonacci").unwrap(),
        itype: cita_vm::InterpreterType::RISCV,
    };
    let r = vm.executor.exec(cita_vm::Context::default(), tx).unwrap();
    println!("{:?}", r);
    let (_, _, _, addr) = match r {
        cita_vm::InterpreterResult::Normal(_, _, _) => unreachable!(),
        cita_vm::InterpreterResult::Revert(_, _) => unreachable!(),
        cita_vm::InterpreterResult::Create(r0, r1, r2, r3) => (r0, r1, r2, r3),
    };

    let tx = cita_vm::Transaction {
        from: vm.account1,
        to: Some(addr),
        value: U256::from(10),
        nonce: U256::from(2),
        gas_limit: 1_000_000,
        gas_price: U256::from(1),
        input: cita_vm::riscv::combine_parameters(vec!["10".into()]),
        itype: cita_vm::InterpreterType::RISCV,
    };
    let r = vm.executor.exec(cita_vm::Context::default(), tx).unwrap();
    println!("{:?}", r);
}
