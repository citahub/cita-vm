#![feature(duration_as_u128)]
extern crate cita_vm;
extern crate evm;

use cita_vm::json_tests::common::*;
use evm::extmock;
use evm::interpreter;
use std::env;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::time;

fn test_json_file(p: &str) {
    let f = fs::File::open(p).unwrap();
    let t = cita_vm::json_tests::vm_test::Test::load(f).unwrap();
    for (name, data) in t.into_iter() {
        let tic = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        io::stderr()
            .write(format!("{}::{}", p, name).as_bytes())
            .unwrap();
        let vm: cita_vm::json_tests::vm_test::Vm = data;
        // Init context
        let mut ctx = interpreter::Context::default();
        ctx.coinbase = vm.env.current_coinbase;
        ctx.difficulty = string_2_u256(vm.env.current_difficulty);
        ctx.gas_limit = string_2_u256(vm.env.current_gas_limit).low_u64();
        ctx.number = string_2_u256(vm.env.current_number);
        ctx.timestamp = string_2_u256(vm.env.current_timestamp).low_u64() as u64;
        ctx.origin = vm.exec.origin;
        ctx.gas_price = string_2_u256(vm.exec.gas_price);

        // Init Config
        let mut cfg = interpreter::InterpreterConf::default();
        cfg.gas_exp_byte = 10;
        cfg.gas_sload = 50;
        cfg.gas_self_destruct = 0;

        // Init params
        let mut params = interpreter::InterpreterParams::default();
        params.gas = 1000000;
        params.contract.code_address = vm.exec.address;
        params.address = vm.exec.address;
        params.sender = vm.exec.caller;
        if vm.exec.data.len() > 2 {
            params.input = string_2_bytes(vm.exec.data.clone());
        } else {
            params.input = Vec::new();
        }
        params.gas = string_2_u256(vm.exec.gas.clone()).low_u64();
        params.value = string_2_u256(vm.exec.value);
        params.contract.code_data = string_2_bytes(vm.exec.code);

        let mut it = interpreter::Interpreter::new(
            ctx,
            cfg,
            Box::new(extmock::DataProviderMock::new()),
            params,
        );

        // Init state db
        if let Some(data) = vm.pre {
            for (address, account) in data.into_iter() {
                for (k, v) in &account.storage {
                    it.data_provider.set_storage(
                        &address,
                        string_2_h256(k.clone()),
                        string_2_h256(v.clone()),
                    );
                    it.data_provider.set_storage_origin(
                        &address,
                        string_2_h256(k.clone()),
                        string_2_h256(v.clone()),
                    );
                }
            }
        }

        // Doit
        let r = it.run();

        match r {
            Ok(data) => {
                match data {
                    evm::interpreter::InterpreterResult::Normal(ret, gas, _)
                    | evm::interpreter::InterpreterResult::Revert(ret, gas, _) => {
                        // Check statedb
                        if let Some(data) = vm.post {
                            for (address, account) in data.into_iter() {
                                for (k, v) in &account.storage {
                                    assert_eq!(
                                        it.data_provider
                                            .get_storage(&address, &string_2_h256(k.to_string())),
                                        string_2_h256(v.to_string())
                                    );
                                }
                            }
                        }
                        // Check output
                        if let Some(data) = vm.out {
                            assert_eq!(ret, string_2_bytes(data))
                        }
                        // Check gas
                        if let Some(except_gas) = vm.gas {
                            assert_eq!(gas, string_2_u256(except_gas).low_u64())
                        }
                    }
                    _ => {}
                }
            }
            Err(_) => assert!(vm.gas.is_none() && vm.post.is_none() && vm.logs.is_none()),
        }

        let toc = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let d = toc - tic;
        io::stderr().write(format!(" {}\n", d).as_bytes()).unwrap();
    }
}

fn test_json_path(p: &str) {
    let info = fs::metadata(p).unwrap();
    if info.is_dir() {
        for entry in fs::read_dir(p).unwrap() {
            let entry = entry.unwrap();
            let p = entry.path();
            test_json_path(p.to_str().unwrap());
        }
    } else {
        test_json_file(p);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    for e in &args[1..] {
        test_json_path(e.as_str());
    }
}
