use alloy_genesis::ChainConfig;
use alloy_consensus::Header;
use alloy_primitives::{Address, B256, FixedBytes, U256, keccak256};
use reth_ethereum_primitives::Block;
use reth_primitives_traits::SealedHeader;
use reth_stateless::validation::StatelessValidationError;
use reth_stateless::{ExecutionWitness, StatelessTrie};
use std::collections::HashMap;
use std::env;

// /// `StatelessInput` is a convenience structure for serializing the input needed
// /// for the stateless validation function.
// #[serde_with::serde_as]
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct StatelessInput {
    /// The block being executed in the stateless validation function
    // #[serde_as(
    //     as = "reth_primitives_traits::serde_bincode_compat::Block<reth_ethereum_primitives::TransactionSigned, alloy_consensus::Header>"
    // )]
    pub block: Block,
    /// `ExecutionWitness` for the stateless validation function
    pub witness: ExecutionWitness,
    pub chain_config: ChainConfig,
}

pub fn get_test_file_path() -> String {
    let args: Vec<String> = env::args().collect();
    assert!(args.len() == 2);
    String::from(args[1].as_str())
}

pub fn load_stateless_input(path_str: &String) -> StatelessInput {
    serde_json::from_reader::<_, StatelessInput>(std::fs::File::open(path_str).unwrap()).unwrap()
}

pub fn load_execution_witness(path_str: &String) -> ExecutionWitness {
    serde_json::from_reader::<_, ExecutionWitness>(std::fs::File::open(path_str).unwrap()).unwrap()
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
        .collect::<Result<_, _>>()
        .unwrap();
    // Sort the headers by their block number to ensure that they are in
    // ascending order.
    ancestor_headers.sort_by_key(|header| header.number);

    // There should be at least one ancestor header.
    // The edge case here would be the genesis block, but we do not create proofs for the genesis
    // block.
    let parent = match ancestor_headers.last() {
        Some(prev_header) => prev_header,
        None => panic!("Parent not in ancestor headers"),
    };

    parent.state_root
}

pub fn init_trie<T: StatelessTrie>(witness: &ExecutionWitness) -> impl StatelessTrie {
    let state_root = get_state_root(&witness);
    T::new(&witness, state_root).unwrap().0
}

pub fn build_storage_hash_map(witness: &ExecutionWitness) -> HashMap<Address, Vec<U256>> {
    use std::collections::HashMap;

    let mut storage = HashMap::new();
    if witness.keys.len() > 0 {
        let mut current_address = Address::default();
        for k in witness.keys.iter() {
            if k.len() == 20 {
                current_address = Address::from(FixedBytes::<20>::from_slice(&k));
                storage.insert(current_address, vec![]);
            } else if k.len() == 32 {
                match storage.get_mut(&current_address) {
                    None => {
                        panic!("Account not found");
                    }
                    Some(value) => {
                        value.push(U256::from_be_slice(k.as_ref()));
                    }
                }
            } else {
                panic!("Invalid key length");
            }
        }
    }
    storage
}

#[cfg(test)]
mod tests {
    static TEST_FILE: &str = "test_data/test.json";
    
    use super::*;
    use sparsestate::SparseState;
    use reth_stateless::trie::StatelessSparseTrie;
    #[test]
    fn state_less_trie_storage_access() {
        let witness = load_execution_witness(&String::from(TEST_FILE));
        let storage = build_storage_hash_map(&witness);
        let trie = init_trie::<SparseState>(&witness);

        for (address, slots) in storage.iter() {
            if trie.account(address.clone()).unwrap().is_some() {
                eprintln!("{}", address.clone());
                for slot in slots {
                    eprintln!("{}", slot.clone());
                    assert!(trie.storage(address.clone(), slot.clone()).is_ok());
                }
            }
        }
    }

    #[test]
    fn state_less_sparse_trie_storage_access() {
        let witness = load_execution_witness(&String::from(TEST_FILE));
        let storage = build_storage_hash_map(&witness);
        let trie = init_trie::<StatelessSparseTrie>(&witness);

        for (address, slots) in storage.iter() {
            if trie.account(address.clone()).unwrap().is_some() {
                eprintln!("{}", address.clone());
                for slot in slots {
                    eprintln!("{}", slot.clone());
                    assert!(trie.storage(address.clone(), slot.clone()).is_ok());
                }
            }
        }

    }
}
