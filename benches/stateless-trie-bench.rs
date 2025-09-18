use alloy_primitives::{Address, FixedBytes};
use criterion::{Criterion, criterion_group, criterion_main};
use guest_libs::mpt::SparseState;
use reth_stateless::StatelessTrie;
use reth_stateless::trie::StatelessSparseTrie;
use stateless_trie_bench::{
    build_storage_hash_map, get_state_root, init_trie, load_execution_witness,
};

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

criterion_group!(
    benches,
    benchmark_stateless_trie_create::<SparseState>,
    benchmark_stateless_trie_create::<StatelessSparseTrie>,
    benchmark_stateless_trie_account::<SparseState>,
    benchmark_stateless_trie_account::<StatelessSparseTrie>,
    benchmark_stateless_trie_storage::<SparseState>,
    benchmark_stateless_trie_storage::<StatelessSparseTrie>
);
criterion_main!(benches);
