use super::core;
use super::opcodes;
use ethereum_types::*;

pub struct Interpreter {
    pub context: core::EVMContext,
}

impl Interpreter {
    pub fn new(context: core::EVMContext) -> Self {
        Interpreter { context: context }
    }

    pub fn run(&mut self) {
        let this = &mut *self;
        let mut pc = 0;
        loop {
            let op = this.context.contract.get_opcode(pc);
            println!("Opcode: {}", op);
            match op {
                opcodes::OpCode::STOP => { break }
                opcodes::OpCode::ADD => {
                    let a = this.context.stack.pop();
                    let b = this.context.stack.pop();
                    this.context.stack.push(a + b);
                }
                opcodes::OpCode::MUL => {}
                opcodes::OpCode::SUB => {}
                opcodes::OpCode::DIV => {}
                opcodes::OpCode::SDIV => {}
                opcodes::OpCode::MOD => {}
                opcodes::OpCode::SMOD => {}
                opcodes::OpCode::ADDMOD => {}
                opcodes::OpCode::MULMOD => {}
                opcodes::OpCode::EXP => {}
                opcodes::OpCode::SIGNEXTEND => {}
                opcodes::OpCode::LT => {}
                opcodes::OpCode::GT => {}
                opcodes::OpCode::SLT => {}
                opcodes::OpCode::SGT => {}
                opcodes::OpCode::EQ => {}
                opcodes::OpCode::ISZERO => {}
                opcodes::OpCode::AND => {}
                opcodes::OpCode::OR => {}
                opcodes::OpCode::XOR => {}
                opcodes::OpCode::NOT => {}
                opcodes::OpCode::BYTE => {}
                opcodes::OpCode::SHL => {}
                opcodes::OpCode::SHR => {}
                opcodes::OpCode::SAR => {}
                opcodes::OpCode::SHA3 => {}
                opcodes::OpCode::ADDRESS => {}
                opcodes::OpCode::BALANCE => {}
                opcodes::OpCode::ORIGIN => {}
                opcodes::OpCode::CALLER => {}
                opcodes::OpCode::CALLVALUE => {}
                opcodes::OpCode::CALLDATALOAD => {}
                opcodes::OpCode::CALLDATASIZE => {}
                opcodes::OpCode::CALLDATACOPY => {}
                opcodes::OpCode::CODESIZE => {}
                opcodes::OpCode::CODECOPY => {}
                opcodes::OpCode::GASPRICE => {}
                opcodes::OpCode::EXTCODESIZE => {}
                opcodes::OpCode::EXTCODECOPY => {}
                opcodes::OpCode::RETURNDATASIZE => {}
                opcodes::OpCode::RETURNDATACOPY => {}
                opcodes::OpCode::EXTCODEHASH => {}
                opcodes::OpCode::BLOCKHASH => {}
                opcodes::OpCode::COINBASE => {}
                opcodes::OpCode::TIMESTAMP => {}
                opcodes::OpCode::NUMBER => {}
                opcodes::OpCode::DIFFICULTY => {}
                opcodes::OpCode::GASLIMIT => {}
                opcodes::OpCode::POP => {}
                opcodes::OpCode::MLOAD => {}
                opcodes::OpCode::MSTORE => {
                    let offset = this.context.stack.pop();
                    let word = this.context.stack.pop();
                    let word = &<[u8; 32]>::from(word)[..];
                    this.context.memory.expand(offset.as_u64() as usize + 32);
                    this.context.memory.set(offset.as_u64() as usize, word);
                }
                opcodes::OpCode::MSTORE8 => {}
                opcodes::OpCode::SLOAD => {}
                opcodes::OpCode::SSTORE => {}
                opcodes::OpCode::JUMP => {}
                opcodes::OpCode::JUMPI => {}
                opcodes::OpCode::PC => {}
                opcodes::OpCode::MSIZE => {}
                opcodes::OpCode::GAS => {}
                opcodes::OpCode::JUMPDEST => {}
                opcodes::OpCode::PUSH1 => {
                    pc += 1;
                    let r = this.context.contract.get_byte(pc);
                    let r = U256::from(r);
                    this.context.stack.push(r);
                }
                opcodes::OpCode::PUSH2 => {}
                opcodes::OpCode::PUSH3 => {}
                opcodes::OpCode::PUSH4 => {}
                opcodes::OpCode::PUSH5 => {}
                opcodes::OpCode::PUSH6 => {}
                opcodes::OpCode::PUSH7 => {}
                opcodes::OpCode::PUSH8 => {}
                opcodes::OpCode::PUSH9 => {}
                opcodes::OpCode::PUSH10 => {}
                opcodes::OpCode::PUSH11 => {}
                opcodes::OpCode::PUSH12 => {}
                opcodes::OpCode::PUSH13 => {}
                opcodes::OpCode::PUSH14 => {}
                opcodes::OpCode::PUSH15 => {}
                opcodes::OpCode::PUSH16 => {}
                opcodes::OpCode::PUSH17 => {}
                opcodes::OpCode::PUSH18 => {}
                opcodes::OpCode::PUSH19 => {}
                opcodes::OpCode::PUSH20 => {}
                opcodes::OpCode::PUSH21 => {}
                opcodes::OpCode::PUSH22 => {}
                opcodes::OpCode::PUSH23 => {}
                opcodes::OpCode::PUSH24 => {}
                opcodes::OpCode::PUSH25 => {}
                opcodes::OpCode::PUSH26 => {}
                opcodes::OpCode::PUSH27 => {}
                opcodes::OpCode::PUSH28 => {}
                opcodes::OpCode::PUSH29 => {}
                opcodes::OpCode::PUSH30 => {}
                opcodes::OpCode::PUSH31 => {}
                opcodes::OpCode::PUSH32 => {}
                opcodes::OpCode::DUP1 => {}
                opcodes::OpCode::DUP2 => {}
                opcodes::OpCode::DUP3 => {}
                opcodes::OpCode::DUP4 => {}
                opcodes::OpCode::DUP5 => {}
                opcodes::OpCode::DUP6 => {}
                opcodes::OpCode::DUP7 => {}
                opcodes::OpCode::DUP8 => {}
                opcodes::OpCode::DUP9 => {}
                opcodes::OpCode::DUP10 => {}
                opcodes::OpCode::DUP11 => {}
                opcodes::OpCode::DUP12 => {}
                opcodes::OpCode::DUP13 => {}
                opcodes::OpCode::DUP14 => {}
                opcodes::OpCode::DUP15 => {}
                opcodes::OpCode::DUP16 => {}
                opcodes::OpCode::SWAP1 => {}
                opcodes::OpCode::SWAP2 => {}
                opcodes::OpCode::SWAP3 => {}
                opcodes::OpCode::SWAP4 => {}
                opcodes::OpCode::SWAP5 => {}
                opcodes::OpCode::SWAP6 => {}
                opcodes::OpCode::SWAP7 => {}
                opcodes::OpCode::SWAP8 => {}
                opcodes::OpCode::SWAP9 => {}
                opcodes::OpCode::SWAP10 => {}
                opcodes::OpCode::SWAP11 => {}
                opcodes::OpCode::SWAP12 => {}
                opcodes::OpCode::SWAP13 => {}
                opcodes::OpCode::SWAP14 => {}
                opcodes::OpCode::SWAP15 => {}
                opcodes::OpCode::SWAP16 => {}
                opcodes::OpCode::LOG0 => {}
                opcodes::OpCode::LOG1 => {}
                opcodes::OpCode::LOG2 => {}
                opcodes::OpCode::LOG3 => {}
                opcodes::OpCode::LOG4 => {}
                opcodes::OpCode::CREATE => {}
                opcodes::OpCode::CALL => {}
                opcodes::OpCode::CALLCODE => {}
                opcodes::OpCode::RETURN => {
                    let init_off = this.context.stack.pop().as_u64() as usize;
				    let init_size = this.context.stack.pop().as_u64() as usize;
                    let r = this.context.memory.get(init_off, init_size);
                    this.context.return_data = Vec::from(r);
                    break
                }
                opcodes::OpCode::DELEGATECALL => {}
                opcodes::OpCode::CREATE2 => {}
                opcodes::OpCode::REVERT => {}
                opcodes::OpCode::STATICCALL => {}
                opcodes::OpCode::SUICIDE => {}
            }
            pc += 1;
        }
    }
}
