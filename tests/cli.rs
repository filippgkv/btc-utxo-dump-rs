mod fixtures;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use db_key::Key;
use leveldb::database::Database;
use leveldb::kv::KV;
use leveldb::options::{Options, WriteOptions};
use tempfile::TempDir;

use fixtures::COINS;

struct DatabaseKey(Vec<u8>);

impl Key for DatabaseKey {
    fn from_u8(bytes: &[u8]) -> Self {
        Self(bytes.to_vec())
    }

    fn as_slice<T, F: Fn(&[u8]) -> T>(&self, callback: F) -> T {
        callback(&self.0)
    }
}

struct TestDatabase {
    _directory: TempDir,
    database_path: PathBuf,
    output_path: PathBuf,
}

impl TestDatabase {
    fn new(obfuscated: bool) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let fixture = Self {
            database_path: directory.path().join("chainstate"),
            output_path: directory.path().join("utxos.csv"),
            _directory: directory,
        };
        let mut options = Options::new();
        options.create_if_missing = true;
        let database = Database::<DatabaseKey>::open(&fixture.database_path, options).unwrap();

        if obfuscated {
            // Core serializes this key as a length-prefixed string containing a leading NUL.
            let key = DatabaseKey::from_u8(b"\x0e\x00obfuscate_key");
            let mut value = vec![fixtures::OBFUSCATION_KEY.len() as u8];
            value.extend(fixtures::OBFUSCATION_KEY);
            database.put(WriteOptions::new(), key, &value).unwrap();
        }
        database
            .put(WriteOptions::new(), DatabaseKey::from_u8(b"B"), &[0; 32])
            .unwrap();

        // Insert in reverse order to verify that the database iterator supplies key order.
        for (index, coin) in COINS.iter().enumerate().rev() {
            let mut key = vec![b'C'];
            key.extend([index as u8; 32]);
            key.push(index as u8);
            let value_hex = if obfuscated {
                coin.obfuscated_hex
            } else {
                coin.value_hex
            };
            database
                .put(
                    WriteOptions::new(),
                    DatabaseKey::from_u8(&key),
                    &hex::decode(value_hex).unwrap(),
                )
                .unwrap();
        }
        drop(database); // Release LevelDB's lock before invoking the command.
        fixture
    }

    fn command(&self) -> Command {
        dump_command(&self.database_path, &self.output_path)
    }

    fn rows(&self) -> Vec<csv::StringRecord> {
        csv::ReaderBuilder::new()
            .has_headers(false)
            .from_path(&self.output_path)
            .unwrap()
            .records()
            .map(|row| row.unwrap())
            .collect()
    }

    fn totals(&self) -> BTreeMap<String, u64> {
        let rows = self.rows();
        assert_eq!(rows[0], vec!["address", "amount"]);
        let mut totals = BTreeMap::new();
        for row in &rows[1..] {
            assert!(
                totals
                    .insert(row[0].to_owned(), row[1].parse().unwrap())
                    .is_none(),
                "duplicate totals row for {}",
                &row[0]
            );
        }
        totals
    }
}

