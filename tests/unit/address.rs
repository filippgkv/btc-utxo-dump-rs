use crate::bitcoin::address::{hash160_to_address, public_key_to_address, segwit_address};

#[test]
fn segwit_addresses_match_bip173_and_bip350_vectors() {
    // https://github.com/bitcoin/bips/blob/master/bip-0350.mediawiki#test-vectors-for-v0-v16-native-segregated-witness-addresses
    let cases = [
        (
            0,
            false,
            "751e76e8199196d454941c45d1b3a323f1433bd6",
            "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4",
        ),
        (
            0,
            true,
            "751e76e8199196d454941c45d1b3a323f1433bd6",
            "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx",
        ),
        (
            0,
            false,
            "1863143c14c5166804bd19203356da136c985678cd4d27a1b8c6329604903262",
            "bc1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3qccfmv3",
        ),
        (
            0,
            true,
            "1863143c14c5166804bd19203356da136c985678cd4d27a1b8c6329604903262",
            "tb1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3q0sl5k7",
        ),
        (
            1,
            false,
            "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
            "bc1p0xlxvlhemja6c4dqv22uapctqupfhlxm9h8z3k2e72q4k9hcz7vqzk5jj0",
        ),
        (
            1,
            true,
            "000000c4a5cad46221b2a187905e5266362b99d5e91c6ce24d165dab93e86433",
            "tb1pqqqqp399et2xygdj5xreqhjjvcmzhxw4aywxecjdzew6hylgvsesf3hn0c",
        ),
    ];

    for (version, testnet, program_hex, expected_address) in cases {
        let program = hex::decode(program_hex).unwrap();
        assert_eq!(
            segwit_address(version, &program, testnet).unwrap(),
            expected_address
        );
    }
}

#[test]
fn base58_addresses_match_known_public_key_and_hash() {
    let public_key =
        hex::decode("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798").unwrap();
    let hash = hex::decode("751e76e8199196d454941c45d1b3a323f1433bd6").unwrap();
    let hash: [u8; 20] = hash.try_into().unwrap();

    assert_eq!(
        hash160_to_address(&hash, 0x00),
        "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH"
    );
    assert_eq!(
        public_key_to_address(&public_key, 0x00),
        "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH"
    );
    assert_eq!(
        public_key_to_address(&public_key, 0x6f),
        "mrCDrCybB6J1vRfbwM5hemdJz73FwDBC8r"
    );
}

#[test]
fn segwit_addresses_reject_invalid_versions_and_program_lengths() {
    let invalid_programs = [
        (0, 0),
        (0, 19),
        (0, 21),
        (0, 31),
        (0, 33),
        (1, 1),
        (1, 41),
        (17, 32),
        (32, 32),
        (255, 32),
    ];

    for (version, length) in invalid_programs {
        let program = vec![0x11; length];
        assert!(
            segwit_address(version, &program, false).is_err(),
            "version={version}, program length={length}"
        );
    }
}
