use crate::utils::fields::{Field, FieldSelection};

#[test]
fn parsed_fields_preserve_order_and_supply_matching_headers_and_positions() {
    let selection: FieldSelection = " address , count , amount ".parse().unwrap();
    assert_eq!(
        selection.ordered(),
        vec![Field::Address, Field::Count, Field::Amount]
    );
    let headers: Vec<_> = selection.headers().collect();
    assert_eq!(headers, vec!["address", "count", "amount"]);
    assert_eq!(selection.indices().address, Some(0));
    assert_eq!(selection.indices().count, Some(1));
    assert_eq!(selection.indices().amount, Some(2));
}

#[test]
fn fields_round_trip_through_their_csv_names() {
    for field in [
        Field::Count,
        Field::Txid,
        Field::Vout,
        Field::Height,
        Field::Coinbase,
        Field::Amount,
        Field::Nsize,
        Field::Script,
        Field::Type,
        Field::Address,
    ] {
        assert_eq!(field.name().parse::<Field>().unwrap(), field);
    }
}

#[test]
fn key_only_fields_skip_values_and_only_script_or_p2pk_addresses_need_decompression() {
    let keys: FieldSelection = "count,txid,vout".parse().unwrap();
    assert!(!keys.indices().needs_utxo_parsing());
    assert!(!keys.indices().needs_decompression(true));
    let amount: FieldSelection = "amount".parse().unwrap();
    assert!(amount.indices().needs_utxo_parsing());
    assert!(!amount.indices().needs_decompression(true));
    let script: FieldSelection = "script".parse().unwrap();
    assert!(script.indices().needs_decompression(false));
    let address: FieldSelection = "address".parse().unwrap();
    assert!(!address.indices().needs_decompression(false));
    assert!(address.indices().needs_decompression(true));
}
