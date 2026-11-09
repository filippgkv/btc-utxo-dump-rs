use csv::Writer;

use super::support::encode_varint;
use crate::address_totals::AddressTotals;
use crate::bitcoin::address::hash160_to_address;
use crate::bitcoin::chainstate::{deobfuscate_value, ChainStateKey};
use crate::dump::{add_utxo_amount, export_utxos, OBFUSCATION_KEY_PREFIX, UTXO_PREFIX};
use crate::utils::cli::Cli;
use db_key::Key;

const OBFUSCATION_KEY: [u8; 8] = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];

fn chainstate_entries() -> Vec<(ChainStateKey, Vec<u8>)> {
    let mut key_value = vec![8];
    key_value.extend(OBFUSCATION_KEY);
    let mut entries = vec![
        (ChainStateKey::from_u8(&[OBFUSCATION_KEY_PREFIX]), key_value),
        (ChainStateKey::from_u8(b"B"), vec![]),
    ];
    for index in 0..3 {
        let mut key = vec![UTXO_PREFIX];
        key.extend([index as u8; 32]);
        key.extend(encode_varint(index));
        let mut value = encode_varint(201); // height 100, coinbase
        value.extend([index as u8 + 1, 0]); // amounts 1, 10, 100; P2PKH
        value.extend([0x11; 20]);
        entries.push((
            ChainStateKey::from_u8(&key),
            deobfuscate_value(&value, &OBFUSCATION_KEY),
        ));
    }
    entries
}

fn export_csv(args: &[&str], entries: Vec<(ChainStateKey, Vec<u8>)>) -> (u64, Vec<Vec<String>>) {
    let cli =
        Cli::try_parse_from(["dump", "--quiet"].into_iter().chain(args.iter().copied())).unwrap();
    let mut writer = Writer::from_writer(Vec::new());
    let count = export_utxos(&cli, entries, &mut writer).unwrap();
    let bytes = writer.into_inner().unwrap();
    let rows = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(bytes.as_slice())
        .records()
        .map(|row| row.unwrap().iter().map(str::to_owned).collect())
        .collect();
    (count, rows)
}

#[test]
fn limits_and_counts_are_exact_in_both_output_modes() {
    for (limit, expected_count, expected_amount) in [
        ("0", 3, "111"),
        ("1", 1, "1"),
        ("2", 2, "11"),
        ("3", 3, "111"),
        ("5", 3, "111"),
    ] {
        let (count, rows) = export_csv(
            &["--max-utxos", limit, "--fields", "count,vout,amount"],
            chainstate_entries(),
        );
        assert_eq!(count, expected_count);
        assert_eq!(rows.len() as u64, count + 1);
        for (index, row) in rows.iter().skip(1).enumerate() {
            assert_eq!(row[0], (index + 1).to_string());
            assert_eq!(row[1], index.to_string());
            assert_eq!(row[2], ["1", "10", "100"][index]);
        }

        let (count, rows) = export_csv(
            &["--max-utxos", limit, "--address-totals"],
            chainstate_entries(),
        );
        assert_eq!(count, expected_count);
        assert_eq!(
            rows,
            vec![
                vec!["address".to_owned(), "amount".to_owned()],
                vec![
                    hash160_to_address(&[0x11; 20], 0),
                    expected_amount.to_owned()
                ]
            ]
        );
    }
}

#[test]
fn limit_does_not_read_past_the_last_requested_utxo() {
    for totals in [false, true] {
        let args = if totals {
            vec!["--max-utxos", "1", "--address-totals"]
        } else {
            vec!["--max-utxos", "1", "--fields", "count"]
        };
        let cli = Cli::try_parse_from(["dump", "--quiet"].into_iter().chain(args)).unwrap();
        let mut writer = Writer::from_writer(Vec::new());
        let stream = chainstate_entries()
            .into_iter()
            .take(3)
            .chain(std::iter::from_fn(|| panic!("read past limit")));
        assert_eq!(export_utxos(&cli, stream, &mut writer).unwrap(), 1);
    }
}

