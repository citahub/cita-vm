use super::common;
use super::err;
use super::ext;
use super::memory;
use super::opcodes;
use super::stack;
use ethereum_types::*;
use std::cmp;

#[derive(Clone, Default)]
pub struct Context {
    pub origin: Address,
    pub gas_price: U256,
    pub gas_limit: u64,
    pub coinbase: Address,
    pub number: U256,
    pub timestamp: u64,
    pub difficulty: U256,
}

// Log is the data struct for LOG0...LOG4.
// The members are "Topics: Vec<H256>, Body: Vec<u8>"
#[derive(Clone)]
pub struct Log(pub Vec<H256>, pub Vec<u8>);

pub enum InterpreterResult {
    // Return data, remain gas and Logs.
    Normal(Vec<u8>, u64, Vec<Log>),
    Revert(Vec<u8>, u64, Vec<Log>),
    // Return data, contract address and remain gas.
    Create(Vec<u8>, Address, u64),
}

#[derive(Clone)]
pub struct InterpreterConf {
    pub print_op: bool,
    pub print_gas_used: bool,
    pub print_stack: bool,
    pub print_mem: bool,
    pub eip1283: bool,
    pub stack_limit: u64,

    pub gas_tier_step: [u64; 8],
    pub gas_exp: u64,
    pub gas_exp_byte: u64,
    pub gas_sha3: u64,
    pub gas_sha3_word: u64,
    pub gas_memory: u64,
    pub gas_sload: u64,
    pub gas_sstore_noop: u64,
    pub gas_sstore_init: u64,
    pub gas_sstore_clear_refund: u64,
    pub gas_sstore_clean: u64,
    pub gas_sstore_dirty: u64,
    pub gas_sstore_reset_clear_refund: u64,
    pub gas_sstore_reset_refund: u64,
    pub gas_sstore_set: u64,
    pub gas_sstore_clear: u64,
    pub gas_sstore_reset: u64,
    pub gas_sstore_refund: u64,
    pub gas_log: u64,
    pub gas_log_data: u64,
    pub gas_log_topic: u64,
    pub gas_create: u64,
    pub gas_jumpdest: u64,
    pub gas_copy: u64,
    pub gas_call: u64,
    pub gas_call_new_account: u64,
    pub gas_call_value_transfer: u64,
    pub gas_call_stipend: u64,
    pub gas_self_destruct: u64,
    pub gas_extcode_size: u64,
}

impl InterpreterConf {
    // The default is version "2d0661f 2018-11-08" of https://ethereum.github.io/yellowpaper/paper.pdf
    // But in order to pass the test, Some modifications must needed.
    //
    // If you want to step through the steps, let the
    //   conf.print_op = true;
    //   conf.print_gas_used = true;
    //   conf.print_stack = true;
    //   conf.print_mem = true;
    pub fn default() -> Self {
        InterpreterConf {
            print_op: false,
            print_gas_used: false,
            print_stack: false,
            print_mem: false,
            eip1283: false,
            stack_limit: 1024,

            gas_tier_step: [0, 2, 3, 5, 8, 10, 20, 0],
            gas_exp: 10,
            gas_exp_byte: 50,
            gas_sha3: 30,
            gas_sha3_word: 6,
            gas_memory: 3,
            gas_sload: 200,
            gas_sstore_noop: 200,
            gas_sstore_init: 20000,
            gas_sstore_clear_refund: 15000,
            gas_sstore_clean: 5000,
            gas_sstore_dirty: 200,
            gas_sstore_reset_clear_refund: 19800,
            gas_sstore_reset_refund: 4800,
            gas_sstore_set: 20000,
            gas_sstore_clear: 5000,
            gas_sstore_reset: 5000,
            gas_sstore_refund: 15000,
            gas_log: 375,
            gas_log_data: 8,
            gas_log_topic: 375,
            gas_create: 32000,
            gas_jumpdest: 1,
            gas_copy: 3,
            gas_call: 700,
            gas_call_new_account: 25000,
            gas_call_value_transfer: 9000,
            gas_call_stipend: 2300,
            gas_self_destruct: 5000,
            gas_extcode_size: 20,
        }
    }
}

#[derive(Clone, Default)]
pub struct Contract {
    pub code_address: Address,
    pub code_data: Vec<u8>,
}

#[derive(Clone, Default)]
pub struct InterpreterParams {
    pub address: Address,
    pub sender: Address,
    pub value: U256,
    pub input: Vec<u8>,
    pub read_only: bool,
    pub contract: Contract,
    pub gas: u64,
    pub extra: U256,
}

pub struct Interpreter {
    pub context: Context,
    pub cfg: InterpreterConf,
    pub data_provider: Box<ext::DataProvider>,
    pub params: InterpreterParams,

    gas: u64,
    stack: stack::Stack<U256>,
    mem: memory::Memory,
    logs: Vec<Log>,
    return_data: Vec<u8>,
    mem_gas: u64,
    gas_tmp: u64,
}

impl Interpreter {
    pub fn new(
        context: Context,
        cfg: InterpreterConf,
        data_provider: Box<ext::DataProvider>,
        params: InterpreterParams,
    ) -> Self {
        let gas = params.gas;
        Interpreter {
            context,
            cfg,
            data_provider,
            params,
            gas,
            stack: stack::Stack::with_capacity(1024),
            mem: memory::Memory::default(),
            logs: Vec::new(),
            return_data: Vec::new(),
            mem_gas: 0,
            gas_tmp: 0,
        }
    }

