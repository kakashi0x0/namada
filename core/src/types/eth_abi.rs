//! This module defines encoding methods compatible with Ethereum
//! smart contracts.

use std::marker::PhantomData;

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
#[doc(inline)]
pub use ethabi::token::Token;

use crate::proto::{Signable, SignableEthMessage};
use crate::types::keccak::{keccak_hash, KeccakHash};

/// A container for data types that are able to be Ethereum ABI-encoded.
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, BorshSchema)]
#[repr(transparent)]
pub struct EncodeCell<T: ?Sized> {
    /// ABI-encoded value of type `T`.
    encoded_data: Vec<u8>,
    /// Indicate we do not own values of type `T`.
    ///
    /// Passing `PhantomData<T>` here would trigger the drop checker,
    /// which is not the desired behavior, since we own an encoded value
    /// of `T`, not a value of `T` itself.
    _marker: PhantomData<*const T>,
}

impl<T> AsRef<[u8]> for EncodeCell<T> {
    fn as_ref(&self) -> &[u8] {
        &self.encoded_data
    }
}

impl<T> ::std::cmp::Eq for EncodeCell<T> {}

impl<T> ::std::cmp::PartialEq for EncodeCell<T> {
    fn eq(&self, other: &Self) -> bool {
        self.encoded_data == other.encoded_data
    }
}

impl<T> ::std::cmp::PartialOrd for EncodeCell<T> {
    fn partial_cmp(&self, other: &Self) -> Option<::std::cmp::Ordering> {
        self.encoded_data.partial_cmp(&other.encoded_data)
    }
}

impl<T> ::std::cmp::Ord for EncodeCell<T> {
    fn cmp(&self, other: &Self) -> ::std::cmp::Ordering {
        self.encoded_data.cmp(&other.encoded_data)
    }
}

impl<T> EncodeCell<T> {
    /// Return a new ABI encoded value of type `T`.
    pub fn new<const N: usize>(value: &T) -> Self
    where
        T: Encode<N>,
    {
        let encoded_data = {
            let tokens = value.tokenize();
            ethabi::encode(tokens.as_slice())
        };
        Self {
            encoded_data,
            _marker: PhantomData,
        }
    }

    /// Here the type information is not compiler deduced,
    /// proceed with caution!
    pub fn new_from<const N: usize>(tokens: [Token; N]) -> Self {
        Self {
            encoded_data: ethabi::encode(&tokens),
            _marker: PhantomData,
        }
    }

    /// Return the underlying ABI encoded value.
    pub fn into_inner(self) -> Vec<u8> {
        self.encoded_data
    }
}

/// Contains a method to encode data to a format compatible with Ethereum.
pub trait Encode<const N: usize>: Sized {
    /// Encodes a struct into a sequence of ABI
    /// [`Token`] instances.
    fn tokenize(&self) -> [Token; N];

    /// Returns the encoded [`Token`] instances, in a type-safe enclosure.
    fn encode(&self) -> EncodeCell<Self> {
        EncodeCell::new(self)
    }

    /// Encodes a slice of [`Token`] instances, and returns the
    /// keccak hash of the encoded string.
    fn keccak256(&self) -> KeccakHash {
        keccak_hash(self.encode().into_inner().as_slice())
    }

    /// Encodes a slice of [`Token`] instances, and returns the
    /// keccak hash of the encoded string appended to an Ethereum
    /// signature header. This can then be signed.
    fn signable_keccak256(&self) -> KeccakHash {
        let message = self.keccak256();
        SignableEthMessage::as_signable(&message)
    }
}

/// Represents an Ethereum encoding method equivalent
/// to `abi.encode`.
pub type AbiEncode<const N: usize> = [Token; N];

impl<const N: usize> Encode<N> for AbiEncode<N> {
    #[inline]
    fn tokenize(&self) -> [Token; N] {
        self.clone()
    }
}

// TODO: test signatures here once we merge secp keys
#[cfg(test)]
mod tests {
    use std::convert::TryInto;
    use std::str::FromStr;

    use data_encoding::HEXLOWER;
    use ethabi::ethereum_types::U256;
    use tiny_keccak::{Hasher, Keccak};
    use crate::types::address::Address;
    use crate::types::eth_bridge_pool::{GasFee, PendingTransfer, TransferToEthereum};

