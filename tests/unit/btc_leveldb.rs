use crate::bitcoin::btc_leveldb::{decompress_amount, read_varint};

use super::support::encode_varint;

#[test]
fn decodes_core_vectors_and_leaves_remaining_input() {
    let cases = [
        (0, "00"),
        (127, "7f"),
        (128, "8000"),
        (255, "807f"),
        (256, "8100"),
        (16_383, "fe7f"),
        (16_384, "ff00"),
        (16_511, "ff7f"),
        (65_535, "82fe7f"),
        (1 << 32, "8efefeff00"),
    ];

    for (expected_value, encoded_hex) in cases {
        let mut bytes = hex::decode(encoded_hex).unwrap();
        bytes.push(42);

        let mut input = bytes.as_slice();
        assert_eq!(
            read_varint(&mut input).unwrap(),
            expected_value,
            "encoded bytes: {encoded_hex}"
        );
        assert_eq!(input, &[42]);
    }
}

#[test]
fn handles_unsigned_boundaries() {
    for value in [u32::MAX as u64, i64::MAX as u64, 1 << 63, u64::MAX] {
        let bytes = encode_varint(value);
        let mut input = bytes.as_slice();
        assert_eq!(read_varint(&mut input).unwrap(), value);
        assert!(input.is_empty());
    }
}

#[test]
fn rejects_truncation_and_both_overflow_paths() {
    let invalid_encodings = [
        ("empty input", ""),
        ("missing second byte", "80"),
        ("missing third byte", "8080"),
        ("value exceeds u64", "ffffffffffffffffffffff"),
    ];

    for (reason, encoded_hex) in invalid_encodings {
        let bytes = hex::decode(encoded_hex).unwrap();
        let mut input = bytes.as_slice();
        assert!(read_varint(&mut input).is_err(), "{reason}");
    }

    let mut bytes = encode_varint(u64::MAX);
    *bytes.last_mut().unwrap() |= 0x80;
    let mut input = bytes.as_slice();
    assert!(
        read_varint(&mut input).is_err(),
        "continuation overflows u64"
    );
}

#[test]
fn decompresses_known_amounts_and_rejects_overflow() {
    for (compressed, amount) in [
        (0, 0),
        (1, 1),
        (2, 10),
        (9, 100_000_000),
        (10, 1_000_000_000),
        (50, 5_000_000_000),
        (1_101, 123),
    ] {
        assert_eq!(decompress_amount(compressed).unwrap(), amount);
    }
    assert!(decompress_amount(u64::MAX).is_err());
}