    #[allow(clippy::cyclomatic_complexity)]
    pub fn run(&mut self) -> Result<InterpreterResult, err::Error> {
        let this = &mut *self;
        let mut pc = 0;
        loop {
            // Step 0: Get opcode
            let op = this.get_op(pc)?;
            pc += 1;
            // Step 1: Log opcode(if necessary)
            if this.cfg.print_op {
                if op >= opcodes::OpCode::PUSH1 && op <= opcodes::OpCode::PUSH32 {
                    let n = op.clone() as u8 - opcodes::OpCode::PUSH1 as u8 + 1;
                    let r = {
                        if pc + u64::from(n) > this.params.contract.code_data.len() as u64 {
                            U256::zero()
                        } else {
                            U256::from(
                                &this.params.contract.code_data
                                    [pc as usize..(pc + u64::from(n)) as usize],
                            )
                        }
                    };
                    println!("[OP] {} {:#x}", op, r);
                } else {
                    println!("[OP] {}", op);
                }
                if this.cfg.print_stack {
                    println!("[STACK]");
                    let l = this.stack.data().len();
                    for i in 0..l {
                        println!("[{}] {:#x}", i, this.stack.back(i));
                    }
                }
                if this.cfg.print_mem {
                    println!("[MEM] len={}", this.mem.len());
                }
            }
            // Step 2: Valid stack
            let stack_require = op.stack_require();
            if !this.stack.require(stack_require as usize) {
                return Err(err::Error::OutOfStack);
            }
            if this.stack.len() as u64 - op.stack_require() + op.stack_returns()
                > this.cfg.stack_limit
            {
                return Err(err::Error::OutOfStack);
            }
            // Step 3: Valid state mod
            if this.params.read_only && op.state_changes() {
                return Err(err::Error::MutableCallInStaticContext);
            }
            // Step 4: Gas cost and mem expand.
            let op_gas = this.cfg.gas_tier_step[op.gas_price_tier().idx()];
            this.use_gas(op_gas)?;
            match op {
                opcodes::OpCode::EXP => {
                    let expon = this.stack.back(1);
                    let bytes = ((expon.bits() + 7) / 8) as u64;
                    let gas = this.cfg.gas_exp + this.cfg.gas_exp_byte * bytes;
                    this.use_gas(gas)?;
                }
                opcodes::OpCode::SHA3 => {
                    let mem_offset = this.stack.back(0);
                    let mem_len = this.stack.back(1);
                    this.mem_gas_work(mem_offset, mem_len)?;
                    this.use_gas(this.cfg.gas_sha3)?;
                    this.use_gas(common::to_word_size(mem_len.low_u64()) * this.cfg.gas_sha3_word)?;
                }
                opcodes::OpCode::CALLDATACOPY => {
                    let mem_offset = this.stack.back(0);
                    let mem_len = this.stack.back(2);
                    this.mem_gas_work(mem_offset, mem_len)?;
                    let gas = common::to_word_size(mem_len.low_u64()) * this.cfg.gas_copy;
                    this.use_gas(gas)?;
                }
                opcodes::OpCode::CODECOPY => {
                    let mem_offset = this.stack.back(0);
                    let mem_len = this.stack.back(2);
                    this.mem_gas_work(mem_offset, mem_len)?;
                    let gas = common::to_word_size(mem_len.low_u64()) * this.cfg.gas_copy;
                    this.use_gas(gas)?;
                }
                opcodes::OpCode::EXTCODESIZE => {
                    this.use_gas(this.cfg.gas_extcode_size)?;
                }
                opcodes::OpCode::MLOAD => {
                    let mem_offset = this.stack.back(0);
                    let mem_len = U256::from(32);
                    this.mem_gas_work(mem_offset, mem_len)?;
                }
                opcodes::OpCode::MSTORE => {
                    let mem_offset = this.stack.back(0);
                    let mem_len = U256::from(32);
                    this.mem_gas_work(mem_offset, mem_len)?;
                }
                opcodes::OpCode::MSTORE8 => {
                    let mem_offset = this.stack.back(0);
                    let mem_len = U256::from(8);
                    this.mem_gas_work(mem_offset, mem_len)?;
                }
                opcodes::OpCode::SLOAD => {
                    this.use_gas(this.cfg.gas_sload)?;
                }
                opcodes::OpCode::SSTORE => {
                    let address = H256::from(&this.stack.back(0));
                    let current_value = U256::from(
                        &*this
                            .data_provider
                            .get_storage(&this.params.address, &address),
                    );
                    let new_value = this.stack.back(1);
                    let original_value = U256::from(
                        &*this
                            .data_provider
                            .get_storage_origin(&this.params.address, &address),
                    );
                    let gas: u64 = {
                        if this.cfg.eip1283 {
                            // See https://github.com/ethereum/EIPs/blob/master/EIPS/eip-1283.md
                            if current_value == new_value {
                                this.cfg.gas_sstore_noop
                            } else if original_value == current_value {
                                if original_value.is_zero() {
                                    this.cfg.gas_sstore_init
                                } else {
                                    if new_value.is_zero() {
                                        this.data_provider.add_refund(
                                            &this.params.address,
                                            this.cfg.gas_sstore_clear_refund,
                                        );
                                    }
                                    this.cfg.gas_sstore_clean
                                }
                            } else {
                                if !original_value.is_zero() {
                                    if current_value.is_zero() {
                                        this.data_provider.sub_refund(
                                            &this.params.address,
                                            this.cfg.gas_sstore_clear_refund,
                                        );
                                    } else if new_value.is_zero() {
                                        this.data_provider.add_refund(
                                            &this.params.address,
                                            this.cfg.gas_sstore_clear_refund,
                                        );
                                    }
                                }
                                if original_value == new_value {
                                    if original_value.is_zero() {
                                        this.data_provider.add_refund(
                                            &this.params.address,
                                            this.cfg.gas_sstore_reset_clear_refund,
                                        );
                                    } else {
                                        this.data_provider.add_refund(
                                            &this.params.address,
                                            this.cfg.gas_sstore_reset_refund,
                                        );
                                    }
                                }
                                this.cfg.gas_sstore_dirty
                            }
                        } else if current_value.is_zero() && !new_value.is_zero() {
                            this.cfg.gas_sstore_set
                        } else if !current_value.is_zero() && new_value.is_zero() {
                            this.data_provider
                                .add_refund(&this.params.address, this.cfg.gas_sstore_refund);
                            this.cfg.gas_sstore_clear
                        } else {
                            this.cfg.gas_sstore_reset
                        }
                    };
                    this.use_gas(gas)?;
                }
                opcodes::OpCode::JUMPDEST => {
                    this.use_gas(this.cfg.gas_jumpdest)?;
                }
                opcodes::OpCode::LOG0
                | opcodes::OpCode::LOG1
                | opcodes::OpCode::LOG2
                | opcodes::OpCode::LOG3
                | opcodes::OpCode::LOG4 => {
                    let n = op.clone() as u8 - opcodes::OpCode::LOG0 as u8;
                    let mem_offset = this.stack.back(0);
                    let mem_len = this.stack.back(1);
                    this.mem_gas_work(mem_offset, mem_len)?;
                    let gas = this.cfg.gas_log
                        + this.cfg.gas_log_topic * u64::from(n)
                        + this.cfg.gas_log_data * mem_len.low_u64();
                    this.use_gas(gas)?;
                }
                opcodes::OpCode::CREATE => {
                    let mem_offset = this.stack.back(1);
                    let mem_len = this.stack.back(2);
                    this.mem_gas_work(mem_offset, mem_len)?;
                    this.use_gas(this.cfg.gas_create)?;
                    this.gas_tmp = this.gas - this.gas / 64;
                    this.use_gas(this.gas_tmp)?;
                }
                opcodes::OpCode::CALL | opcodes::OpCode::CALLCODE => {
                    let gas_req = this.stack.back(0);
                    let address = common::u256_to_address(&this.stack.back(1));
                    let value = this.stack.back(2);
                    let mem_offset = this.stack.back(3);
                    let mem_len = this.stack.back(4);
                    let out_offset = this.stack.back(5);
                    let out_len = this.stack.back(6);

                    if gas_req.bits() > 64 {
                        return Err(err::Error::OutOfGas);
                    }
                    this.use_gas(this.cfg.gas_call)?;

                    let is_value_transfer = !value.is_zero();
                    if op == opcodes::OpCode::CALL
                        && is_value_transfer
                        && this.data_provider.is_empty(&address)
                    {
                        this.use_gas(this.cfg.gas_call_new_account)?;
                    }
                    if is_value_transfer {
                        this.use_gas(this.cfg.gas_call_value_transfer)?;
                    }
                    this.mem_gas_work(mem_offset, mem_len)?;
                    this.mem_gas_work(out_offset, out_len)?;
                    this.gas_tmp = cmp::min(this.gas - this.gas / 64, gas_req.low_u64());
                    this.use_gas(this.gas_tmp)?;
                }
                opcodes::OpCode::RETURN => {
                    let mem_offset = this.stack.back(0);
                    let mem_len = this.stack.back(1);
                    this.mem_gas_work(mem_offset, mem_len)?;
                }
                opcodes::OpCode::DELEGATECALL | opcodes::OpCode::STATICCALL => {
                    let gas_req = this.stack.back(0);
                    let mem_offset = this.stack.back(2);
                    let mem_len = this.stack.back(3);
                    let out_offset = this.stack.back(4);
                    let out_len = this.stack.back(5);
                    if gas_req.bits() > 64 {
                        return Err(err::Error::OutOfGas);
                    }
                    this.use_gas(this.cfg.gas_call)?;
                    this.mem_gas_work(mem_offset, mem_len)?;
                    this.mem_gas_work(out_offset, out_len)?;
                    this.gas_tmp = cmp::min(this.gas - this.gas / 64, gas_req.low_u64());
                    this.use_gas(this.gas_tmp)?;
                }
                opcodes::OpCode::CREATE2 => {
                    let mem_offset = this.stack.back(1);
                    let mem_len = this.stack.back(2);
                    this.mem_gas_work(mem_offset, mem_len)?;
                    this.use_gas(this.cfg.gas_create)?;
                    this.use_gas(common::to_word_size(mem_len.low_u64()) * this.cfg.gas_sha3_word)?;
                    this.gas_tmp = this.gas - this.gas / 64;
                    this.use_gas(this.gas_tmp)?;
                }
                opcodes::OpCode::REVERT => {
                    let mem_offset = this.stack.back(0);
                    let mem_len = this.stack.back(1);
                    this.mem_gas_work(mem_offset, mem_len)?;
                }
                opcodes::OpCode::SELFDESTRUCT => {
                    this.use_gas(this.cfg.gas_self_destruct)?;
                }
                _ => {}
            }
            // Step 5: Let's dance!
            match op {
                opcodes::OpCode::STOP => break,
                opcodes::OpCode::ADD => {
                    let a = this.stack.pop();
                    let b = this.stack.pop();
                    this.stack.push(a.overflowing_add(b).0);
                }
                opcodes::OpCode::MUL => {
                    let a = this.stack.pop();
                    let b = this.stack.pop();
                    this.stack.push(a.overflowing_mul(b).0);
                }
                opcodes::OpCode::SUB => {
                    let a = this.stack.pop();
                    let b = this.stack.pop();
                    this.stack.push(a.overflowing_sub(b).0);
                }
                opcodes::OpCode::DIV => {
                    let a = this.stack.pop();
                    let b = this.stack.pop();
                    this.stack
                        .push(if b.is_zero() { U256::zero() } else { a / b })
                }
                opcodes::OpCode::SDIV => {
                    let (a, neg_a) = common::get_sign(this.stack.pop());
                    let (b, neg_b) = common::get_sign(this.stack.pop());
                    let min = (U256::one() << 255) - U256::one();
                    this.stack.push(if b.is_zero() {
                        U256::zero()
                    } else if a == min && b == !U256::one() {
                        min
                    } else {
                        common::set_sign(a / b, neg_a ^ neg_b)
                    })
                }
                opcodes::OpCode::MOD => {
                    let a = this.stack.pop();
                    let b = this.stack.pop();
                    this.stack
                        .push(if !b.is_zero() { a % b } else { U256::zero() });
                }
                opcodes::OpCode::SMOD => {
                    let ua = this.stack.pop();
                    let ub = this.stack.pop();
                    let (a, sign_a) = common::get_sign(ua);
                    let b = common::get_sign(ub).0;
                    this.stack.push(if !b.is_zero() {
                        let c = a % b;
                        common::set_sign(c, sign_a)
                    } else {
                        U256::zero()
                    });
                }
                opcodes::OpCode::ADDMOD => {
                    let a = this.stack.pop();
                    let b = this.stack.pop();
                    let c = this.stack.pop();
                    this.stack.push(if !c.is_zero() {
                        let a5 = U512::from(a);
                        let res = a5.overflowing_add(U512::from(b)).0;
                        let x = res % U512::from(c);
                        U256::from(x)
                    } else {
                        U256::zero()
                    });
                }
                opcodes::OpCode::MULMOD => {
                    let a = this.stack.pop();
                    let b = this.stack.pop();
                    let c = this.stack.pop();
                    this.stack.push(if !c.is_zero() {
                        let a5 = U512::from(a);
                        let res = a5.overflowing_mul(U512::from(b)).0;
                        let x = res % U512::from(c);
                        U256::from(x)
                    } else {
                        U256::zero()
                    });
                }
                opcodes::OpCode::EXP => {
                    let base = this.stack.pop();
                    let expon = this.stack.pop();
                    let res = base.overflowing_pow(expon).0;
                    this.stack.push(res);
                }
                opcodes::OpCode::SIGNEXTEND => {
                    let bit = this.stack.pop();
                    if bit < U256::from(32) {
                        let number = this.stack.pop();
                        let bit_position = (bit.low_u64() * 8 + 7) as usize;
                        let bit = number.bit(bit_position);
                        let mask = (U256::one() << bit_position) - U256::one();
                        this.stack
                            .push(if bit { number | !mask } else { number & mask });
                    }
                }
                opcodes::OpCode::LT => {
                    let a = this.stack.pop();
                    let b = this.stack.pop();
                    this.stack.push(common::bool_to_u256(a < b));
                }
                opcodes::OpCode::GT => {
                    let a = this.stack.pop();
                    let b = this.stack.pop();
                    this.stack.push(common::bool_to_u256(a > b));
                }
                opcodes::OpCode::SLT => {
                    let (a, neg_a) = common::get_sign(this.stack.pop());
                    let (b, neg_b) = common::get_sign(this.stack.pop());
                    let is_positive_lt = a < b && !(neg_a | neg_b);
                    let is_negative_lt = a > b && (neg_a & neg_b);
                    let has_different_signs = neg_a && !neg_b;
                    this.stack.push(common::bool_to_u256(
                        is_positive_lt | is_negative_lt | has_different_signs,
                    ));
                }
                opcodes::OpCode::SGT => {
                    let (a, neg_a) = common::get_sign(this.stack.pop());
                    let (b, neg_b) = common::get_sign(this.stack.pop());
                    let is_positive_gt = a > b && !(neg_a | neg_b);
                    let is_negative_gt = a < b && (neg_a & neg_b);
                    let has_different_signs = !neg_a && neg_b;
                    this.stack.push(common::bool_to_u256(
                        is_positive_gt | is_negative_gt | has_different_signs,
                    ));
                }
                opcodes::OpCode::EQ => {
                    let a = this.stack.pop();
                    let b = this.stack.pop();
                    this.stack.push(common::bool_to_u256(a == b));
                }
                opcodes::OpCode::ISZERO => {
                    let a = this.stack.pop();
                    this.stack.push(common::bool_to_u256(a.is_zero()));
                }
                opcodes::OpCode::AND => {
                    let a = this.stack.pop();
                    let b = this.stack.pop();
                    this.stack.push(a & b);
                }
                opcodes::OpCode::OR => {
                    let a = this.stack.pop();
                    let b = this.stack.pop();
                    this.stack.push(a | b);
                }
                opcodes::OpCode::XOR => {
                    let a = this.stack.pop();
                    let b = this.stack.pop();
                    this.stack.push(a ^ b);
                }
                opcodes::OpCode::NOT => {
                    let a = this.stack.pop();
                    this.stack.push(!a);
                }
                opcodes::OpCode::BYTE => {
                    let word = this.stack.pop();
                    let val = this.stack.pop();
                    let byte = if word < U256::from(32) {
                        (val >> (8 * (31 - word.low_u64() as usize))) & U256::from(0xff)
                    } else {
                        U256::zero()
                    };
                    this.stack.push(byte);
                }
                opcodes::OpCode::SHL => {
                    const CONST_256: U256 = U256([256, 0, 0, 0]);
                    let shift = this.stack.pop();
                    let value = this.stack.pop();
                    let result = if shift >= CONST_256 {
                        U256::zero()
                    } else {
                        value << (shift.as_u32() as usize)
                    };
                    this.stack.push(result);
                }
                opcodes::OpCode::SHR => {
                    const CONST_256: U256 = U256([256, 0, 0, 0]);
                    let shift = this.stack.pop();
                    let value = this.stack.pop();
                    let result = if shift >= CONST_256 {
                        U256::zero()
                    } else {
                        value >> (shift.as_u32() as usize)
                    };
                    this.stack.push(result);
                }
                opcodes::OpCode::SAR => {
                    const CONST_256: U256 = U256([256, 0, 0, 0]);
                    const CONST_HIBIT: U256 = U256([0, 0, 0, 0x8000_0000_0000_0000]);
                    let shift = this.stack.pop();
                    let value = this.stack.pop();
                    let sign = value & CONST_HIBIT != U256::zero();
                    let result = if shift >= CONST_256 {
                        if sign {
                            U256::max_value()
                        } else {
                            U256::zero()
                        }
                    } else {
                        let shift = shift.as_u32() as usize;
                        let mut shifted = value >> shift;
                        if sign {
                            shifted = shifted | (U256::max_value() << (256 - shift));
                        }
                        shifted
                    };
                    this.stack.push(result);
                }
                opcodes::OpCode::SHA3 => {
                    let mem_offset = this.stack.pop();
                    let mem_len = this.stack.pop();
                    let k = this.data_provider.sha3(
                        this.mem
                            .get(mem_offset.low_u64() as usize, mem_len.low_u64() as usize),
                    );
                    this.stack.push(U256::from(k));
                }
                opcodes::OpCode::ADDRESS => {
                    this.stack
                        .push(common::address_to_u256(this.params.address));
                }
                opcodes::OpCode::BALANCE => {
                    let address = common::u256_to_address(&this.stack.pop());
                    let balance = this.data_provider.get_balance(&address);
                    this.stack.push(balance);
                }
                opcodes::OpCode::ORIGIN => {
                    this.stack
                        .push(common::address_to_u256(this.context.origin));
                }
                opcodes::OpCode::CALLER => {
                    this.stack.push(common::address_to_u256(this.params.sender));
                }
                opcodes::OpCode::CALLVALUE => {
                    this.stack.push(this.params.value);
                }
                opcodes::OpCode::CALLDATALOAD => {
                    let big_id = this.stack.pop();
                    let id = big_id.low_u64() as usize;
                    let max = id.wrapping_add(32);
                    if !this.params.input.is_empty() {
                        let bound = cmp::min(this.params.input.len(), max);
                        if id < bound && big_id < U256::from(this.params.input.len()) {
                            let mut v = [0u8; 32];
                            v[0..bound - id].clone_from_slice(&this.params.input[id..bound]);
                            this.stack.push(U256::from(&v[..]))
                        } else {
                            this.stack.push(U256::zero())
                        }
                    } else {
                        this.stack.push(U256::zero())
                    }
                }
                opcodes::OpCode::CALLDATASIZE => {
                    this.stack.push(U256::from(this.params.input.len()));
                }
                opcodes::OpCode::CALLDATACOPY => {
                    let mem_offset = this.stack.pop();
                    let raw_offset = this.stack.pop();
                    let size = this.stack.pop();

                    let data = common::copy_data(this.params.input.as_slice(), raw_offset, size);
                    this.mem.set(mem_offset.as_usize(), data.as_slice());
                }
                opcodes::OpCode::CODESIZE => {
                    this.stack
                        .push(U256::from(this.params.contract.code_data.len()));
                }
                opcodes::OpCode::CODECOPY => {
                    let mem_offset = this.stack.pop();
                    let raw_offset = this.stack.pop();
                    let size = this.stack.pop();
                    let data = common::copy_data(
                        this.params.contract.code_data.as_slice(),
                        raw_offset,
                        size,
                    );
                    this.mem.set(mem_offset.as_usize(), data.as_slice());
                }
                opcodes::OpCode::GASPRICE => {
                    this.stack.push(this.context.gas_price);
                }
                opcodes::OpCode::EXTCODESIZE => {
                    let address = common::u256_to_address(&this.stack.pop());
                    let len = this.data_provider.get_code_size(&address);
                    this.stack.push(U256::from(len));
                }
                opcodes::OpCode::EXTCODECOPY => {
                    let address = common::u256_to_address(&this.stack.pop());
                    let code_offset = this.stack.pop();
                    let len = this.stack.pop();
                    let code = this.data_provider.get_code(&address);
                    let val = common::copy_data(code, code_offset, len);
                    this.mem.set(code_offset.as_usize(), val.as_slice())
                }
                opcodes::OpCode::RETURNDATASIZE => {
                    this.stack.push(U256::from(this.return_data.len()))
                }
                opcodes::OpCode::RETURNDATACOPY => {
                    let mem_offset = this.stack.pop();
                    let raw_offset = this.stack.pop();
                    let size = this.stack.pop();
                    let return_data_len = U256::from(this.return_data.len());
                    if raw_offset.saturating_add(size) > return_data_len {
                        return Err(err::Error::OutOfBounds);
                    }
                    let data = common::copy_data(&this.return_data, raw_offset, size);
                    this.mem.set(mem_offset.as_usize(), data.as_slice())
                }
                opcodes::OpCode::EXTCODEHASH => {
                    let address = common::u256_to_address(&this.stack.pop());
                    let hash = this.data_provider.get_code_hash(&address);
                    this.stack.push(U256::from(hash));
                }
                opcodes::OpCode::BLOCKHASH => {
                    let block_number = this.stack.pop();
                    let block_hash = this.data_provider.get_block_hash(&block_number);
                    this.stack.push(U256::from(&*block_hash));
                }
                opcodes::OpCode::COINBASE => {
                    this.stack
                        .push(common::address_to_u256(this.context.coinbase));
                }
                opcodes::OpCode::TIMESTAMP => {
                    this.stack.push(U256::from(this.context.timestamp));
                }
                opcodes::OpCode::NUMBER => {
                    this.stack.push(this.context.number);
                }
                opcodes::OpCode::DIFFICULTY => {
                    this.stack.push(this.context.difficulty);
                }
                opcodes::OpCode::GASLIMIT => {
                    this.stack.push(U256::from(this.context.gas_limit));
                }
                opcodes::OpCode::POP => {
                    this.stack.pop();
                }
                opcodes::OpCode::MLOAD => {
                    let offset = this.stack.pop().as_u64();
                    let word = this.mem.get(offset as usize, 32);
                    this.stack.push(U256::from(word));
                }
                opcodes::OpCode::MSTORE => {
                    let offset = this.stack.pop();
                    let word = this.stack.pop();
                    let word = &<[u8; 32]>::from(word)[..];
                    this.mem.set(offset.low_u64() as usize, word);
                }
                opcodes::OpCode::MSTORE8 => {
                    let offset = this.stack.pop();
                    let word = this.stack.pop();
                    this.mem
                        .set(offset.low_u64() as usize, &[word.low_u64() as u8]);
                }
                opcodes::OpCode::SLOAD => {
                    let key = H256::from(this.stack.pop());
                    let word =
                        U256::from(&*this.data_provider.get_storage(&this.params.address, &key));
                    this.stack.push(word);
                }
                opcodes::OpCode::SSTORE => {
                    let address = H256::from(&this.stack.pop());
                    let value = this.stack.pop();
                    this.data_provider.set_storage(
                        &this.params.address,
                        address,
                        H256::from(&value),
                    );
                }
                opcodes::OpCode::JUMP => {
                    let jump = this.stack.pop().low_u64();
                    if jump >= this.params.contract.code_data.len() as u64 {
                        return Err(err::Error::OutOfCode);
                    }
                    pc = jump;
                }
                opcodes::OpCode::JUMPI => {
                    let jump = this.stack.pop().low_u64();
                    let condition = this.stack.pop();
                    if !condition.is_zero() {
                        if jump >= this.params.contract.code_data.len() as u64 {
                            return Err(err::Error::OutOfCode);
                        }
                        pc = jump;
                    }
                }
                opcodes::OpCode::PC => {
                    this.stack.push(U256::from(pc - 1));
                }
                opcodes::OpCode::MSIZE => {
                    this.stack.push(U256::from(this.mem.len()));
                }
                opcodes::OpCode::GAS => {
                    this.stack.push(U256::from(this.gas));
                }
                opcodes::OpCode::JUMPDEST => {}
                opcodes::OpCode::PUSH1
                | opcodes::OpCode::PUSH2
                | opcodes::OpCode::PUSH3
                | opcodes::OpCode::PUSH4
                | opcodes::OpCode::PUSH5
                | opcodes::OpCode::PUSH6
                | opcodes::OpCode::PUSH7
                | opcodes::OpCode::PUSH8
                | opcodes::OpCode::PUSH9
                | opcodes::OpCode::PUSH10
                | opcodes::OpCode::PUSH11
                | opcodes::OpCode::PUSH12
                | opcodes::OpCode::PUSH13
                | opcodes::OpCode::PUSH14
                | opcodes::OpCode::PUSH15
                | opcodes::OpCode::PUSH16
                | opcodes::OpCode::PUSH17
                | opcodes::OpCode::PUSH18
                | opcodes::OpCode::PUSH19
                | opcodes::OpCode::PUSH20
                | opcodes::OpCode::PUSH21
                | opcodes::OpCode::PUSH22
                | opcodes::OpCode::PUSH23
                | opcodes::OpCode::PUSH24
                | opcodes::OpCode::PUSH25
                | opcodes::OpCode::PUSH26
                | opcodes::OpCode::PUSH27
                | opcodes::OpCode::PUSH28
                | opcodes::OpCode::PUSH29
                | opcodes::OpCode::PUSH30
                | opcodes::OpCode::PUSH31
                | opcodes::OpCode::PUSH32 => {
                    let n = op as u8 - opcodes::OpCode::PUSH1 as u8 + 1;
                    let e = pc + u64::from(n);
                    let e = cmp::min(e, this.params.contract.code_data.len() as u64);
                    let r = U256::from(&this.params.contract.code_data[pc as usize..e as usize]);
                    pc = e;
                    this.stack.push(r);
                }
                opcodes::OpCode::DUP1
                | opcodes::OpCode::DUP2
                | opcodes::OpCode::DUP3
                | opcodes::OpCode::DUP4
                | opcodes::OpCode::DUP5
                | opcodes::OpCode::DUP6
                | opcodes::OpCode::DUP7
                | opcodes::OpCode::DUP8
                | opcodes::OpCode::DUP9
                | opcodes::OpCode::DUP10
                | opcodes::OpCode::DUP11
                | opcodes::OpCode::DUP12
                | opcodes::OpCode::DUP13
                | opcodes::OpCode::DUP14
                | opcodes::OpCode::DUP15
                | opcodes::OpCode::DUP16 => {
                    let p = op as u8 - opcodes::OpCode::DUP1 as u8;
                    this.stack.dup(p as usize);
                }
                opcodes::OpCode::SWAP1
                | opcodes::OpCode::SWAP2
                | opcodes::OpCode::SWAP3
                | opcodes::OpCode::SWAP4
                | opcodes::OpCode::SWAP5
                | opcodes::OpCode::SWAP6
                | opcodes::OpCode::SWAP7
                | opcodes::OpCode::SWAP8
                | opcodes::OpCode::SWAP9
                | opcodes::OpCode::SWAP10
                | opcodes::OpCode::SWAP11
                | opcodes::OpCode::SWAP12
                | opcodes::OpCode::SWAP13
                | opcodes::OpCode::SWAP14
                | opcodes::OpCode::SWAP15
                | opcodes::OpCode::SWAP16 => {
                    let p = op as u8 - opcodes::OpCode::SWAP1 as u8 + 1;
                    this.stack.swap(p as usize);
                }
                opcodes::OpCode::LOG0
                | opcodes::OpCode::LOG1
                | opcodes::OpCode::LOG2
                | opcodes::OpCode::LOG3
                | opcodes::OpCode::LOG4 => {
                    let n = op as u8 - opcodes::OpCode::LOG0 as u8;
                    let mem_offset = this.stack.pop();
                    let mem_len = this.stack.pop();
                    let mut topics: Vec<H256> = Vec::new();
                    for _ in 0..n {
                        let r = H256::from(this.stack.pop());
                        topics.push(r);
                    }
                    let data = this
                        .mem
                        .get(mem_offset.low_u64() as usize, mem_len.low_u64() as usize);
                    this.logs.push(Log(topics, Vec::from(data)));
                }
                opcodes::OpCode::CREATE | opcodes::OpCode::CREATE2 => {
                    let value = this.stack.pop();
                    let mem_offset = this.stack.pop();
                    let mem_len = this.stack.pop();
                    let salt = {
                        match op {
                            opcodes::OpCode::CREATE => U256::zero(),
                            opcodes::OpCode::CREATE2 => this.stack.pop(),
                            _ => panic!("instruction can only be CREATE/CREATE2 checked above"),
                        }
                    };
                    let data = this
                        .mem
                        .get(mem_offset.low_u64() as usize, mem_len.low_u64() as usize);

                    let mut params = InterpreterParams::default();
                    params.sender = this.params.address;
                    params.gas = this.gas_tmp;
                    params.input = Vec::from(data);
                    params.value = value;
                    params.extra = salt;
                    let r = this.data_provider.call(op, params);
                    match r {
                        Ok(data) => match data {
                            InterpreterResult::Create(_, add, gas) => {
                                this.stack.push(common::address_to_u256(add));
                                this.gas += gas;
                            }
                            InterpreterResult::Revert(ret, gas, _) => {
                                this.stack.push(U256::zero());
                                this.gas += gas;
                                this.return_data = ret;
                                break;
                            }
                            _ => {}
                        },
                        Err(_) => {
                            this.stack.push(U256::zero());
                        }
                    }
                }
                opcodes::OpCode::CALL
                | opcodes::OpCode::CALLCODE
                | opcodes::OpCode::DELEGATECALL
                | opcodes::OpCode::STATICCALL => {
                    let _ = this.stack.pop();
                    let address = common::u256_to_address(&this.stack.pop());
                    let value = {
                        if op == opcodes::OpCode::CALL || op == opcodes::OpCode::CALLCODE {
                            this.stack.pop()
                        } else {
                            U256::zero()
                        }
                    };
                    let mem_offset = this.stack.pop();
                    let mem_len = this.stack.pop();
                    let out_offset = this.stack.pop();
                    let _ = this.stack.pop();

                    let mut gas = this.gas_tmp;
                    if !value.is_zero() {
                        gas += this.cfg.gas_call_stipend;
                    }

                    let data = this
                        .mem
                        .get(mem_offset.low_u64() as usize, mem_len.low_u64() as usize);

                    let mut params = InterpreterParams::default();
                    params.sender = this.params.address;
                    params.gas = gas;
                    params.contract.code_address = address;
                    params.contract.code_data =
                        Vec::from(this.data_provider.get_code(&params.contract.code_address));
                    params.input = Vec::from(data);
                    match op {
                        opcodes::OpCode::CALL => {
                            params.address = address;
                            params.value = value;
                        }
                        opcodes::OpCode::CALLCODE => {
                            params.address = this.params.address;
                            params.value = value;
                        }
                        opcodes::OpCode::DELEGATECALL => {
                            params.address = this.params.address;
                        }
                        opcodes::OpCode::STATICCALL => {
                            params.address = address;
                        }
                        _ => {}
                    }
                    let r = this.data_provider.call(op, params);
                    match r {
                        Ok(data) => match data {
                            InterpreterResult::Normal(ret, gas, _) => {
                                this.stack.push(U256::one());
                                this.mem.set(out_offset.low_u64() as usize, ret.as_slice());
                                this.gas += gas;
                            }
                            InterpreterResult::Revert(ret, gas, _) => {
                                this.stack.push(U256::zero());
                                this.mem.set(out_offset.low_u64() as usize, ret.as_slice());
                                this.gas += gas;
                            }
                            _ => {}
                        },
                        Err(_) => {
                            this.stack.push(U256::zero());
                        }
                    }
                }
                opcodes::OpCode::RETURN => {
                    let mem_offset = this.stack.pop();
                    let mem_len = this.stack.pop();
                    let r = this
                        .mem
                        .get(mem_offset.low_u64() as usize, mem_len.low_u64() as usize);
                    this.return_data = Vec::from(r);
                    break;
                }
                opcodes::OpCode::REVERT => {
                    let mem_offset = this.stack.pop();
                    let mem_len = this.stack.pop();
                    let r = this
                        .mem
                        .get(mem_offset.low_u64() as usize, mem_len.low_u64() as usize);
                    this.return_data = Vec::from(r);
                    return Ok(InterpreterResult::Revert(
                        this.return_data.clone(),
                        this.gas,
                        this.logs.clone(),
                    ));
                }
                opcodes::OpCode::SELFDESTRUCT => {
                    let address = this.stack.pop();
                    this.data_provider
                        .selfdestruct(&this.params.address, &common::u256_to_address(&address));
                    break;
                }
            }
            if this.cfg.print_op {
                println!();
            }
        }
        Ok(InterpreterResult::Normal(
            this.return_data.clone(),
            this.gas,
            this.logs.clone(),
        ))
    }