fn dump_command(database_path: &Path, output_path: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_btc-utxo-dump-rs"));
    command
        .arg("--db")
        .arg(database_path)
        .arg("--output")
        .arg(output_path);
    command
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn exports_all_script_types_from_obfuscated_and_unobfuscated_databases() {
    let fields = "count,txid,vout,height,coinbase,amount,nsize,script,type,address";

    for obfuscated in [false, true] {
        let fixture = TestDatabase::new(obfuscated);
        for testnet in [false, true] {
            for include_p2pk in [false, true] {
                let mut command = fixture.command();
                command.args(["--quiet", "--fields", fields]);
                if testnet {
                    command.arg("--testnet");
                }
                if include_p2pk {
                    command.arg("--include-p2pk");
                }
                let output = command.output().unwrap();
                assert_success(&output);
                assert!(output.stdout.is_empty());

                let rows = fixture.rows();
                assert_eq!(rows[0], fields.split(',').collect::<Vec<_>>());
                assert_eq!(rows.len(), COINS.len() + 1);
                for (index, coin) in COINS.iter().enumerate() {
                    let mut address = if testnet {
                        coin.testnet_address
                    } else {
                        coin.mainnet_address
                    };
                    if coin.script_type == "p2pk" && !include_p2pk {
                        address = None;
                    }
                    let expected = vec![
                        (index + 1).to_string(),
                        format!("{index:02x}").repeat(32),
                        index.to_string(),
                        "100".to_owned(),
                        "1".to_owned(),
                        coin.amount.to_string(),
                        coin.nsize.to_string(),
                        coin.script_hex.to_owned(),
                        coin.script_type.to_owned(),
                        address.unwrap_or("").to_owned(),
                    ];
                    assert_eq!(rows[index + 1], expected, "{}, obfuscated={obfuscated}, testnet={testnet}, include_p2pk={include_p2pk}", coin.name);
                }
            }
        }
    }
}

#[test]
fn default_columns_decompress_p2pk_only_when_addresses_are_requested() {
    let fixture = TestDatabase::new(true);
    for testnet in [false, true] {
        for include_p2pk in [false, true] {
            let mut command = fixture.command();
            command.arg("--quiet");
            if testnet {
                command.arg("--testnet");
            }
            if include_p2pk {
                command.arg("--include-p2pk");
            }
            let output = command.output().unwrap();
            assert_success(&output);

            let rows = fixture.rows();
            assert_eq!(
                rows[0],
                vec!["count", "txid", "vout", "amount", "type", "address"]
            );
            assert_eq!(rows.len(), COINS.len() + 1);
            for (index, coin) in COINS.iter().enumerate() {
                let mut expected_address = if testnet {
                    coin.testnet_address
                } else {
                    coin.mainnet_address
                };
                if coin.script_type == "p2pk" && !include_p2pk {
                    expected_address = None;
                }
                assert_eq!(&rows[index + 1][4], coin.script_type, "{}", coin.name);
                assert_eq!(
                    &rows[index + 1][5],
                    expected_address.unwrap_or(""),
                    "{}, testnet={testnet}, include_p2pk={include_p2pk}",
                    coin.name
                );
            }
        }
    }
}

#[test]
fn invalid_fields_leave_an_existing_output_file_unchanged() {
    let fixture = TestDatabase::new(true);
    for fields in ["unknown", "amount,amount", ""] {
        fs::write(&fixture.output_path, "existing output\n").unwrap();
        let output = fixture
            .command()
            .args(["--fields", fields])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert_eq!(
            fs::read_to_string(&fixture.output_path).unwrap(),
            "existing output\n"
        );
    }
}

#[test]
fn custom_field_order_controls_both_headers_and_values() {
    let fixture = TestDatabase::new(true);
    let output = fixture
        .command()
        .args([
            "--quiet",
            "--fields",
            " address , count , amount ",
            "--max-utxos",
            "1",
        ])
        .output()
        .unwrap();
    assert_success(&output);
    let rows = fixture.rows();
    assert_eq!(rows[0], vec!["address", "count", "amount"]);
    assert_eq!(
        rows[1],
        vec!["1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH", "1", "1"]
    );
}

#[test]
fn command_reports_the_exact_limit_and_quiet_overrides_verbose() {
    let fixture = TestDatabase::new(true);
    let output = fixture
        .command()
        .args(["--max-utxos", "2", "--fields", "count"])
        .output()
        .unwrap();
    assert_success(&output);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "\nTotal UTXOs: 2\n"
    );
    assert_eq!(
        fs::read_to_string(&fixture.output_path).unwrap(),
        "count\n1\n2\n"
    );

    let output = fixture
        .command()
        .args(["--quiet", "--verbose", "--max-utxos", "1"])
        .output()
        .unwrap();
    assert_success(&output);
    assert!(output.stdout.is_empty());
}

