use crate::evm::{Contract, EVMContext};
use crate::instructions;
use crate::interpreter::gas_table::GasTable;
use crate::opcode::OpCode;
use std::collections::HashMap;
use std::error;
use std::fmt;

const STACK_LIMIT: usize = 1024;

type ExecutionFunc = fn(&mut EVMContext, &Contract);
type GasFunc = fn(&mut GasTable, &mut EVMContext, &Contract) -> Result<u64, Box<error::Error>>;
type StackValidationFunc = fn(&mut EVMContext) -> bool;
type MemorySizeFunc = fn(&mut EVMContext) -> u64;

#[derive(Debug)]
struct ErrGasUintOverflow;

impl fmt::Display for ErrGasUintOverflow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "gas u64 overflow")
    }
}

impl error::Error for ErrGasUintOverflow {
    fn description(&self) -> &str {
        "gas u64 overflow"
    }
    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

const GAS_QUICK_STEP: u64 = 2;
const GAS_FASTEST_STEP: u64 = 3;
const GAS_FAST_STEP: u64 = 5;
const GAS_MID_STEP: u64 = 8;
const GAS_SLOW_STEP: u64 = 10;
const GAS_EXT_STEP: u64 = 20;
const GAS_RETURN: u64 = 0;
const GAS_STOP: u64 = 0;
const GAS_CONTRACT_BYTE: u64 = 200;

pub fn gas_func_const_zero(
    _: &mut GasTable,
    _: &mut EVMContext,
    _: &Contract,
) -> Result<u64, Box<error::Error>> {
    Ok(0)
}

pub fn gas_func_const_fastest_step(
    _: &mut GasTable,
    _: &mut EVMContext,
    _: &Contract,
) -> Result<u64, Box<error::Error>> {
    Ok(GAS_FASTEST_STEP)
}

pub fn stack_validation_func_bull(c: &mut EVMContext, pop: usize, push: usize) -> bool {
    if !c.stack.require(pop) {
        return false;
    }
    if c.stack.len() + push - pop > STACK_LIMIT as usize {
        return false;
    }
    return true;
}

pub fn stack_validation_func_0x00_0x00(c: &mut EVMContext) -> bool {
    stack_validation_func_bull(c, 0, 0)
}

pub fn stack_validation_func_0x02_0x01(c: &mut EVMContext) -> bool {
    stack_validation_func_bull(c, 2, 1)
}

pub fn memory_size_func_nil(_: &mut EVMContext) -> u64 {
    0
}

pub fn memory_size_func_const_0(_: &mut EVMContext) -> u64 {
    0
}

pub struct Operation {
    pub execute: ExecutionFunc,
    pub gas_cost: GasFunc,
    pub validate_stack: StackValidationFunc,
    pub memory_size: MemorySizeFunc,
    pub halts: bool,
    pub jumps: bool,
    pub writes: bool,
    pub valid: bool,
    pub reverts: bool,
    pub returns: bool,
}

pub fn new_frontier_instruction_set() -> HashMap<u8, Operation> {
    let mut data: HashMap<u8, Operation> = HashMap::new();
    let default_operation = Operation {
        execute: instructions::stop,
        gas_cost: gas_func_const_zero,
        validate_stack: stack_validation_func_0x00_0x00,
        memory_size: memory_size_func_nil,
        halts: false,
        jumps: false,
        writes: false,
        valid: false,
        reverts: false,
        returns: false,
    };
    // data.insert(
    //     OpCode::ADD as u8,
    //     Operation {
    //     ..default_operation
    // });
    data.insert(
        OpCode::STOP as u8,
        Operation {
            execute: instructions::stop,
            gas_cost: gas_func_const_zero,
            validate_stack: stack_validation_func_0x00_0x00,
            halts: true,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::ADD as u8,
        Operation {
            execute: instructions::add,
            gas_cost: gas_func_const_fastest_step,
            validate_stack: stack_validation_func_0x02_0x01,
            valid: true,
            ..default_operation
        },
    );
    data
}

pub fn new_homestead_instruction_set() -> HashMap<u8, Operation> {
    let data: HashMap<u8, Operation> = new_frontier_instruction_set();
    data
}

pub fn new_byzantium_instruction_set() -> HashMap<u8, Operation> {
    let data: HashMap<u8, Operation> = new_homestead_instruction_set();
    data
}

pub fn new_constantinople_instructionSet() -> HashMap<u8, Operation> {
    let data: HashMap<u8, Operation> = new_byzantium_instruction_set();
    data
}
