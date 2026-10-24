use std::borrow::Cow;

use super::support::encode_varint;
use crate::bitcoin::utxo::{ScriptType, UtxoValue};

fn encode_utxo(code: u64, amount: u64, nsize: u64, script: &[u8]) -> Vec<u8> {
    let mut bytes = encode_varint(code);
    bytes.extend(encode_varint(amount));
    bytes.extend(encode_varint(nsize));
    bytes.extend_from_slice(script);
    bytes
}

fn compressed_public_key() -> Vec<u8> {
    hex::decode("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798").unwrap()
}

fn parse_raw_script(script: &[u8]) -> UtxoValue<'static> {
    UtxoValue {
        height: 0,
        coinbase: false,
        amount: 1,
        nsize: script.len() as u64 + 6,
        script: Cow::Owned(script.to_vec()),
    }
}

#[test]
fn parses_special_script_encodings_with_optional_pubkey_decompression() {
    let key = compressed_public_key();
    let hash = [0x11; 20];

    for nsize in 0..=5 {
        let payload = if nsize < 2 {
            hash.as_slice()
        } else {
            &key[1..]
        };
        for decompress in [false, true] {
            let bytes = encode_utxo(201, 50, nsize, payload);
            let utxo = UtxoValue::parse(&bytes, decompress).unwrap();
            assert_eq!(utxo.height, 100);
            assert!(utxo.coinbase);
            assert_eq!(utxo.amount, 5_000_000_000);
            assert_eq!(utxo.nsize, nsize);
            if nsize < 2 {
                assert_eq!(utxo.script, payload);
            } else if nsize < 4 || !decompress {
                assert_eq!(utxo.script[0], nsize as u8);
                assert_eq!(&utxo.script[1..], payload);
            } else {
                let mut compressed = key.clone();
                compressed[0] = nsize as u8 - 2;
                let expected = secp256k1::PublicKey::from_slice(&compressed)
                    .unwrap()
                    .serialize_uncompressed();
                assert_eq!(utxo.script.as_ref(), expected.as_slice());
            }
        }
    }
    let non_coinbase_bytes = encode_utxo(200, 0, 6, &[]);
    let non_coinbase_utxo = UtxoValue::parse(&non_coinbase_bytes, false).unwrap();
    assert_eq!(non_coinbase_utxo.height, 100);
    assert!(!non_coinbase_utxo.coinbase);
}

#[test]
fn rejects_truncated_and_trailing_script_data() {
    for nsize in 0..=6 {
        let payload_size = match nsize {
            0 | 1 => 20,
            2..=5 => 32,
            _ => 0,
        };
        let bytes = encode_utxo(201, 50, nsize, &vec![0x11; payload_size]);
        for length in 0..bytes.len() {
            assert!(
                UtxoValue::parse(&bytes[..length], false).is_err(),
                "nsize={nsize}, length={length}"
            );
        }
        let mut trailing = bytes;
        trailing.push(0);
        assert!(UtxoValue::parse(&trailing, false).is_err());
    }
}

#[test]
fn rejects_numeric_overflow_and_invalid_decompressed_pubkeys() {
    for bytes in [
        encode_utxo((u64::from(u32::MAX) + 1) << 1, 0, 6, &[]),
        encode_utxo(0, u64::MAX, 6, &[]),
        encode_utxo(0, 0, u64::MAX, &[]),
        vec![0xff; 11],
    ] {
        assert!(UtxoValue::parse(&bytes, false).is_err());
    }
    assert!(UtxoValue::parse(&encode_utxo(0, 1, 4, &[0xff; 32]), true).is_err());
}

#[test]
fn witness_programs_ending_in_checkmultisig_keep_their_type_and_address() {
    for (version, size, expected) in [
        (0, 20, ScriptType::P2WPKH),
        (0, 32, ScriptType::P2WSH),
        (0x51, 32, ScriptType::P2TR),
    ] {
        let mut script = vec![version, size];
        script.extend(vec![0x11; size as usize]);
        *script.last_mut().unwrap() = 0xae;
        let utxo = parse_raw_script(&script);
        assert_eq!(utxo.get_script_type(), expected);
        for testnet in [false, true] {
            let address = utxo
                .get_address_with_type(utxo.get_script_type(), testnet, false)
                .unwrap();
            assert!(address.starts_with(if testnet { "tb1" } else { "bc1" }));
        }
    }
}

#[test]
fn witness_templates_require_exact_lengths() {
    for (version, size) in [(0, 20), (0, 32), (0x51, 32)] {
        for actual_size in [size - 1, size + 1] {
            let mut script = vec![version, size];
            script.extend(vec![0x11; actual_size as usize]);
            assert_eq!(
                parse_raw_script(&script).get_script_type(),
                ScriptType::NonStandard
            );
        }
    }
}

fn multisig_script(required: u8, count: u8, push: &[u8]) -> Vec<u8> {
    let mut script = if required <= 16 {
        vec![0x50 + required]
    } else {
        vec![1, required]
    };
    for _ in 0..count {
        script.extend_from_slice(push);
        script.extend(compressed_public_key());
    }
    if count <= 16 {
        script.push(0x50 + count);
    } else {
        script.extend([1, count]);
    }
    script.push(0xae);
    script
}

#[test]
fn multisig_requires_matching_counts_valid_key_shapes_and_no_extra_operations() {
    let public_key_pushes = [
        ("direct push", "21"),
        ("OP_PUSHDATA1", "4c21"),
        ("OP_PUSHDATA2", "4d2100"),
        ("OP_PUSHDATA4", "4e21000000"),
    ];

    for (push_name, encoded_hex) in public_key_pushes {
        let push = hex::decode(encoded_hex).unwrap();
        let script = multisig_script(2, 3, &push);
        assert_eq!(
            parse_raw_script(&script).get_script_type(),
            ScriptType::P2MS,
            "{push_name}"
        );
    }
    assert_eq!(
        parse_raw_script(&multisig_script(17, 20, &[33])).get_script_type(),
        ScriptType::P2MS
    );
    let valid = multisig_script(2, 3, &[33]);
    let mut mismatched_count = valid.clone();
    let count_pos = mismatched_count.len() - 2;
    mismatched_count[count_pos] = 0x52;
    let mut invalid_key = valid.clone();
    invalid_key[2] = 0x05;
    let mut extra_opcode = valid;
    extra_opcode.insert(extra_opcode.len() - 1, 0x61);
    for script in [
        vec![],
        vec![0xae],
        vec![0x51, 0xae],
        vec![0x6a, 0x01, 0xae],
        multisig_script(3, 2, &[33]),
        multisig_script(1, 21, &[33]),
        mismatched_count,
        invalid_key,
        extra_opcode,
    ] {
        assert_eq!(
            parse_raw_script(&script).get_script_type(),
            ScriptType::NonStandard
        );
    }
}
