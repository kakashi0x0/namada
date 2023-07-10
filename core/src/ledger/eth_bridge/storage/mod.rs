//! Functionality for accessing the storage subspace
pub mod bridge_pool;
pub mod wrapped_erc20s;

use super::ADDRESS;
use crate::ledger::parameters::storage::*;
use crate::ledger::parameters::ADDRESS as PARAM_ADDRESS;
use crate::types::address::Address;
use crate::types::storage::{Key, KeySeg};
use crate::types::token::balance_key;

/// Key prefix for the storage subspace
pub fn prefix() -> Key {
    Key::from(ADDRESS.to_db_key())
}

/// Key for storing the initial Ethereum block height when
/// events will first be extracted from.
pub fn eth_start_height_key() -> Key {
    get_eth_start_height_key_at_addr(PARAM_ADDRESS)
}

/// The key to the escrow of the VP.
pub fn escrow_key(nam_addr: &Address) -> Key {
    balance_key(nam_addr, &ADDRESS)
}

/// Returns whether a key belongs to this account or not
pub fn is_eth_bridge_key(nam_addr: &Address, key: &Key) -> bool {
    key == &escrow_key(nam_addr)
        || matches!(key.segments.get(0), Some(first_segment) if first_segment == &ADDRESS.to_db_key())
}

/// A key for storing the active / inactive status
/// of the Ethereum bridge.
pub fn active_key() -> Key {
    get_active_status_key_at_addr(PARAM_ADDRESS)
}

/// Storage key for the minimum confirmations parameter.
pub fn min_confirmations_key() -> Key {
    get_min_confirmations_key_at_addr(PARAM_ADDRESS)
}

/// Storage key for the Ethereum address of wNam.
pub fn native_erc20_key() -> Key {
    get_native_erc20_key_at_addr(PARAM_ADDRESS)
}

/// Storage key for the Ethereum address of the bridge contract.
pub fn bridge_contract_key() -> Key {
    get_bridge_contract_address_key_at_addr(PARAM_ADDRESS)
}

/// Storage key for the Ethereum address of the governance contract.
pub fn governance_contract_key() -> Key {
    get_governance_contract_address_key_at_addr(PARAM_ADDRESS)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::address;
    use crate::types::address::nam;

    #[test]
    fn test_is_eth_bridge_key_returns_true_for_eth_bridge_address() {
        let key = Key::from(super::ADDRESS.to_db_key());
        assert!(is_eth_bridge_key(&nam(), &key));
    }

    #[test]
    fn test_is_eth_bridge_key_returns_true_for_eth_bridge_subkey() {
        let key = Key::from(super::ADDRESS.to_db_key())
            .push(&"arbitrary key segment".to_owned())
            .expect("Could not set up test");
        assert!(is_eth_bridge_key(&nam(), &key));
    }

    #[test]
    fn test_is_eth_bridge_key_returns_false_for_different_address() {
        let key =
            Key::from(address::testing::established_address_1().to_db_key());
        assert!(!is_eth_bridge_key(&nam(), &key));
    }

    #[test]
    fn test_is_eth_bridge_key_returns_false_for_different_address_subkey() {
        let key =
            Key::from(address::testing::established_address_1().to_db_key())
                .push(&"arbitrary key segment".to_owned())
                .expect("Could not set up test");
        assert!(!is_eth_bridge_key(&nam(), &key));
    }
}