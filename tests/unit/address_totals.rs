use std::collections::HashMap;

use crate::address_filter::AddressFilter;
use crate::address_totals::AddressTotals;
use crate::utils::cli::Cli;

#[test]
fn prefix_filters_match_text_filtering_after_balances_are_summed() {
    for prefix in [
        "",
        "1",
        "3",
        "bc1q",
        "bc1p",
        "2",
        "tb1q",
        "tb1p",
        "11",
        "1Bg",
        "3C",
        "bc1",
        "bc1qw",
        "bc1p0xl",
        "tb1qw",
        "m",
        "n",
        "BC1Q",
        "unmatched",
    ] {
        for testnet in [false, true] {
            for include_p2pk in [false, true] {
                for addresses_only in [false, true] {
                    check_prefix_filter(prefix, testnet, include_p2pk, addresses_only);
                }
            }
        }
    }
}

fn check_prefix_filter(prefix: &str, testnet: bool, include_p2pk: bool, addresses_only: bool) {
    let mut args = vec![
        "dump",
        "--address-totals",
        "--prefix",
        prefix,
        "--min-amount",
        "800000",
        "--sort",
        "address",
    ];
    if testnet {
        args.push("--testnet");
    }
    if include_p2pk {
        args.push("--include-p2pk");
    }
    if addresses_only {
        args.push("--addresses-only");
    }
    let mut cli = Cli::try_parse_from(args).unwrap();
    let mut totals = AddressTotals::new(&cli.address_filter);
    // Totals own their filter even if the caller's configuration changes.
    cli.address_filter = AddressFilter::Text("unrelated".to_owned());
    let mut reference = HashMap::<String, u64>::new();
    let mut values: Vec<_> = super::fixtures::COINS
        .iter()
        .map(|coin| hex::decode(coin.value_hex).unwrap())
        .collect();
    // Leading-zero hashes produce multiple leading '1' characters.
    let mut zero_hash = vec![0, 1, 0];
    zero_hash.extend([0; 20]);
    values.push(zero_hash);
    for amount in [399_999, 400_000, 400_001] {
        for value in &values {
            let mut utxo = crate::bitcoin::utxo::UtxoValue::parse(value, include_p2pk).unwrap();
            utxo.amount = amount;
            totals.add(&utxo, &cli).unwrap();
            if let Some(address) =
                utxo.get_address_with_type(utxo.get_script_type(), testnet, include_p2pk)
            {
                if address.starts_with(prefix) {
                    *reference.entry(address).or_default() += amount;
                }
            }
        }
    }
    let mut actual = csv::Writer::from_writer(Vec::new());
    totals.write_output(&cli, &mut actual).unwrap();
    let mut expected = csv::Writer::from_writer(Vec::new());
    AddressTotals::ByPrefix {
        prefix: prefix.to_owned(),
        totals: reference,
    }
    .write_output(&cli, &mut expected)
    .unwrap();
    assert_eq!(actual.into_inner().unwrap(), expected.into_inner().unwrap(), "prefix={prefix:?}, testnet={testnet}, include_p2pk={include_p2pk}, addresses_only={addresses_only}");
}

#[test]
fn amount_sort_handles_u64_balances_and_breaks_ties_alphabetically() {
    let low_address = "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH";
    let first_tied_address = "1EHNa6Q4Jz2uvNExL497mE43ikXhwF6kZm";
    let second_tied_address = "1GrLCmVQXoyJXaPJQdqssNqwxvha1eUo2E";
    let totals = AddressTotals::ByPrefix {
        prefix: String::new(),
        totals: HashMap::from([
            (low_address.to_owned(), 1),
            (second_tied_address.to_owned(), u64::MAX),
            (first_tied_address.to_owned(), u64::MAX),
        ]),
    };
    let cli = Cli::try_parse_from([
        "dump",
        "--address-totals",
        "--addresses-only",
        "--sort",
        "amount",
    ])
    .unwrap();
    let mut writer = csv::Writer::from_writer(Vec::new());
    totals.write_output(&cli, &mut writer).unwrap();
    let contents = String::from_utf8(writer.into_inner().unwrap()).unwrap();
    assert_eq!(
        contents,
        format!("{first_tied_address}\n{second_tied_address}\n{low_address}\n")
    );
}
