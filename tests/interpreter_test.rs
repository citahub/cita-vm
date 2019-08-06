use std::fs;
use std::io;
use std::io::Write;

use cita_vm::evm;
use cita_vm::evm::extmock;
use cita_vm::json_tests::common::*;
use cita_vm::{InterpreterParams, InterpreterResult};

fn test_json_file(p: &str) {
    let f = fs::File::open(p).unwrap();
    let t = cita_vm::json_tests::vm_test::Test::load(f).unwrap();
    for (name, data) in t.into_iter() {
        io::stderr().write_all(format!("{}::{}\n", p, name).as_bytes()).unwrap();
        let vm: cita_vm::json_tests::vm_test::Vm = data;
        // Init context
        let mut ctx = cita_vm::Context::default();
        ctx.coinbase = vm.env.current_coinbase;
        ctx.difficulty = string_2_u256(vm.env.current_difficulty);
        ctx.gas_limit = string_2_u256(vm.env.current_gas_limit).low_u64();
        ctx.number = string_2_u256(vm.env.current_number);
        ctx.timestamp = string_2_u256(vm.env.current_timestamp).low_u64() as u64;

        // Init Config
        let mut cfg = evm::InterpreterConf::default();
        cfg.gas_exp_byte = 10;
        cfg.gas_sload = 50;
        cfg.gas_self_destruct = 0;
        cfg.gas_self_destruct_new_account = 0;

        // Init params
        let mut params = InterpreterParams::default();
        params.origin = vm.exec.origin;
        params.contract.code_address = vm.exec.address;
        params.address = vm.exec.address;
        params.sender = vm.exec.caller;
        if vm.exec.data.len() > 2 {
            params.input = string_2_bytes(vm.exec.data.clone());
        } else {
            params.input = Vec::new();
        }
        params.gas_limit = string_2_u256(vm.exec.gas.clone()).low_u64();
        params.gas_price = string_2_u256(vm.exec.gas_price);
        params.value = string_2_u256(vm.exec.value);
        params.contract.code_data = string_2_bytes(vm.exec.code);

        let mut it = cita_vm::evm::Interpreter::new(ctx, cfg, Box::new(extmock::DataProviderMock::default()), params);

        // Init state db
        if let Some(data) = vm.pre {
            for (address, account) in data.into_iter() {
                for (k, v) in &account.storage {
                    it.data_provider
                        .set_storage(&address, string_2_h256(k.clone()), string_2_h256(v.clone()));
                    it.data_provider
                        .set_storage_origin(&address, string_2_h256(k.clone()), string_2_h256(v.clone()));
                }
            }
        }

        // Doit
        let r = it.run();

        match r {
            Ok(data) => {
                match data {
                    InterpreterResult::Normal(ret, gas, _) | InterpreterResult::Revert(ret, gas) => {
                        // Check statedb
                        if let Some(data) = vm.post {
                            for (address, account) in data.into_iter() {
                                for (k, v) in &account.storage {
                                    assert_eq!(
                                        it.data_provider.get_storage(&address, &string_2_h256(k.to_string())),
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
                        if let Some(expected_gas) = vm.gas {
                            assert_eq!(gas, string_2_u256(expected_gas).low_u64())
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
    test_json_path(r"/tmp/jsondata/VMTests/vmArithmeticTest");
    test_json_path(r"/tmp/jsondata/VMTests/vmBitwiseLogicOperation");
    test_json_path(r"/tmp/jsondata/VMTests/vmBlockInfoTest");
    test_json_path(r"/tmp/jsondata/VMTests/vmEnvironmentalInfo");
    test_json_path(r"/tmp/jsondata/VMTests/vmIOandFlowOperations");
    test_json_path(r"/tmp/jsondata/VMTests/vmLogTest");
    test_json_path(r"/tmp/jsondata/VMTests/vmPushDupSwapTest");
    test_json_path(r"/tmp/jsondata/VMTests/vmRandomTest");
    test_json_path(r"/tmp/jsondata/VMTests/vmSha3Test");
    test_json_path(r"/tmp/jsondata/VMTests/vmSystemOperations");
    test_json_path(r"/tmp/jsondata/VMTests/vmTests");
}
