use crate::utils::cli::{AddressSort, Cli};

#[test]
fn classifies_address_family_prefixes_for_the_selected_network() {
    use crate::address_filter::{AddressFilter, AddressTypes};

    for (prefix, testnet, types) in [
        ("1", false, AddressTypes::P2PKH),
        ("3", false, AddressTypes::P2SH),
        ("bc1q", false, AddressTypes::WitnessV0),
        ("bc1p", false, AddressTypes::P2TR),
        ("2", true, AddressTypes::P2SH),
        ("tb1q", true, AddressTypes::WitnessV0),
        ("tb1p", true, AddressTypes::P2TR),
    ] {
        for network in [false, true] {
            let mut args = vec!["dump", "--address-totals", "--prefix", prefix];
            if network {
                args.push("--testnet");
            }
            let cli = Cli::try_parse_from(args).unwrap();
            let expected = if network == testnet {
                types
            } else {
                AddressTypes::None
            };
            assert_eq!(cli.address_filter, AddressFilter::Types(expected));
        }
    }
    for testnet in [false, true] {
        assert_eq!(
            AddressFilter::classify(String::new(), testnet),
            AddressFilter::Types(AddressTypes::All)
        );
        for prefix in [
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
            assert_eq!(
                AddressFilter::classify(prefix.to_owned(), testnet),
                AddressFilter::Text(prefix.to_owned())
            );
        }
    }
}

#[test]
fn field_whitespace_is_handled_consistently() {
    let cli = Cli::try_parse_from(["dump", "--fields", " count , amount , vout "]).unwrap();
    let indices = cli.fields.indices();
    assert_eq!(indices.count, Some(0));
    assert_eq!(indices.amount, Some(1));
    assert_eq!(indices.vout, Some(2));
}

#[test]
fn rejects_duplicate_unknown_and_empty_fields() {
    for fields in ["amount,amount", "amount, amount ", "unknown", "", "count,"] {
        assert!(
            Cli::try_parse_from(["dump", "--fields", fields]).is_err(),
            "fields={fields:?}"
        );
    }
}

#[test]
fn address_totals_ignore_fields_as_documented() {
    let cli = Cli::try_parse_from(["dump", "--address-totals", "--fields", "ignored"]).unwrap();
    assert!(cli.fields.ordered().is_empty());
}

#[test]
fn address_output_options_require_address_totals() {
    for arguments in [
        vec!["dump", "--addresses-only"],
        vec!["dump", "--min-amount", "1000000"],
        vec!["dump", "--min-amount", "0"],
        vec!["dump", "--sort", "address"],
        vec!["dump", "--sort", "amount"],
    ] {
        assert!(Cli::try_parse_from(arguments).is_err());
    }
    let defaults = Cli::try_parse_from(["dump"]).unwrap();
    assert_eq!(defaults.min_amount, 0);
    assert!(!defaults.addresses_only);
    assert_eq!(defaults.sort, None);
}

#[test]
fn sorting_accepts_address_or_amount_and_rejects_unknown_orders() {
    for (argument, expected) in [
        ("address", AddressSort::Address),
        ("alphabetical", AddressSort::Address),
        ("amount", AddressSort::Amount),
        ("value", AddressSort::Amount),
    ] {
        let cli = Cli::try_parse_from(["dump", "--address-totals", "--sort", argument]).unwrap();
        assert_eq!(cli.sort, Some(expected));
    }
    assert!(Cli::try_parse_from(["dump", "--address-totals", "--sort", "unknown"]).is_err());
    assert!(Cli::try_parse_from(["dump", "--address-totals", "--sort"]).is_err());
}

#[test]
fn minimum_amount_accepts_integer_satoshis_only() {
    let cli = Cli::try_parse_from([
        "dump",
        "--address-totals",
        "--addresses-only",
        "--min-amount",
        "1000000",
    ])
    .unwrap();
    assert_eq!(cli.min_amount, 1_000_000);
    assert!(cli.addresses_only);

    for amount in ["-1", "0.01", "one-million", "18446744073709551616"] {
        assert!(Cli::try_parse_from(["dump", "--address-totals", "--min-amount", amount]).is_err());
    }
}