    fn use_gas(&mut self, gas: u64) -> Result<(), err::Error> {
        if self.cfg.print_gas_used && gas != 0 {
            println!("[Gas] - {}", gas);
        }
        if self.gas < gas {
            return Err(err::Error::OutOfGas);
        }
        self.gas -= gas;
        Ok(())
    }

    fn mem_gas_cost(&mut self, size: u64) -> u64 {
        let goc = common::mem_gas_cost(size, self.cfg.gas_memory);
        if goc > self.mem_gas {
            let fee = goc - self.mem_gas;
            self.mem_gas = goc;
            fee
        } else {
            0
        }
    }

    fn mem_gas_work(&mut self, mem_offset: U256, mem_len: U256) -> Result<(), err::Error> {
        let (mem_sum, b) = mem_offset.overflowing_add(mem_len);
        if b || mem_sum.bits() > 64 {
            return Err(err::Error::OutOfGas);
        }
        if mem_len != U256::zero() {
            let gas = self.mem_gas_cost(mem_sum.low_u64());
            self.use_gas(gas)?;
        }
        self.mem.expand(mem_sum.low_u64() as usize);
        Ok(())
    }

    fn get_byte(&self, n: u64) -> u8 {
        if n < self.params.contract.code_data.len() as u64 {
            return self.params.contract.code_data[n as usize];
        }
        0
    }

