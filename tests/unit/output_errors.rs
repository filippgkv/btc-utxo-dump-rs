use std::io::{self, Write};

use csv::WriterBuilder;
use db_key::Key;

use super::fixtures::COINS;
use crate::bitcoin::chainstate::ChainStateKey;
use crate::dump::export_utxos;
use crate::utils::cli::Cli;

struct FailingWriter {
    remaining_bytes: usize,
}

impl Write for FailingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.remaining_bytes == 0 {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "simulated output failure",
            ));
        }

        let written = bytes.len().min(self.remaining_bytes);
        self.remaining_bytes -= written;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn csv_totals_and_address_lists_propagate_data_row_write_failures() {
    for (address_totals, addresses_only, sort) in [
        (false, false, None),
        (true, false, None),
        (true, true, None),
        (true, false, Some("address")),
        (true, true, Some("amount")),
    ] {
        let mut arguments = vec!["dump", "--quiet", "--fields", "count,amount"];
        if address_totals {
            arguments.push("--address-totals");
        }
        if addresses_only {
            arguments.push("--addresses-only");
        }
        if let Some(order) = sort {
            arguments.extend(["--sort", order]);
        }
        let cli = Cli::try_parse_from(arguments).unwrap();
        let header = if addresses_only {
            ""
        } else if address_totals {
            "address,amount\n"
        } else {
            "count,amount\n"
        };
        let output = FailingWriter {
            remaining_bytes: header.len(),
        };
        let mut writer = WriterBuilder::new().buffer_capacity(1).from_writer(output);

        let mut key = vec![b'C'];
        key.extend([0; 33]); // 32-byte transaction ID followed by output index zero.
        let entry = (
            ChainStateKey::from_u8(&key),
            hex::decode(COINS[0].value_hex).unwrap(),
        );
        let error = export_utxos(&cli, [entry], &mut writer).unwrap_err();

        assert!(format!("{error:#}").contains("simulated output failure"));
        if !address_totals {
            assert!(error.to_string().contains("Failed to process UTXO 1"));
        }
    }
}