#[test]
fn missing_database_fails_before_creating_output() {
    let fixture = TestDatabase::new(true);
    let missing_path = fixture.database_path.join("missing");
    let output = dump_command(&missing_path, &fixture.output_path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Failed to open database"));
    assert!(!fixture.output_path.exists());
    assert!(!missing_path.join("CURRENT").exists());
}

#[test]
fn locked_database_fails_before_creating_output() {
    let fixture = TestDatabase::new(true);
    let _held_database =
        Database::<DatabaseKey>::open(&fixture.database_path, Options::new()).unwrap();
    let output = fixture.command().output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Failed to open database"));
    assert!(!fixture.output_path.exists());
}

#[test]
fn output_creation_failure_returns_a_nonzero_exit_status() {
    let fixture = TestDatabase::new(true);
    fs::create_dir(&fixture.output_path).unwrap();
    let output = fixture.command().arg("--quiet").output().unwrap();
    assert!(!output.status.success());
    assert!(!output.stderr.is_empty());
    assert!(output.stdout.is_empty());
    assert!(fixture.output_path.is_dir());
}

#[cfg(target_os = "linux")]
#[test]
fn final_buffer_flush_failure_returns_a_nonzero_exit_status() {
    let fixture = TestDatabase::new(true);
    // These rows fit in the CSV buffer; /dev/full fails when main flushes it.
    let output = dump_command(&fixture.database_path, Path::new("/dev/full"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("No space left on device"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Total UTXOs"));
}

#[test]
fn totals_keep_distinct_addresses_separate_and_skip_scripts_without_addresses() {
    let fixture = TestDatabase::new(true);
    let output = fixture
        .command()
        .args(["--quiet", "--address-totals"])
        .output()
        .unwrap();
    assert_success(&output);

    let expected = [
        ("1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH", 1),
        ("3CNHUhP3uyB9EUtRLsmvFUmvGdjGdkTxJw", 10),
        ("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4", 1_000_000),
        (
            "bc1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3qccfmv3",
            10_000_000,
        ),
        (
            "bc1p0xlxvlhemja6c4dqv22uapctqupfhlxm9h8z3k2e72q4k9hcz7vqzk5jj0",
            100_000_000,
        ),
    ];
    let totals = fixture.totals();
    assert_eq!(totals.len(), expected.len());
    for (address, amount) in expected {
        assert_eq!(totals.get(address), Some(&amount), "{address}");
    }
}

#[test]
fn totals_merge_p2pk_and_p2pkh_outputs_with_the_same_address() {
    let fixture = TestDatabase::new(true);
    let output = fixture
        .command()
        .args(["--quiet", "--address-totals", "--include-p2pk"])
        .output()
        .unwrap();
    assert_success(&output);

    let expected = [
        ("1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH", 101),
        ("3CNHUhP3uyB9EUtRLsmvFUmvGdjGdkTxJw", 10),
        ("1GrLCmVQXoyJXaPJQdqssNqwxvha1eUo2E", 1_000),
        ("1EHNa6Q4Jz2uvNExL497mE43ikXhwF6kZm", 10_000),
        ("1JPbzbsAx1HyaDQoLMapWGoqf9pD5uha5m", 100_000),
        ("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4", 1_000_000),
        (
            "bc1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3qccfmv3",
            10_000_000,
        ),
        (
            "bc1p0xlxvlhemja6c4dqv22uapctqupfhlxm9h8z3k2e72q4k9hcz7vqzk5jj0",
            100_000_000,
        ),
    ];
    let totals = fixture.totals();
    assert_eq!(totals.len(), expected.len());
    for (address, amount) in expected {
        assert_eq!(totals.get(address), Some(&amount), "{address}");
    }
}

#[test]
fn totals_match_nonempty_prefixes_on_both_networks() {
    let fixture = TestDatabase::new(true);
    for (testnet, prefix, expected_address) in [
        (false, "1Bg", "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH"),
        (true, "mrC", "mrCDrCybB6J1vRfbwM5hemdJz73FwDBC8r"),
    ] {
        let mut command = fixture.command();
        command.args([
            "--quiet",
            "--address-totals",
            "--include-p2pk",
            "--prefix",
            prefix,
        ]);
        if testnet {
            command.arg("--testnet");
        }
        let output = command.output().unwrap();
        assert_success(&output);
        let totals = fixture.totals();
        assert_eq!(totals.len(), 1);
        assert_eq!(totals.get(expected_address), Some(&101));
    }
}

#[test]
fn totals_limit_applies_to_database_entries_and_reports_the_processed_count() {
    let fixture = TestDatabase::new(true);
    let output = fixture
        .command()
        .args(["--address-totals", "--max-utxos", "2"])
        .output()
        .unwrap();
    assert_success(&output);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "\nTotal UTXOs: 2\n"
    );
    let totals = fixture.totals();
    assert_eq!(totals.len(), 2);
    assert_eq!(totals.get("1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH"), Some(&1));
    assert_eq!(totals.get("3CNHUhP3uyB9EUtRLsmvFUmvGdjGdkTxJw"), Some(&10));
}

#[test]
fn corrupt_utxo_stops_the_command_with_a_contextual_error() {
    let fixture = TestDatabase::new(false);
    let database = Database::<DatabaseKey>::open(&fixture.database_path, Options::new()).unwrap();
    let mut key = vec![b'C'];
    key.extend([0; 33]);
    database
        .put(WriteOptions::new(), DatabaseKey::from_u8(&key), &[0x80])
        .unwrap();
    drop(database);

    let output = fixture.command().output().unwrap();
    assert!(!output.status.success());
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(message.contains("Failed to process UTXO 1"));
    assert!(message.contains("Truncated VARINT"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Total UTXOs"));
}
