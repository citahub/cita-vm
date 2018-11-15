use crate::evm::Contract;
use crate::evm::EVMContext;
use crate::interpreter::gas_table;
use crate::interpreter::jump_table;
use crate::interpreter::memory::Memory;
use crate::interpreter::stack::Stack;
use crate::opcode::OpCode;
use ethereum_types::U256;
use std::collections::HashMap;
use std::error::Error;
use std::u64;
use crate::interpreter::common::to_word_size;

pub struct Config {
    pub jump_table: HashMap<u8, jump_table::Operation>,
}

pub struct Interpreter {
    ctx: EVMContext,
    cfg: Config,
    gas_table: gas_table::GasTable,
    read_only: bool,
    return_data: Vec<u8>,
}

impl Interpreter {
    pub fn new(ctx: EVMContext, cfg: Config) -> Self {
        Interpreter {
            ctx: ctx,
            cfg: cfg,
            gas_table: gas_table::GAS_TABLE,
            read_only: false,
            return_data: Vec::new(),
        }
    }

    // TODO: Next time someone asks you why you write this code, tell them you don't know also.
    // It's not necessary to perform this judgment maybe... Tests are required.
    pub fn enforce_restrictions(&self) -> bool {
        return true;
    }

    // TODO: go-ethereum run returns []byte from operation's execute.
    pub fn run(
        &mut self,
        contract: &mut Contract,
        input: Vec<u8>,
        read_only: bool,
    ) -> Result<Vec<u8>, Box<Error>> {
        // Init Funcs
        self.ctx.depth += 1;
        if read_only && !self.read_only {
            self.read_only = true;
        }
        self.return_data = Vec::new();
        if contract.code.len() == 0 {
            return Ok(Vec::new());
        }

        let op: OpCode;
        let mem = Memory::new();
        let stack: Stack<U256> = Stack::with_capacity(1024);
        let mut pc: u64 = 0;
        let cost: u64;

        contract.input = input;

        // Let's dance!
        while !self.ctx.abort {
            let opcode = contract.get_opcode(pc);
            let operation: &jump_table::Operation =
                self.cfg.jump_table.get(&(opcode as u8)).unwrap();
            if !operation.valid {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "invalid opcode",
                )));
            }
            if !(operation.validate_stack)(&mut self.ctx) {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "invalid stack",
                )));
            }

            if !self.enforce_restrictions() {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "invalid enforce restrictions",
                )));
            }

            // TODO: in go-ethereum there are some checks.
            let memory_size = (operation.memory_size)(&mut self.ctx);
            let memory_size = to_word_size(memory_size.as_u64());

            cost = (operation.gas_cost)(&mut self.gas_table, &mut self.ctx, &contract, memory_size)?;
            if !contract.use_gas(cost) {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "out of gas",
                )));
            }
            if memory_size > 0 {
                self.ctx.memory.resize(memory_size as usize);
            }

            // Execute the operation!
            // TODO: might could return []byte
            (operation.execute)(&mut self.ctx, &contract);

            let res = vec![0; 0];
            if operation.returns {
                // TODO:
                self.return_data = res.clone();
            }
            match operation {
                _ if operation.reverts => {
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "execution reverted",
                    )));
                }
                _ if operation.halts => return Ok(res.clone()),
                _ => pc += 1,
            }
            break;
        }
        // Drop Funcs
        self.read_only = false;
        self.ctx.depth -= 1;
        Ok(vec![0; 0])
    }

    pub fn can_run(&self) -> bool {
        false
    }
}
