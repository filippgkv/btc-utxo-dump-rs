use super::fixtures::{COINS, OBFUSCATION_KEY};
use super::support::encode_varint;
use crate::bitcoin::chainstate::{deobfuscate_value, ChainStateKey};
use db_key::Key;

#[test]
fn deobfuscates_fixed_ciphertext_across_multiple_key_cycles() {
    for coin in COINS {
        let ciphertext = hex::decode(coin.obfuscated_hex).unwrap();
        let expected = hex::decode(coin.value_hex).unwrap();
        assert_eq!(
            deobfuscate_value(&ciphertext, &OBFUSCATION_KEY),
            expected,
            "{}",
            coin.name
        );
    }
}

#[test]
fn empty_and_zero_obfuscation_keys_preserve_the_value() {
    let value = hex::decode(COINS[0].value_hex).unwrap();
    assert_eq!(deobfuscate_value(&value, &[]), value);
    assert_eq!(deobfuscate_value(&value, &[0; 8]), value);
    assert!(deobfuscate_value(&[], &OBFUSCATION_KEY).is_empty());
}

#[test]
fn decodes_txid_byte_order_and_output_index_boundaries() {
    let expected_txid = "1f1e1d1c1b1a191817161514131211100f0e0d0c0b0a09080706050403020100";

    for vout in [0, 127, 128, u32::MAX] {
        let mut bytes = vec![b'C'];
        bytes.extend(0..32);
        bytes.extend(encode_varint(u64::from(vout)));
        let key = ChainStateKey::from_u8(&bytes);
        assert_eq!(key.vout().unwrap(), vout);
        assert_eq!(key.txid().unwrap(), expected_txid);
    }
}

#[test]
fn malformed_keys_return_errors() {
    assert_eq!(ChainStateKey::from_u8(&[]).first_byte(), None);
    for bytes in [vec![], vec![b'C'], vec![b'X'; 34]] {
        let key = ChainStateKey::from_u8(&bytes);
        assert!(key.txid().is_err());
        assert!(key.vout().is_err());
    }
    let invalid_output_indices = [
        ("missing output index", ""),
        ("truncated output index", "80"),
        ("trailing bytes", "0000"),
        ("value exceeds u64", "ffffffffffffffffffffff"),
        ("value exceeds u32", "8efefeff00"),
    ];

    for (reason, encoded_hex) in invalid_output_indices {
        let mut bytes = vec![b'C'];
        bytes.extend([0; 32]);
        bytes.extend(hex::decode(encoded_hex).unwrap());
        let key = ChainStateKey::from_u8(&bytes);
        assert!(key.vout().is_err(), "{reason}");
    }
}
