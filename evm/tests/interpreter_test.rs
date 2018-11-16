extern crate evm;
use evm::core;
use evm::interpreter;
use ethereum_types::*;
use evm::opcodes;

#[test]
fn test_interpreter_execute_0x01() {
    let mut context = core::EVMContext::new();
    context.contract.code = vec![
        opcodes::OpCode::PUSH1 as u8, 10,
        opcodes::OpCode::PUSH1 as u8, 0,
        opcodes::OpCode::MSTORE as u8,
        opcodes::OpCode::PUSH1 as u8, 32,
        opcodes::OpCode::PUSH1 as u8, 0,
        opcodes::OpCode::RETURN as u8,
    ];
    let mut it = interpreter::Interpreter::new(context);
    it.run();
    let r = U256::from_big_endian(&it.context.return_data[..]);
    assert_eq!(r, U256::from(10));
}


#[test]
fn test_interpreter_execute_0x02() {
    let mut context = core::EVMContext::new();
    context.contract.code = vec![
        opcodes::OpCode::PUSH2 as u8, 10, 0,
        opcodes::OpCode::MSTORE as u8,
        opcodes::OpCode::PUSH2 as u8, 32, 0,
        opcodes::OpCode::RETURN as u8,
    ];
    let mut it = interpreter::Interpreter::new(context);
    it.run();
    let r = U256::from_big_endian(&it.context.return_data[..]);
    assert_eq!(r, U256::from(10));
}
