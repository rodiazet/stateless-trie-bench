use alloy_primitives::{hex, Address};
use criterion::{criterion_group, criterion_main, Criterion};
use guest_libs::mpt::SparseState;
use reth_stateless::StatelessTrie;
use reth_stateless::trie::StatelessSparseTrie;
use stateless_trie_bench::get_state_root;
use stateless_trie_bench::build_execution_witness;

fn benchmark_stateless_trie_create<T: StatelessTrie>(c: &mut Criterion) {
    let witness = build_execution_witness();
    let state_root = get_state_root(&witness);

    c.bench_function(format!("stateless trie create {}", std::any::type_name::<T>()).as_str(), |b| b.iter(|| T::new(&witness, state_root)));
}

fn benchmark_stateless_trie_account<T: StatelessTrie>(c: &mut Criterion) {
    let witness = build_execution_witness();
    let state_root = get_state_root(&witness);
    let trie = T::new(&witness, state_root).unwrap().0;

    c.bench_function(
        format!("stateless trie account {}", std::any::type_name::<T>()).as_str(),
        |b|
            b.iter(|| trie.account(Address::from(hex!("0xa94f5374fce5edbc8e2a8697c15331677e6ebf0b")))));
}


criterion_group!(benches,
    benchmark_stateless_trie_create::<SparseState>,
    benchmark_stateless_trie_account::<SparseState>,

    benchmark_stateless_trie_create::<StatelessSparseTrie>,
    benchmark_stateless_trie_account::<StatelessSparseTrie>
);
criterion_main!(benches);
