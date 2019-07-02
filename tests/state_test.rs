use std::fs;
use std::io;
use std::io::Write;
use std::sync::Arc;
use std::thread;

use cita_vm::json_tests::common::*;
use cita_vm::*;
use state::State;

fn test_json_file(p: &str) {
    let f = fs::File::open(p).unwrap();
    let t = cita_vm::json_tests::general_state_test::Test::load(f).unwrap();
    for (name, data) in t.into_iter() {
        let data_post_constantinople = data.post.unwrap().constantinople;
        if data_post_constantinople.is_none() {
            continue;
        }
        for (i, postdata) in data_post_constantinople.unwrap().iter().enumerate() {
            io::stderr()
                .write_all(format!("{}::{}::{}\n", p, name, i).as_bytes())
                .unwrap();
            let d = Arc::new(cita_trie::MemoryDB::new(false));
            let mut state_provider = State::new(d).unwrap();

            for (address, account) in data.pre.clone().unwrap() {
                let balance = string_2_u256(account.balance);
                let code = string_2_bytes(account.code);
                let nonce = string_2_u256(account.nonce);
                if code.is_empty() {
                    state_provider.new_contract(&address, balance, nonce, vec![]);
                } else {
                    state_provider.new_contract(&address, balance, nonce, code);
                }
                for (k, v) in account.storage {
                    let kk = string_2_h256(k);
                    let vv = string_2_h256(v);
                    state_provider.set_storage(&address, kk, vv).unwrap();
                }
            }
            state_provider.commit().unwrap();

            let idx_gas = &postdata.indexes[&String::from("gas")];
            let idx_value = &postdata.indexes[&String::from("value")];
            let idx_data = &postdata.indexes[&String::from("data")];

            let str_block_gas = data.env.current_gas_limit.clone();
            let str_gas = data.transaction.gas_limit.clone()[*idx_gas].clone();
            let str_value = data.transaction.value.clone()[*idx_value].clone();
            let str_data = data.transaction.data.clone()[*idx_data].clone();

            let evm_context = cita_vm::Context {
                gas_limit: string_2_u256(str_block_gas.clone()).low_u64(),
                coinbase: data.env.current_coinbase,
                number: string_2_u256(data.env.current_number.clone()),
                timestamp: string_2_u256(data.env.current_timestamp.clone()).low_u64(),
                difficulty: string_2_u256(data.env.current_difficulty.clone()),
            };
            let mut cfg = Config::default();
            cfg.block_gas_limit = string_2_u256(data.env.current_gas_limit.clone()).low_u64();
            let mut tx = Transaction {
                from: secret_2_address(data.transaction.secret_key.as_str()),
                to: None,
                value: string_2_u256(str_value),
                nonce: string_2_u256(data.transaction.nonce.clone()),
                gas_limit: string_2_u256(str_gas).low_u64(),
                gas_price: string_2_u256(data.transaction.gas_price.clone()),
                input: string_2_bytes(str_data),
                itype: InterpreterType::EVM,
            };
            if !data.transaction.to.is_empty() {
                tx.to = Some(string_2_address(data.transaction.to.clone()));
            }

            let exepinst = Executive::new(Arc::new(BlockDataProviderMock::default()), state_provider, cfg);
            let _ = exepinst.exec(evm_context, tx);
            let root = exepinst.commit().unwrap();
            assert_eq!(root, string_2_h256(postdata.hash.clone()));
        }
    }
}

fn test_json_path_skip(p: &str, skip: Vec<&'static str>) {
    let info = fs::metadata(p).unwrap();
    if info.is_dir() {
        for entry in fs::read_dir(p).unwrap() {
            let entry = entry.unwrap();
            let p = entry.path();
            if skip.contains(&entry.file_name().into_string().unwrap().as_str()) {
                continue;
            }
            test_json_path(p.to_str().unwrap());
        }
    } else {
        if p.ends_with("json") {
            test_json_file(p);
        }
    }
}

fn test_json_path(p: &str) {
    test_json_path_skip(p, vec![])
}

