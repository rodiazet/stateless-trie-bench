use std::env;
use alloy_consensus::Header;
use alloy_primitives::{keccak256, B256};
use reth_primitives_traits::SealedHeader;
use reth_stateless::validation::StatelessValidationError;
use alloy_consensus::BlockHeader;
use reth_stateless::{ExecutionWitness, StatelessInput};

pub fn get_test_file_path() -> String {
    let args: Vec<String> = env::args().collect();
    assert!(args.len() == 2);
    String::from(args[1].as_str())
}

pub fn load_stateless_input(path_str: &String) -> StatelessInput {
    serde_json::from_reader::<_, StatelessInput>(std::fs::File::open(path_str).unwrap()).unwrap()
}

pub fn get_state_root(witness: &ExecutionWitness) -> B256 {
    let mut ancestor_headers: Vec<_> = witness
        .headers
        .iter()
        .map(|bytes| {
            let hash = keccak256(bytes);
            alloy_rlp::decode_exact::<Header>(bytes)
                .map(|h| SealedHeader::new(h, hash))
                .map_err(|_| StatelessValidationError::HeaderDeserializationFailed)
        })
        .collect::<Result<_, _>>().unwrap();
    // Sort the headers by their block number to ensure that they are in
    // ascending order.
    ancestor_headers.sort_by_key(|header| header.number());


    // There should be at least one ancestor header.
    // The edge case here would be the genesis block, but we do not create proofs for the genesis
    // block.
    let parent = match ancestor_headers.last() {
        Some(prev_header) => prev_header,
        None => panic!("Parent not in ancestor headers"),
    };

    parent.state_root
}