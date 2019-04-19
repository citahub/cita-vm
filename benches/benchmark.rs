use bencher::{benchmark_group, benchmark_main, Bencher};
use ethereum_types::{Address, U256};
use std::cell::RefCell;
use std::sync::Arc;

fn wrapper(bench: &mut Bencher, code: &str, data: &str) {
    let db = Arc::new(cita_vm::state::MemoryDB::new(false));
    let mut state = cita_vm::state::State::new(db).unwrap();
    state.new_contract(
        &Address::from("0xBd770416a3345F91E4B34576cb804a576fa48EB1"),
        U256::from(10),
        U256::from(1),
        hex::decode(code).unwrap(),
    );
    state.new_contract(
        &Address::from("0x1000000000000000000000000000000000000000"),
        U256::from(1_000_000_000_000_000u64),
        U256::from(1),
        vec![],
    );
    let block_data_provider = Arc::new(cita_vm::BlockDataProviderMock::default());
    let state_data_provider = Arc::new(RefCell::new(state));
    let context = cita_vm::evm::Context::default();
    let config = cita_vm::Config::default();

    let mut tx = cita_vm::Transaction {
        from: Address::from("0x1000000000000000000000000000000000000000"),
        to: Some(Address::from("0xBd770416a3345F91E4B34576cb804a576fa48EB1")),
        value: U256::from(0),
        nonce: U256::from(1),
        gas_limit: 80000,
        gas_price: U256::from(1),
        input: hex::decode(data).unwrap(),
    };

    bench.iter(|| {
        cita_vm::exec(
            block_data_provider.clone(),
            state_data_provider.clone(),
            context.clone(),
            config.clone(),
            tx.clone(),
        )
        .unwrap();
        tx.nonce += U256::one();
    });
}

/// Benchmark test for ./c.sol
fn bench_count(bench: &mut Bencher) {
    let code =
        "6080604052348015600f57600080fd5b5060043610604f576000357c01000000000000000000000000000000000000000000000\
         0000000000090048063119fbbd4146054578063c3da42b814605c575b600080fd5b605a6078565b005b6062608a565b60405180\
         82815260200191505060405180910390f35b60016000808282540192505081905550565b6000548156fea165627a7a72305820b\
         103212493f5223caaefa1174d99e347c1b108e57075d082c80bcbc003b7822e0029";
    let data = "119fbbd4";
    wrapper(bench, code, data);
}

benchmark_group!(benches, bench_count);
benchmark_main!(benches);