#[rustfmt::skip]
#[test]
fn test_state_pass() {
    thread::Builder::new().stack_size(134_217_728).spawn(move || {
        test_json_path("/tmp/jsondata/GeneralStateTests/stArgsZeroOneBalance");
        test_json_path("/tmp/jsondata/GeneralStateTests/stAttackTest");
        test_json_path("/tmp/jsondata/GeneralStateTests/stBadOpcode");
        test_json_path("/tmp/jsondata/GeneralStateTests/stBugs");
        test_json_path("/tmp/jsondata/GeneralStateTests/stCallCodes");
        test_json_path_skip("/tmp/jsondata/GeneralStateTests/stCallCreateCallCodeTest", vec![
            "createInitFailStackSizeLargerThan1024.json",
            "createInitFailStackUnderflow.json",
        ]);
        test_json_path("/tmp/jsondata/GeneralStateTests/stCallDelegateCodesCallCodeHomestead");
        test_json_path("/tmp/jsondata/GeneralStateTests/stCallDelegateCodesHomestead");
        test_json_path("/tmp/jsondata/GeneralStateTests/stChangedEIP150");
        test_json_path("/tmp/jsondata/GeneralStateTests/stCodeCopyTest");
        test_json_path("/tmp/jsondata/GeneralStateTests/stCodeSizeLimit");
        test_json_path_skip("/tmp/jsondata/GeneralStateTests/stCreate2", vec![
            "create2callPrecompiles.json",
        ]);
        test_json_path("/tmp/jsondata/GeneralStateTests/stCreateTest");
        test_json_path("/tmp/jsondata/GeneralStateTests/stDelegatecallTestHomestead");
        test_json_path("/tmp/jsondata/GeneralStateTests/stEIP150singleCodeGasPrices");
        test_json_path("/tmp/jsondata/GeneralStateTests/stEIP150Specific");
        test_json_path("/tmp/jsondata/GeneralStateTests/stEIP158Specific");
        test_json_path("/tmp/jsondata/GeneralStateTests/stExample");
        test_json_path("/tmp/jsondata/GeneralStateTests/stExtCodeHash");
        test_json_path("/tmp/jsondata/GeneralStateTests/stHomesteadSpecific");
        test_json_path("/tmp/jsondata/GeneralStateTests/stInitCodeTest");
        test_json_path("/tmp/jsondata/GeneralStateTests/stLogTests");
        test_json_path("/tmp/jsondata/GeneralStateTests/stMemExpandingEIP150Calls");
        test_json_path_skip("/tmp/jsondata/GeneralStateTests/stMemoryStressTest", vec![
            "FillStack.json",
            "JUMPI_Bounds.json",
            "JUMP_Bounds.json",
            "JUMP_Bounds2.json",
        ]);
        test_json_path_skip("/tmp/jsondata/GeneralStateTests/stMemoryTest", vec![
            "stackLimitGas_1023.json",
            "stackLimitGas_1024.json",
            "stackLimitGas_1025.json",
            "stackLimitPush31_1023.json",
            "stackLimitPush31_1024.json",
            "stackLimitPush31_1025.json",
            "stackLimitPush32_1023.json",
            "stackLimitPush32_1024.json",
            "stackLimitPush32_1025.json",
        ]);
        test_json_path("/tmp/jsondata/GeneralStateTests/stNonZeroCallsTest");
        test_json_path_skip("/tmp/jsondata/GeneralStateTests/stPreCompiledContracts", vec![
            "modexp.json",
            "modexp_0_0_0_1000000.json",
            "modexp_0_0_0_155000.json",
            "modexp_0_1_0_1000000.json",
            "modexp_0_1_0_155000.json",
            "modexp_0_1_0_20500.json",
            "modexp_0_1_0_22000.json",
            "modexp_0_1_0_25000.json",
            "modexp_0_1_0_35000.json",
            "modexp_0_3_100_1000000.json",
            "modexp_0_3_100_155000.json",
            "modexp_0_3_100_20500.json",
            "modexp_0_3_100_22000.json",
            "modexp_0_3_100_25000.json",
            "modexp_0_3_100_35000.json",
            "modexp_1_0_0_1000000.json",
            "modexp_1_0_0_155000.json",
            "modexp_1_0_0_20500.json",
            "modexp_1_0_0_22000.json",
            "modexp_1_0_0_25000.json",
            "modexp_1_0_0_35000.json",
            "modexp_1_0_1_1000000.json",
            "modexp_1_0_1_155000.json",
            "modexp_1_0_1_20500.json",
            "modexp_1_0_1_22000.json",
            "modexp_1_0_1_25000.json",
            "modexp_1_0_1_35000.json",
            "modexp_1_1_1_1000000.json",
            "modexp_1_1_1_155000.json",
            "modexp_1_1_1_20500.json",
            "modexp_1_1_1_22000.json",
            "modexp_1_1_1_25000.json",
            "modexp_1_1_1_35000.json",
            "modexp_37120_22411_22000.json",
            "modexp_37120_37111_0_1000000.json",
            "modexp_37120_37111_0_155000.json",
            "modexp_37120_37111_0_20500.json",
            "modexp_37120_37111_0_22000.json",
            "modexp_37120_37111_0_25000.json",
            "modexp_37120_37111_0_35000.json",
            "modexp_37120_37111_1_1000000.json",
            "modexp_37120_37111_1_155000.json",
            "modexp_37120_37111_1_20500.json",
            "modexp_37120_37111_1_25000.json",
            "modexp_37120_37111_1_35000.json",
            "modexp_37120_37111_37111_1000000.json",
            "modexp_37120_37111_37111_155000.json",
            "modexp_37120_37111_37111_20500.json",
            "modexp_37120_37111_37111_22000.json",
            "modexp_37120_37111_37111_25000.json",
            "modexp_37120_37111_37111_35000.json",
            "modexp_37120_37111_97_1000000.json",
            "modexp_37120_37111_97_155000.json",
            "modexp_37120_37111_97_20500.json",
            "modexp_37120_37111_97_22000.json",
            "modexp_37120_37111_97_25000.json",
            "modexp_37120_37111_97_35000.json",
            "modexp_39936_1_55201_1000000.json",
            "modexp_39936_1_55201_155000.json",
            "modexp_39936_1_55201_20500.json",
            "modexp_39936_1_55201_22000.json",
            "modexp_39936_1_55201_25000.json",
            "modexp_39936_1_55201_35000.json",
            "modexp_3_09984_39936_1000000.json",
            "modexp_3_09984_39936_155000.json",
            "modexp_3_09984_39936_22000.json",
            "modexp_3_09984_39936_25000.json",
            "modexp_3_09984_39936_35000.json",
            "modexp_3_28948_11579_20500.json",
            "modexp_3_5_100_1000000.json",
            "modexp_3_5_100_155000.json",
            "modexp_3_5_100_20500.json",
            "modexp_3_5_100_22000.json",
            "modexp_3_5_100_25000.json",
            "modexp_3_5_100_35000.json",
            "modexp_49_2401_2401_1000000.json",
            "modexp_49_2401_2401_155000.json",
            "modexp_49_2401_2401_20500.json",
            "modexp_49_2401_2401_22000.json",
            "modexp_49_2401_2401_25000.json",
            "modexp_49_2401_2401_35000.json",
            "modexp_55190_55190_42965_1000000.json",
            "modexp_55190_55190_42965_155000.json",
            "modexp_55190_55190_42965_20500.json",
            "modexp_55190_55190_42965_22000.json",
            "modexp_55190_55190_42965_25000.json",
            "modexp_55190_55190_42965_35000.json",
            "modexp_9_37111_37111_1000000.json",
            "modexp_9_37111_37111_155000.json",
            "modexp_9_37111_37111_20500.json",
            "modexp_9_37111_37111_22000.json",
            "modexp_9_37111_37111_35000.json",
            "modexp_9_3711_37111_25000.json",
        ]);
        test_json_path_skip("/tmp/jsondata/GeneralStateTests/stPreCompiledContracts2", vec![
            "modexpRandomInput.json",
            "modexp_0_0_0_20500.json",
            "modexp_0_0_0_22000.json",
            "modexp_0_0_0_25000.json",
            "modexp_0_0_0_35000.json",
        ]);
        test_json_path_skip("/tmp/jsondata/GeneralStateTests/stQuadraticComplexityTest", vec![
            "Call1MB1024Calldepth.json",
            "Call50000.json",
            "Call50000bytesContract50_1.json",
            "Call50000bytesContract50_2.json",
            "Call50000bytesContract50_3.json",
            "Call50000_ecrec.json",
            "Call50000_rip160.json",
            "Call50000_sha256.json",
            "Callcode50000.json",
            "Create1000.json",
            "Create1000Byzantium.json",
            "QuadraticComplexitySolidity_CallDataCopy.json",
            "Return50000.json",
            "Return50000_2.json",
        ]);
        test_json_path_skip("/tmp/jsondata/GeneralStateTests/stRandom", vec![
            "randomStatetest307.json",
        ]);
        test_json_path_skip("/tmp/jsondata/GeneralStateTests/stRandom2", vec![
            "randomStatetest618.json",
            "randomStatetest646.json",
        ]);
        test_json_path("/tmp/jsondata/GeneralStateTests/stRecursiveCreate");
        test_json_path("/tmp/jsondata/GeneralStateTests/stRefundTest");
        test_json_path_skip("/tmp/jsondata/GeneralStateTests/stReturnDataTest", vec![
            "modexp_modsize0_returndatasize.json",
        ]);
        test_json_path("/tmp/jsondata/GeneralStateTests/stRevertTest");
        test_json_path("/tmp/jsondata/GeneralStateTests/stShift");
        test_json_path("/tmp/jsondata/GeneralStateTests/stSolidityTest");
        test_json_path("/tmp/jsondata/GeneralStateTests/stSpecialTest");
        test_json_path_skip("/tmp/jsondata/GeneralStateTests/stSStoreTest", vec![
            "sstore_combinations_initial0.json",
            "sstore_combinations_initial1.json",
            "sstore_combinations_initial2.json",
        ]);
        test_json_path_skip("/tmp/jsondata/GeneralStateTests/stStackTests", vec![
            "stackOverflowM1DUP.json",
            "stackOverflowM1PUSH.json",
        ]);
        test_json_path_skip("/tmp/jsondata/GeneralStateTests/stStaticCall", vec![
            "static_Call50000_rip160.json",
            "static_Call50000_sha256.json",
            "static_CallEcrecover0_0input.json",
            "static_Call1MB1024Calldepth.json",
            "static_Call50000_ecrec.json",
            "static_Call50000_identity.json",
        ]);
        test_json_path("/tmp/jsondata/GeneralStateTests/stSystemOperationsTest");
        test_json_path("/tmp/jsondata/GeneralStateTests/stTransactionTest");
        test_json_path("/tmp/jsondata/GeneralStateTests/stTransitionTest");
        test_json_path("/tmp/jsondata/GeneralStateTests/stWalletTest");
        test_json_path("/tmp/jsondata/GeneralStateTests/stZeroCallsRevert");
        test_json_path("/tmp/jsondata/GeneralStateTests/stZeroCallsTest");
    }).unwrap().join().unwrap();
}
