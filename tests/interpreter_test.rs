extern crate cita_vm;
extern crate evm;

use cita_vm::json_tests::common::*;
use std::fs;
use std::io;
use std::io::prelude::*;

fn test_json_file(p: &str) {
    let f = fs::File::open(p).unwrap();
    let t = cita_vm::json_tests::vm_test::Test::load(f).unwrap();
    for (name, data) in t.into_iter() {
        io::stderr()
            .write(format!("{}::{}\n", p, name).as_bytes())
            .unwrap();
        let vm: cita_vm::json_tests::vm_test::Vm = data;
        let mut it = evm::interpreter::Interpreter::default();
        // Init Config
        it.cfg.print_op = true;
        it.cfg.print_gas_used = true;
        it.cfg.gas_exp_byte = 10;
        it.cfg.gas_sload = 50;
        it.cfg.gas_self_destruct = 0;

        // Init context
        it.context.coinbase = vm.env.current_coinbase;
        it.context.difficulty = string_2_u256(vm.env.current_difficulty);
        it.context.gas_limit = string_2_u256(vm.env.current_gas_limit).low_u64();
        it.context.number = string_2_u256(vm.env.current_number);
        it.context.timestamp = string_2_u256(vm.env.current_timestamp).low_u64() as u64;
        it.context.origin = vm.exec.origin;
        it.context.gas_price = string_2_u256(vm.exec.gas_price);

        // Init txinfo
        it.code_address = vm.exec.address;
        it.address = vm.exec.address;
        it.sender = vm.exec.caller;
        if vm.exec.data.len() > 2 {
            it.input = string_2_bytes(vm.exec.data.clone());
        } else {
            it.input = Vec::new();
        }
        it.gas = string_2_u256(vm.exec.gas.clone()).low_u64();
        it.value = string_2_u256(vm.exec.value);

        // Init contract code
        it.code_data = string_2_bytes(vm.exec.code);

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

#[test]
fn test_vm() {
    test_json_path(r"./tests/jsondata/VMTests/vmArithmeticTest");
    test_json_path(r"./tests/jsondata/VMTests/vmBitwiseLogicOperation");
    test_json_path(r"./tests/jsondata/VMTests/vmBlockInfoTest");
    test_json_path(r"./tests/jsondata/VMTests/vmEnvironmentalInfo");
    test_json_path(r"./tests/jsondata/VMTests/vmIOandFlowOperations");
    test_json_path(r"./tests/jsondata/VMTests/vmLogTest");
    test_json_path(r"./tests/jsondata/VMTests/vmPushDupSwapTest");
    test_json_path(r"./tests/jsondata/VMTests/vmRandomTest");
    test_json_path(r"./tests/jsondata/VMTests/vmSha3Test");
    test_json_path(r"./tests/jsondata/VMTests/vmSystemOperations");
    test_json_path(r"./tests/jsondata/VMTests/vmTests");
}