    fn get_op(&self, n: u64) -> Result<opcodes::OpCode, err::Error> {
        if let Some(data) = opcodes::OpCode::from_u8(self.get_byte(n)) {
            return Ok(data);
        }
        Err(err::Error::InvalidOpcode)
    }
}

#[cfg(test)]
mod tests {
    // The unit tests just carried from go-ethereum.
    use super::super::extmock;
    use super::*;

    fn default_interpreter() -> Interpreter {
        let mut it = Interpreter::new(
            Context::default(),
            InterpreterConf::default(),
            Box::new(extmock::DataProviderMock::default()),
            InterpreterParams::default(),
        );
        it.context.gas_limit = 1_000_000;
        it.params.gas = 1_000_000;
        it.gas = 1_000_000;
        it
    }

    #[test]
    fn test_interpreter_execute() {
        let mut it = default_interpreter();;
        it.params.contract.code_data = vec![
            opcodes::OpCode::PUSH1 as u8,
            10,
            opcodes::OpCode::PUSH1 as u8,
            0,
            opcodes::OpCode::MSTORE as u8,
            opcodes::OpCode::PUSH1 as u8,
            32,
            opcodes::OpCode::PUSH1 as u8,
            0,
            opcodes::OpCode::RETURN as u8,
        ];
        it.run().unwrap();
        let r = U256::from_big_endian(&it.return_data[..]);
        assert_eq!(r, U256::from(10));
    }

