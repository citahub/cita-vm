use crate::evm::{Contract, EVMContext};
use crate::instructions;
use crate::interpreter::common;
use crate::interpreter::common::to_word_size;
use crate::interpreter::gas_table;
use crate::interpreter::gas_table::GasTable;
use crate::interpreter::gas_table::GAS_TABLE;
use crate::interpreter::memory::Memory;
use crate::opcode::OpCode;
use ethereum_types::U256;
use std::collections::HashMap;
use std::error;
use std::error::Error;
use std::fmt;
use std::u64;

const STACK_LIMIT: usize = 1024;
const MEMORY_GAS: usize = 3;
const QUAD_COEFF_DIV: usize = 512;
const SHA3_GAS: usize = 30;
const SHA3_WORD_GAS: usize = 6;

type ExecutionFunc = fn(&mut EVMContext, &Contract);
type GasFunc = fn(&mut GasTable, &mut EVMContext, &Contract, u64) -> Result<u64, Box<error::Error>>;
type StackValidationFunc = fn(&mut EVMContext) -> bool;
type MemorySizeFunc = fn(&mut EVMContext) -> U256;

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
    _: u64,
) -> Result<u64, Box<error::Error>> {
    Ok(0)
}

pub fn gas_func_const_quick_step(
    _: &mut GasTable,
    _: &mut EVMContext,
    _: &Contract,
    _: u64,
) -> Result<u64, Box<error::Error>> {
    Ok(GAS_QUICK_STEP)
}

pub fn gas_func_const_fastest_step(
    _: &mut GasTable,
    _: &mut EVMContext,
    _: &Contract,
    _: u64,
) -> Result<u64, Box<error::Error>> {
    Ok(GAS_FASTEST_STEP)
}

pub fn gas_func_const_fast_step(
    _: &mut GasTable,
    _: &mut EVMContext,
    _: &Contract,
    _: u64,
) -> Result<u64, Box<error::Error>> {
    Ok(GAS_FAST_STEP)
}

pub fn gas_func_const_mid_step(
    _: &mut GasTable,
    _: &mut EVMContext,
    _: &Contract,
    _: u64,
) -> Result<u64, Box<error::Error>> {
    Ok(GAS_MID_STEP)
}

pub fn gas_func_const_slow_step(
    _: &mut GasTable,
    _: &mut EVMContext,
    _: &Contract,
    _: u64,
) -> Result<u64, Box<error::Error>> {
    Ok(GAS_SLOW_STEP)
}

pub fn gas_func_const_ext_step(
    _: &mut GasTable,
    _: &mut EVMContext,
    _: &Contract,
    _: u64,
) -> Result<u64, Box<error::Error>> {
    Ok(GAS_EXT_STEP)
}

pub fn gas_func_const_return(
    _: &mut GasTable,
    _: &mut EVMContext,
    _: &Contract,
    _: u64,
) -> Result<u64, Box<error::Error>> {
    Ok(GAS_RETURN)
}
pub fn gas_func_const_stop(
    _: &mut GasTable,
    _: &mut EVMContext,
    _: &Contract,
    _: u64,
) -> Result<u64, Box<error::Error>> {
    Ok(GAS_STOP)
}
pub fn gas_func_const_contract_byte(
    _: &mut GasTable,
    _: &mut EVMContext,
    _: &Contract,
    _: u64,
) -> Result<u64, Box<error::Error>> {
    Ok(GAS_CONTRACT_BYTE)
}

pub fn gas_func_exp(
    g: &mut GasTable,
    c: &mut EVMContext,
    _: &Contract,
    _: u64,
) -> Result<u64, Box<error::Error>> {
    let data = c.stack.data();
    let r: U256 = data[c.stack.len() - 2];
    let bitlen = common::u256_bit_len(r);
    let exp_byte_len = (bitlen + 7) / 8;
    let gas = exp_byte_len * g.exp_byte;

    if gas > u64::MAX - GAS_SLOW_STEP {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "errGasUintOverflow",
        )));
    }
    Ok(gas)
}

pub fn memory_gas_cost(mem: &mut Memory, new_mem_size: u64) -> Result<u64, Box<Error>> {
    if new_mem_size == 0 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "errGasUintOverflow",
        )));
    }
    if new_mem_size > 0xffffffffe0 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "errGasUintOverflow",
        )));
    }

    let new_mem_size_words = to_word_size(new_mem_size);
    let new_mem_size = new_mem_size_words * 32;
    if new_mem_size > mem.len() as u64 {
        let square = new_mem_size_words * new_mem_size_words;
        let lin_coef = new_mem_size_words * MEMORY_GAS as u64;
        let quad_coef = square / QUAD_COEFF_DIV as u64;
        let new_total_fee = lin_coef + quad_coef;
        let fee = new_total_fee - mem.last_gas_cost;
        mem.last_gas_cost = new_total_fee;
        return Ok(fee);
    }
    Ok(0)
}

