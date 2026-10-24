use super::fixtures::COINS;
use crate::bitcoin::utxo::UtxoValue;

#[test]
fn every_script_type_produces_the_expected_address_on_both_networks() {
    for coin in COINS {
        let bytes = hex::decode(coin.value_hex).unwrap();
        for include_p2pk in [false, true] {
            let utxo = UtxoValue::parse(&bytes, include_p2pk).unwrap();
            assert_eq!(utxo.height, 100, "{}", coin.name);
            assert!(utxo.coinbase, "{}", coin.name);
            assert_eq!(utxo.amount, coin.amount, "{}", coin.name);
            assert_eq!(utxo.nsize, coin.nsize, "{}", coin.name);
            assert_eq!(
                utxo.get_script_type().to_string(),
                coin.script_type,
                "{}",
                coin.name
            );

            if include_p2pk {
                assert_eq!(hex::encode(&utxo.script), coin.script_hex, "{}", coin.name);
            }

            for testnet in [false, true] {
                let mut expected = if testnet {
                    coin.testnet_address
                } else {
                    coin.mainnet_address
                };
                if coin.script_type == "p2pk" && !include_p2pk {
                    expected = None;
                }

                let address =
                    utxo.get_address_with_type(utxo.get_script_type(), testnet, include_p2pk);
                assert_eq!(
                    address.as_deref(),
                    expected,
                    "{}, testnet={testnet}, include_p2pk={include_p2pk}",
                    coin.name
                );
            }
        }
    }
}