#[test]
fn totals_limit_counts_inputs_before_prefix_filtering() {
    let (count, rows) = export_csv(
        &[
            "--address-totals",
            "--max-utxos",
            "1",
            "--prefix",
            "unmatched",
        ],
        chainstate_entries(),
    );
    assert_eq!(count, 1);
    assert_eq!(rows, vec![vec!["address", "amount"]]);
}

#[test]
fn whitespace_fields_produce_normalized_headers_and_populated_columns() {
    let (count, rows) = export_csv(
        &["--max-utxos", "1", "--fields", " count , amount , vout "],
        chainstate_entries(),
    );
    assert_eq!(count, 1);
    assert_eq!(
        rows,
        vec![vec!["count", "amount", "vout"], vec!["1", "1", "0"]]
    );
}

#[test]
fn all_fields_preserve_the_existing_csv_representation() {
    let (_, rows) = export_csv(
        &[
            "--fields",
            "count,txid,vout,height,coinbase,amount,nsize,script,type,address",
        ],
        chainstate_entries(),
    );
    for (index, row) in rows.iter().skip(1).enumerate() {
        assert_eq!(
            *row,
            vec![
                (index + 1).to_string(),
                hex::encode([index as u8; 32]),
                index.to_string(),
                "100".to_owned(),
                "1".to_owned(),
                ["1", "10", "100"][index].to_owned(),
                "0".to_owned(),
                hex::encode([0x11; 20]),
                "p2pkh".to_owned(),
                hash160_to_address(&[0x11; 20], 0),
            ]
        );
    }
}

#[test]
fn an_empty_database_writes_only_headers() {
    for totals in [false, true] {
        let args = if totals {
            vec!["--address-totals"]
        } else {
            vec!["--fields", "count"]
        };
        let (count, rows) = export_csv(&args, vec![]);
        assert_eq!(count, 0);
        assert_eq!(rows.len(), 1);
    }
}

#[test]
fn invalid_metadata_and_utxo_values_return_contextual_errors() {
    for value in [vec![], vec![8], vec![1, 1, 2]] {
        let cli = Cli::try_parse_from(["dump", "--quiet"]).unwrap();
        let mut writer = Writer::from_writer(Vec::new());
        assert!(export_utxos(
            &cli,
            vec![(ChainStateKey::from_u8(&[OBFUSCATION_KEY_PREFIX]), value)],
            &mut writer
        )
        .is_err());
    }
    for totals in [false, true] {
        let args = if totals {
            vec!["--address-totals"]
        } else {
            vec!["--fields", "amount"]
        };
        let cli = Cli::try_parse_from(["dump", "--quiet"].into_iter().chain(args)).unwrap();
        let mut writer = Writer::from_writer(Vec::new());
        let error = export_utxos(
            &cli,
            vec![(ChainStateKey::from_u8(&[UTXO_PREFIX]), vec![])],
            &mut writer,
        )
        .unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("Failed to process UTXO 1"));
        assert!(message.contains("Truncated VARINT"));
    }
}

#[test]
fn address_totals_reject_overflow() {
    for prefix in ["", "1"] {
        let cli = Cli::try_parse_from(["dump", "--address-totals", "--prefix", prefix]).unwrap();
        let mut totals = AddressTotals::new(&cli.address_filter);
        // This compressed amount represents 10^19 satoshis: each value fits u64,
        // but summing two of them does not.
        let mut value = encode_varint(201);
        value.extend(encode_varint(100_000_000_000));
        value.push(0);
        value.extend([0x11; 20]);
        add_utxo_amount(&cli, &mut totals, &mut value, &[]).unwrap();
        let error = add_utxo_amount(&cli, &mut totals, &mut value, &[]).unwrap_err();
        assert!(error.to_string().contains("Address total exceeds u64"));
    }
}