    #[test]
    fn test_op_byte() {
        let data = vec![
            (
                "ABCDEF0908070605040302010000000000000000000000000000000000000000",
                "0",
                "AB",
            ),
            (
                "ABCDEF0908070605040302010000000000000000000000000000000000000000",
                "1",
                "CD",
            ),
            (
                "00CDEF090807060504030201ffffffffffffffffffffffffffffffffffffffff",
                "0",
                "00",
            ),
            (
                "ABCDEF0908070605040302010000000000000000000000000000000000000000",
                "1",
                "CD",
            ),
            (
                "00CDEF090807060504030201ffffffffffffffffffffffffffffffffffffffff",
                "0",
                "00",
            ),
            (
                "00CDEF090807060504030201ffffffffffffffffffffffffffffffffffffffff",
                "1",
                "CD",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000102030",
                "31",
                "30",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000102030",
                "30",
                "20",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "32",
                "00",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "4294967295",
                "00",
            ),
        ];
        for (val, th, expected) in data {
            let mut it = default_interpreter();;
            it.stack
                .push_n(&[U256::from(val), U256::from(th.parse::<u64>().unwrap())]);
            it.params.contract.code_data = vec![opcodes::OpCode::BYTE as u8];
            it.run().unwrap();
            assert_eq!(it.stack.pop(), U256::from(expected));
        }
    }

