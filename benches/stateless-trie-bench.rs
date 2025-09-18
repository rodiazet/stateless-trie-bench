use std::hint::black_box;
use alloy_primitives::{Address, FixedBytes};
use criterion::{criterion_group, criterion_main, Criterion};
use guest_libs::mpt::SparseState;
use reth_stateless::StatelessTrie;
use reth_stateless::trie::StatelessSparseTrie;
use stateless_trie_bench::{get_state_root, load_execution_witness};

static TEST_FILE: &str = "test_data/mainnet_block_164E2F4_test.json";

fn benchmark_stateless_trie_create<T: StatelessTrie>(c: &mut Criterion) {
    let witness = load_execution_witness(&String::from(TEST_FILE)).clone();
    let state_root = get_state_root(&witness);

    let trie = T::new(&witness, state_root);
    assert!(trie.is_ok());

    c.bench_function(
        format!("new {}",
                std::any::type_name::<T>()).as_str(),
        |b| b.iter(|| T::new(black_box(&witness), black_box(state_root))));
}

fn benchmark_stateless_trie_account<T: StatelessTrie>(c: &mut Criterion) {
    let witness = load_execution_witness(&String::from(TEST_FILE)).clone();
    let state_root = get_state_root(&witness);
    let trie = T::new(&witness, state_root).unwrap().0;

    let addresses = witness.keys.iter().filter(|key| key.len() == 20);
    for address in addresses.clone() {
        assert!(trie.account(Address::from(FixedBytes::<20>::from_slice(address))).is_ok());
    }

    c.bench_function(
        format!("account {}", std::any::type_name::<T>()).as_str(),
        |b|
            b.iter(||
                for address in addresses.clone()
                {
                    let _ = trie.account(Address::from(FixedBytes::<20>::from_slice(address)));
                }
            )
    );
}

criterion_group!(benches,
    benchmark_stateless_trie_create::<SparseState>,
    benchmark_stateless_trie_create::<StatelessSparseTrie>,
    benchmark_stateless_trie_account::<SparseState>,
    benchmark_stateless_trie_account::<StatelessSparseTrie>
);
criterion_main!(benches);