    use super::*;
    use crate::types::ethereum_events::EthAddress;
    use crate::types::vote_extensions::validator_set_update::ValidatorSetArgs;

    /// Checks if we get the same result as `abi.encode`, for some given
    /// input data.
    #[test]
    fn test_abi_encode() {
        let expected = "0x000000000000000000000000000000000000000000000000000000000000002a000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000047465737400000000000000000000000000000000000000000000000000000000";
        let expected = HEXLOWER
            .decode(&expected.as_bytes()[2..])
            .expect("Test failed");
        let got = AbiEncode::encode(&[
            Token::Uint(U256::from(42u64)),
            Token::String("test".into()),
        ]);
        assert_eq!(expected, got.into_inner());
    }

    /// Sanity check our keccak hash implementation.
    #[test]
    fn test_keccak_hash_impl() {
        let expected =
            "1c8aff950685c2ed4bc3174f3472287b56d9517b9c948127319a09a7a36deac8";
        assert_eq!(
            expected,
            &HEXLOWER.encode(
                &{
                    let mut st = Keccak::v256();
                    let mut output = [0; 32];
                    st.update(b"hello");
                    st.finalize(&mut output);
                    output
                }[..]
            )
        );
    }

    /// Test that the methods for converting a keccak hash to/from
    /// a string type are inverses.
    #[test]
    fn test_hex_roundtrip() {
        let original =
            "1C8AFF950685C2ED4BC3174F3472287B56D9517B9C948127319A09A7A36DEAC8";
        let keccak_hash: KeccakHash = original.try_into().expect("Test failed");
        assert_eq!(keccak_hash.to_string().as_str(), original);
    }

    #[test]
    fn test_abi_encode_address() {
        let address =
            EthAddress::from_str("0xF0457e703bf0B9dEb1a6003FFD71C77E44575f95")
                .expect("Test failed");
        let expected = "0x000000000000000000000000f0457e703bf0b9deb1a6003ffd71c77e44575f95";
        let expected = HEXLOWER
            .decode(&expected.as_bytes()[2..])
            .expect("Test failed");
        let encoded = ethabi::encode(&[Token::Address(address.0.into())]);
        assert_eq!(expected, encoded);
    }

    #[test]
    fn test_abi_encode_valset_args() {
        let valset_update = ValidatorSetArgs {
            validators: vec![
                EthAddress::from_str(
                    "0x241D37B7Cf5233b3b0b204321420A86e8f7bfdb5",
                )
                    .expect("Test failed"),
            ],
            voting_powers: vec![8828299.into()],
            epoch: 0.into(),
        };
        let encoded = valset_update.encode().into_inner();
        let encoded = HEXLOWER.encode(&encoded);
        let expected = "00000000000000000000000000000000000000000000000000000000000000200000\
        00000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000\
        000000000000000000000000000000a0000000000000000000000000000000000000000000000000000000000000\
        00000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000\
        241d37b7cf5233b3b0b204321420a86e8f7bfdb50000000000000000000000000000000000000000000000000000\
        000000000001000000000000000000000000000000000000000000000000000000000086b58b";
        assert_eq!(expected, encoded);
    }

    #[test]
    fn test_abi_encode_pending_transfer() {
        let transfer = PendingTransfer {
            transfer: TransferToEthereum {
                asset: EthAddress::from_str("0x3949c97925e5Aa13e34ddb18EAbf0B70ABB0C7d4")
                    .expect("Test failed"),
                recipient: EthAddress::from_str("0x3949c97925e5Aa13e34ddb18EAbf0B70ABB0C7d4")
                    .expect("Test failed"),
                sender: Address::decode("atest1v4ehgw36xvcyyvejgvenxs34g3zygv3jxqunjd6rxyeyys3sxy6rwvfkx4qnj33hg9qnvse4lsfctw")
                    .expect("Test failed"),
                amount: 76.into(),
            },
            gas_fee: GasFee {
                amount: Default::default(),
                payer: Address::decode("atest1v4ehgw36xvcyyvejgvenxs34g3zygv3jxqunjd6rxyeyys3sxy6rwvfkx4qnj33hg9qnvse4lsfctw")
                    .expect("Test failed")
            },
        };
    }
}