    #[test]
    fn test_op_shl() {
        let data = vec![
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "00",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "01",
                "0000000000000000000000000000000000000000000000000000000000000002",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "ff",
                "8000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "0100",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "0101",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "00",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "01",
                "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "ff",
                "8000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0100",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000000",
                "01",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "01",
                "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe",
            ),
        ];
        for (x, y, expected) in data {
            let mut it = default_interpreter();;
            it.stack.push_n(&[U256::from(x), U256::from(y)]);
            it.params.contract.code_data = vec![opcodes::OpCode::SHL as u8];
            it.run().unwrap();
            assert_eq!(it.stack.pop(), U256::from(expected));
        }
    }

    #[test]
    fn test_op_shr() {
        let data = vec![
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "00",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "01",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000000",
                "01",
                "4000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000000",
                "ff",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000000",
                "0100",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000000",
                "0101",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "00",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "01",
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "ff",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0100",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000000",
                "01",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
        ];
        for (x, y, expected) in data {
            let mut it = default_interpreter();;
            it.stack.push_n(&[U256::from(x), U256::from(y)]);
            it.params.contract.code_data = vec![opcodes::OpCode::SHR as u8];
            it.run().unwrap();
            assert_eq!(it.stack.pop(), U256::from(expected));
        }
    }

    #[test]
    fn test_op_sar() {
        let data = vec![
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "00",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "01",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000000",
                "01",
                "c000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000000",
                "ff",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000000",
                "0100",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000000",
                "0101",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "00",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "01",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "ff",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0100",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000000",
                "01",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "4000000000000000000000000000000000000000000000000000000000000000",
                "fe",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "f8",
                "000000000000000000000000000000000000000000000000000000000000007f",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "fe",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "ff",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0100",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
        ];
        for (x, y, expected) in data {
            let mut it = default_interpreter();;
            it.stack.push_n(&[U256::from(x), U256::from(y)]);
            it.params.contract.code_data = vec![opcodes::OpCode::SAR as u8];
            it.run().unwrap();
            assert_eq!(it.stack.pop(), U256::from(expected));
        }
    }

    #[test]
    fn test_op_sgt() {
        let data = vec![
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000001",
                "8000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000001",
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "8000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffb",
                "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd",
                "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffb",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
        ];
        for (x, y, expected) in data {
            let mut it = default_interpreter();;
            it.stack.push_n(&[U256::from(x), U256::from(y)]);
            it.params.contract.code_data = vec![opcodes::OpCode::SGT as u8];
            it.run().unwrap();
            assert_eq!(it.stack.pop(), U256::from(expected));
        }
    }

    #[test]
    fn test_op_slt() {
        let data = vec![
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "0000000000000000000000000000000000000000000000000000000000000001",
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000001",
                "8000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "8000000000000000000000000000000000000000000000000000000000000001",
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "8000000000000000000000000000000000000000000000000000000000000001",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
            (
                "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffb",
                "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            (
                "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd",
                "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffb",
                "0000000000000000000000000000000000000000000000000000000000000001",
            ),
        ];
        for (x, y, expected) in data {
            let mut it = default_interpreter();;
            it.stack.push_n(&[U256::from(x), U256::from(y)]);
            it.params.contract.code_data = vec![opcodes::OpCode::SLT as u8];
            it.run().unwrap();
            assert_eq!(it.stack.pop(), U256::from(expected));
        }
    }

    #[test]
    fn test_op_mstore() {
        let mut it = default_interpreter();;
        let v = "abcdef00000000000000abba000000000deaf000000c0de00100000000133700";
        it.stack.push_n(&[U256::from(v), U256::zero()]);
        it.params.contract.code_data = vec![opcodes::OpCode::MSTORE as u8];
        it.run().unwrap();
        assert_eq!(it.mem.get(0, 32), common::hex_decode(v).unwrap().as_slice());
        it.stack.push_n(&[U256::one(), U256::zero()]);
        it.params.contract.code_data = vec![opcodes::OpCode::MSTORE as u8];
        it.run().unwrap();
        assert_eq!(
            it.mem.get(0, 32),
            common::hex_decode("0000000000000000000000000000000000000000000000000000000000000001")
                .unwrap()
                .as_slice()
        );
    }

    #[test]
    fn test_op_sstore_eip_1283() {
        // From https://github.com/ethereum/EIPs/blob/master/EIPS/eip-1283.md#test-cases
        let data = vec![
            ("0x60006000556000600055", 412, 0, 0),
            ("0x60006000556001600055", 20212, 0, 0),
            ("0x60016000556000600055", 20212, 19800, 0),
            ("0x60016000556002600055", 20212, 0, 0),
            ("0x60016000556001600055", 20212, 0, 0),
            ("0x60006000556000600055", 5212, 15000, 1),
            ("0x60006000556001600055", 5212, 4800, 1),
            ("0x60006000556002600055", 5212, 0, 1),
            ("0x60026000556000600055", 5212, 15000, 1),
            ("0x60026000556003600055", 5212, 0, 1),
            ("0x60026000556001600055", 5212, 4800, 1),
            ("0x60026000556002600055", 5212, 0, 1),
            ("0x60016000556000600055", 5212, 15000, 1),
            ("0x60016000556002600055", 5212, 0, 1),
            ("0x60016000556001600055", 412, 0, 1),
            ("0x600160005560006000556001600055", 40218, 19800, 0),
            ("0x600060005560016000556000600055", 10218, 19800, 1),
        ];
        for (code, use_gas, refund, origin) in data {
            let mut it = default_interpreter();;
            it.cfg.eip1283 = true;
            assert_eq!(it.gas, it.context.gas_limit);
            it.data_provider.set_storage_origin(
                &it.params.contract.code_address,
                H256::zero(),
                H256::from(origin),
            );
            it.data_provider.set_storage(
                &it.params.contract.code_address,
                H256::zero(),
                H256::from(origin),
            );
            it.params.contract.code_data = common::hex_decode(code).unwrap();
            it.run().unwrap();
            assert_eq!(it.gas, it.context.gas_limit - use_gas);
            assert_eq!(it.data_provider.get_refund(&Address::zero()), refund);
        }
    }

    #[test]
    fn test_op_invalid() {
        let mut it = default_interpreter();;
        it.params.contract.code_data = common::hex_decode("0xfb").unwrap();
        let r = it.run();
        assert!(r.is_err());
        assert_eq!(r.err(), Some(err::Error::InvalidOpcode))
    }

    #[test]
    fn test_op_call() {
        // Test from https://github.com/ethereum/tests/blob/develop/GeneralStateTests/stCallCodes/callcall_00.json
        // Step from go-ethereum:
        //
        // PUSH1           pc=00000000 gas=1000000 cost=3
        // PUSH1           pc=00000002 gas=999997 cost=3
        // PUSH1           pc=00000004 gas=999994 cost=3
        // PUSH1           pc=00000006 gas=999991 cost=3
        // PUSH1           pc=00000008 gas=999988 cost=3
        // PUSH20          pc=00000010 gas=999985 cost=3
        // PUSH3           pc=00000031 gas=999982 cost=3
        // CALL            pc=00000035 gas=999979 cost=359706     // 1st CALL
        // PUSH1           pc=00000000 gas=352300 cost=3
        // PUSH1           pc=00000002 gas=352297 cost=3
        // PUSH1           pc=00000004 gas=352294 cost=3
        // PUSH1           pc=00000006 gas=352291 cost=3
        // PUSH1           pc=00000008 gas=352288 cost=3
        // PUSH20          pc=00000010 gas=352285 cost=3
        // PUSH3           pc=00000031 gas=352282 cost=3
        // CALL            pc=00000035 gas=352279 cost=259706     // 2nd CALL
        // PUSH1           pc=00000000 gas=252300 cost=3
        // PUSH1           pc=00000002 gas=252297 cost=3
        // SSTORE          pc=00000004 gas=252294 cost=20000
        // CALLER          pc=00000005 gas=232294 cost=2
        // PUSH1           pc=00000006 gas=232292 cost=3
        // SSTORE          pc=00000008 gas=232289 cost=20000
        // CALLVALUE       pc=00000009 gas=212289 cost=2
        // PUSH1           pc=00000010 gas=212287 cost=3
        // SSTORE          pc=00000012 gas=212284 cost=20000
        // ADDRESS         pc=00000013 gas=192284 cost=2
        // PUSH1           pc=00000014 gas=192282 cost=3
        // SSTORE          pc=00000016 gas=192279 cost=20000
        // ORIGIN          pc=00000017 gas=172279 cost=2
        // PUSH1           pc=00000018 gas=172277 cost=3
        // SSTORE          pc=00000020 gas=172274 cost=5000
        // CALLDATASIZE    pc=00000021 gas=167274 cost=2
        // PUSH1           pc=00000022 gas=167272 cost=3
        // SSTORE          pc=00000024 gas=167269 cost=20000
        // CODESIZE        pc=00000025 gas=147269 cost=2
        // PUSH1           pc=00000026 gas=147267 cost=3
        // SSTORE          pc=00000028 gas=147264 cost=20000
        // GASPRICE        pc=00000029 gas=127264 cost=2
        // PUSH1           pc=00000030 gas=127262 cost=3
        // SSTORE          pc=00000032 gas=127259 cost=20000
        // STOP            pc=00000033 gas=107259 cost=0          // 2nd CALL end
        // PUSH1           pc=00000036 gas=199832 cost=3
        // SSTORE          pc=00000038 gas=199829 cost=20000
        // STOP            pc=00000039 gas=179829 cost=0          // 1nd CALL end
        // PUSH1           pc=00000036 gas=820102 cost=3
        // SSTORE          pc=00000038 gas=820099 cost=20000
        // STOP            pc=00000039 gas=800099 cost=0
        //
        // Cost = 199901
        let mut it = default_interpreter();
        it.context.gas_price = U256::one();
        let mut data_provider = extmock::DataProviderMock::default();

        let mut account0 = extmock::Account::default();
        account0.balance = U256::from("0de0b6b3a7640000");
        account0.code = common::hex_decode(
            "0x6040600060406000600173100000000000000000000000000000000000000162055730f1600055",
        )
        .unwrap();
        data_provider.db.insert(
            Address::from("0x1000000000000000000000000000000000000000"),
            account0,
        );

        let mut account1 = extmock::Account::default();
        account1.balance = U256::from("0de0b6b3a7640000");
        account1.code = common::hex_decode(
            "0x604060006040600060027310000000000000000000000000000000000000026203d090f1600155",
        )
        .unwrap();
        data_provider.db.insert(
            Address::from("0x1000000000000000000000000000000000000001"),
            account1,
        );

        let mut account2 = extmock::Account::default();
        account2.balance = U256::zero();
        account2.code = common::hex_decode(
            "0x600160025533600455346007553060e6553260e8553660ec553860ee553a60f055",
        )
        .unwrap();
        data_provider.db.insert(
            Address::from("0x1000000000000000000000000000000000000002"),
            account2,
        );

        it.data_provider = Box::new(data_provider);
        it.params.contract.code_data = common::hex_decode(
            "0x6040600060406000600173100000000000000000000000000000000000000162055730f1600055",
        )
        .unwrap();
        let r = it.run().unwrap();
        match r {
            InterpreterResult::Normal(_, gas, _) => assert_eq!(gas, it.params.gas - 199_901),
            _ => assert!(false),
        }
    }
}
