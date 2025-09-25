use std::sync::Arc;
use std::time::Duration;
use alloy_primitives::{Address, FixedBytes};
use criterion::{Criterion, criterion_group, criterion_main};
use guest_libs::mpt::SparseState;
use guest_libs::senders::recover_block;
use reth_chainspec::ChainSpec;
use reth_evm_ethereum::EthEvmConfig;
use reth_stateless::{stateless_validation_with_trie, Genesis, StatelessTrie};
use reth_stateless::trie::StatelessSparseTrie;
use stateless_trie_bench::{build_storage_hash_map, get_state_root, get_test_file_path, init_trie, load_execution_witness, load_stateless_input};

static TEST_FILE: &str = "test_data/mainnet_block_164E2F4_test.json";

fn benchmark_stateless_trie_create<T: StatelessTrie>(c: &mut Criterion) {
    let witness = load_execution_witness(&String::from(TEST_FILE));
    let state_root = get_state_root(&witness);

    assert!(T::new(&witness, state_root).is_ok());

    c.bench_function(
        format!("new {}", std::any::type_name::<T>()).as_str(),
        |b| b.iter(|| T::new(&witness, state_root)),
    );
}

fn benchmark_stateless_trie_account<T: StatelessTrie>(c: &mut Criterion) {
    let witness = load_execution_witness(&String::from(TEST_FILE));
    let state_root = get_state_root(&witness);
    let trie = T::new(&witness, state_root).unwrap().0;

    let addresses: Vec<Address> = witness
        .keys
        .iter()
        .filter(|key| key.len() == 20)
        .map(|key| Address::from(FixedBytes::<20>::from_slice(key)))
        .collect();

    for address in addresses.iter() {
        assert!(trie.account(address.clone()).is_ok());
    }

    c.bench_function(
        format!("account {}", std::any::type_name::<T>()).as_str(),
        |b| {
            b.iter(|| {
                for address in addresses.iter() {
                    let _ = trie.account(address.clone()).unwrap();
                }
            })
        },
    );
}

fn benchmark_stateless_trie_storage<T: StatelessTrie>(c: &mut Criterion) {
    let witness = load_execution_witness(&String::from(TEST_FILE));
    let storage = build_storage_hash_map(&witness);
    let trie = init_trie::<T>(&witness);

    c.bench_function(
        format!("account and storage {}", std::any::type_name::<T>()).as_str(),
        |b| {
            b.iter(|| {
                for (address, slots) in storage.iter() {
                    // `account` must be called before `storage`
                    let _ = trie.account(address.clone()).unwrap();
                    for slot in slots {
                        let _ = trie.storage(address.clone(), slot.clone()).unwrap();
                    }
                }
            })
        },
    );
}

fn benchmark_stateless_validation<T: StatelessTrie>(c: &mut Criterion) {
    let input = load_stateless_input(&String::from("test_data/rpc_block_23439901.json"));

    let genesis = Genesis {
        config: input.chain_config.clone(),
        ..Default::default()
    };
    let chain_spec: Arc<ChainSpec> = Arc::new(genesis.into());
    let recovered_block = recover_block(input.block, &chain_spec).unwrap();
    let evm_config = EthEvmConfig::new(chain_spec.clone());

    c.bench_function(
        format!("stateless validation {}", std::any::type_name::<T>()).as_str(),
        |b| {
            b.iter(|| {
                let r = stateless_validation_with_trie::<T, _, _>(
                    recovered_block.clone(),
                    input.witness.clone(),
                    chain_spec.clone(),
                    evm_config.clone(),
                );

                if r.is_err() {
                    panic!("Error")
                }
            })
        },
    );
}

criterion_group!(
    name = benches;
    config = Criterion::default().measurement_time(Duration::new(10, 00));
    targets = 
    // benchmark_stateless_trie_create::<SparseState>,
    // benchmark_stateless_trie_create::<StatelessSparseTrie>,
    // benchmark_stateless_trie_account::<SparseState>,
    // benchmark_stateless_trie_account::<StatelessSparseTrie>,
    // benchmark_stateless_trie_storage::<SparseState>,
    // benchmark_stateless_trie_storage::<StatelessSparseTrie>,
    benchmark_stateless_validation::<SparseState>,
    benchmark_stateless_validation::<StatelessSparseTrie>
);
criterion_main!(benches);
