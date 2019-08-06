use ethereum_types::U256;

use cita_vm::InterpreterType;

fn main() {
    env_logger::init();
    let vm = cita_vm::FakeVM::new();
    let code = "6080604052600436106049576000357c0100000000000000000000000000000\
                000000000000000000000000000900463ffffffff16806360fe47b114604e57\
                80636d4ce63c146078575b600080fd5b348015605957600080fd5b506076600\
                4803603810190808035906020019092919050505060a0565b005b3480156083\
                57600080fd5b50608a60aa565b6040518082815260200191505060405180910\
                390f35b8060008190555050565b600080549050905600a165627a7a72305820\
                99c66a25d59f0aa78f7ebc40748fa1d1fbc335d8d780f284841b30e0365acd9\
                60029";
    vm.executor
        .state_provider
        .borrow_mut()
        .set_code(&vm.account2, hex::decode(code).unwrap())
        .unwrap();
    let tx = cita_vm::Transaction {
        from: vm.account1,
        to: Some(vm.account2),
        value: U256::from(0),
        nonce: U256::from(1),
        gas_limit: 80000,
        gas_price: U256::from(1),
        input: hex::decode("60fe47b1000000000000000000000000000000000000000000000000000000000000002a").unwrap(),
        itype: InterpreterType::EVM,
    };
    let r = vm.executor.exec(cita_vm::Context::default(), tx).unwrap();
    println!("return={:?}", r);

    let tx = cita_vm::Transaction {
        from: vm.account1,
        to: Some(vm.account2),
        value: U256::from(0),
        nonce: U256::from(2),
        gas_limit: 80000,
        gas_price: U256::from(1),
        input: hex::decode("6d4ce63c").unwrap(),
        itype: InterpreterType::EVM,
    };
    let r = vm.executor.exec(cita_vm::Context::default(), tx).unwrap();
    println!("return={:?}", r);
}