pub fn gas_sha3(
    _: &mut GasTable,
    c: &mut EVMContext,
    _: &Contract,
    memory_size: u64,
) -> Result<u64, Box<error::Error>> {
    let gas = memory_gas_cost(&mut c.memory, memory_size)?;
    let gas = gas + SHA3_GAS as u64;
    let word_gas = c.stack.back(1).as_u64();
    let word_gas = common::to_word_size(word_gas) * SHA3_WORD_GAS as u64;
    let gas = gas + word_gas;
    Ok(gas)
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

pub fn stack_validation_func_0x01_0x01(c: &mut EVMContext) -> bool {
    stack_validation_func_bull(c, 1, 1)
}

pub fn stack_validation_func_0x02_0x01(c: &mut EVMContext) -> bool {
    stack_validation_func_bull(c, 2, 1)
}

pub fn stack_validation_func_0x03_0x01(c: &mut EVMContext) -> bool {
    stack_validation_func_bull(c, 3, 1)
}

pub fn memory_nil(_: &mut EVMContext) -> U256 {
    U256::zero()
}

pub fn memory_const_0(_: &mut EVMContext) -> U256 {
    U256::zero()
}

pub fn memory_sha3(c: &mut EVMContext) -> U256 {
    common::calc_mem_size(c.stack.back(0), c.stack.back(1))
}

pub fn memory_call_data_copy(c: &mut EVMContext) -> U256 {
    common::calc_mem_size(c.stack.back(0), c.stack.back(2))
}

pub fn memory_return_data_copy(c: &mut EVMContext) -> U256 {
    common::calc_mem_size(c.stack.back(0), c.stack.back(2))
}

pub fn memory_code_copy(c: &mut EVMContext) -> U256 {
    common::calc_mem_size(c.stack.back(0), c.stack.back(2))
}

pub fn memory_ext_code_copy(c: &mut EVMContext) -> U256 {
    common::calc_mem_size(c.stack.back(1), c.stack.back(3))
}

pub fn memory_mload(c: &mut EVMContext) -> U256 {
    common::calc_mem_size(c.stack.back(0), U256::from(32))
}

pub fn memory_mstore8(c: &mut EVMContext) -> U256 {
    common::calc_mem_size(c.stack.back(0), U256::from(1))
}

pub fn memory_mstore(c: &mut EVMContext) -> U256 {
    common::calc_mem_size(c.stack.back(0), U256::from(32))
}

pub fn memory_create(c: &mut EVMContext) -> U256 {
    common::calc_mem_size(c.stack.back(1), c.stack.back(2))
}

pub fn memory_create2(c: &mut EVMContext) -> U256 {
    common::calc_mem_size(c.stack.back(1), c.stack.back(2))
}

pub fn memory_call(c: &mut EVMContext) -> U256 {
    let x = common::calc_mem_size(c.stack.back(5), c.stack.back(6));
    let y = common::calc_mem_size(c.stack.back(3), c.stack.back(4));
    if x > y {
        return x;
    }
    y
}

pub fn memory_delegate_call(c: &mut EVMContext) -> U256 {
    let x = common::calc_mem_size(c.stack.back(4), c.stack.back(5));
    let y = common::calc_mem_size(c.stack.back(2), c.stack.back(3));
    if x > y {
        return x;
    }
    y
}

pub fn memory_static_call(c: &mut EVMContext) -> U256 {
    let x = common::calc_mem_size(c.stack.back(4), c.stack.back(5));
    let y = common::calc_mem_size(c.stack.back(2), c.stack.back(3));
    if x > y {
        return x;
    }
    y
}

pub fn memory_return(c: &mut EVMContext) -> U256 {
    common::calc_mem_size(c.stack.back(0), c.stack.back(1))
}

pub fn memory_revert(c: &mut EVMContext) -> U256 {
    common::calc_mem_size(c.stack.back(0), c.stack.back(1))
}

pub fn memory_log(c: &mut EVMContext) -> U256 {
    let (msize, mstart) = (c.stack.back(1), c.stack.back(0));
    common::calc_mem_size(mstart, msize)
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

pub fn new_instruction_set() -> HashMap<u8, Operation> {
    let mut data: HashMap<u8, Operation> = HashMap::new();
    let default_operation = Operation {
        execute: instructions::stop,
        gas_cost: gas_func_const_zero,
        validate_stack: stack_validation_func_0x00_0x00,
        memory_size: memory_nil,
        halts: false,
        jumps: false,
        writes: false,
        valid: false,
        reverts: false,
        returns: false,
    };
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
    data.insert(
        OpCode::MUL as u8,
        Operation {
            execute: instructions::mul,
            gas_cost: gas_func_const_fast_step,
            validate_stack: stack_validation_func_0x02_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::SUB as u8,
        Operation {
            execute: instructions::sub,
            gas_cost: gas_func_const_fastest_step,
            validate_stack: stack_validation_func_0x02_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::DIV as u8,
        Operation {
            execute: instructions::div,
            gas_cost: gas_func_const_fast_step,
            validate_stack: stack_validation_func_0x02_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::SDIV as u8,
        Operation {
            execute: instructions::sdiv,
            gas_cost: gas_func_const_fast_step,
            validate_stack: stack_validation_func_0x02_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::MOD as u8,
        Operation {
            execute: instructions::r#mod,
            gas_cost: gas_func_const_fast_step,
            validate_stack: stack_validation_func_0x02_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::SMOD as u8,
        Operation {
            execute: instructions::smod,
            gas_cost: gas_func_const_fast_step,
            validate_stack: stack_validation_func_0x02_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::ADDMOD as u8,
        Operation {
            execute: instructions::add_mod,
            gas_cost: gas_func_const_mid_step,
            validate_stack: stack_validation_func_0x03_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::MULMOD as u8,
        Operation {
            execute: instructions::mul_mod,
            gas_cost: gas_func_const_mid_step,
            validate_stack: stack_validation_func_0x03_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::EXP as u8,
        Operation {
            execute: instructions::exp,
            gas_cost: gas_func_exp,
            validate_stack: stack_validation_func_0x02_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::SIGNEXTEND as u8,
        Operation {
            execute: instructions::sign_extend,
            gas_cost: gas_func_const_fast_step,
            validate_stack: stack_validation_func_0x02_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::LT as u8,
        Operation {
            execute: instructions::lt,
            gas_cost: gas_func_const_fastest_step,
            validate_stack: stack_validation_func_0x02_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::GT as u8,
        Operation {
            execute: instructions::gt,
            gas_cost: gas_func_const_fastest_step,
            validate_stack: stack_validation_func_0x02_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::SLT as u8,
        Operation {
            execute: instructions::slt,
            gas_cost: gas_func_const_fastest_step,
            validate_stack: stack_validation_func_0x02_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::SGT as u8,
        Operation {
            execute: instructions::sgt,
            gas_cost: gas_func_const_fastest_step,
            validate_stack: stack_validation_func_0x02_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::EQ as u8,
        Operation {
            execute: instructions::eq,
            gas_cost: gas_func_const_fastest_step,
            validate_stack: stack_validation_func_0x02_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::ISZERO as u8,
        Operation {
            execute: instructions::is_zero,
            gas_cost: gas_func_const_fastest_step,
            validate_stack: stack_validation_func_0x01_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::AND as u8,
        Operation {
            execute: instructions::and,
            gas_cost: gas_func_const_fastest_step,
            validate_stack: stack_validation_func_0x02_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::XOR as u8,
        Operation {
            execute: instructions::xor,
            gas_cost: gas_func_const_fastest_step,
            validate_stack: stack_validation_func_0x02_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::OR as u8,
        Operation {
            execute: instructions::or,
            gas_cost: gas_func_const_fastest_step,
            validate_stack: stack_validation_func_0x02_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::NOT as u8,
        Operation {
            execute: instructions::not,
            gas_cost: gas_func_const_fastest_step,
            validate_stack: stack_validation_func_0x01_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::BYTE as u8,
        Operation {
            execute: instructions::byte,
            gas_cost: gas_func_const_fastest_step,
            validate_stack: stack_validation_func_0x02_0x01,
            valid: true,
            ..default_operation
        },
    );
    data.insert(
        OpCode::SHA3 as u8,
        Operation {
            execute: instructions::sha3,
            gas_cost: gas_sha3,
            validate_stack: stack_validation_func_0x02_0x01,
            valid: true,
            ..default_operation
        },
    );
    data
}
