use anyhow::{ensure, Context};
use csv::Writer;

use crate::address_totals::AddressTotals;
use crate::bitcoin::chainstate::{deobfuscate_in_place, ChainStateKey};
use crate::bitcoin::utxo::UtxoValue;
use crate::csv_output::CsvOutput;
use crate::utils::cli::Cli;

pub(crate) const UTXO_PREFIX: u8 = 0x43;
pub(crate) const OBFUSCATION_KEY_PREFIX: u8 = 0x0E;
const REPORT_PROGRESS_INTERVAL: u64 = 100_000;

pub(crate) fn export_utxos(
    cli: &Cli,
    entries: impl IntoIterator<Item = (ChainStateKey, Vec<u8>)>,
    writer: &mut Writer<impl std::io::Write>,
) -> anyhow::Result<u64> {
    let mut utxo_count = 0;
    let mut obfuscation_key = Vec::new();
    let mut address_totals = AddressTotals::new(&cli.address_filter);
    let mut output = CsvOutput::new(cli);

    if cli.address_totals {
        if !cli.addresses_only {
            writer.write_record(["address", "amount"])?;
        }
    } else {
        writer.write_record(cli.fields.headers())?;
    }

    for (key, mut value) in entries {
        match key.first_byte().context("Empty chainstate key")? {
            UTXO_PREFIX => {
                let next_count = utxo_count + 1;
                let result = if cli.address_totals {
                    add_utxo_amount(cli, &mut address_totals, &mut value, &obfuscation_key)
                } else {
                    output.write_utxo(next_count, &key, &mut value, &obfuscation_key, writer)
                };
                result.with_context(|| format!("Failed to process UTXO {}", next_count))?;
                utxo_count = next_count;

                if !cli.quiet && utxo_count % REPORT_PROGRESS_INTERVAL == 0 {
                    println!("{} utxos processed", utxo_count);
                }
                if cli.max_utxos > 0 && utxo_count >= cli.max_utxos {
                    break;
                }
            }
            OBFUSCATION_KEY_PREFIX => {
                let (&length, key) = value
                    .split_first()
                    .context("Missing obfuscation key length")?;
                ensure!(
                    key.len() == usize::from(length),
                    "Invalid obfuscation key length"
                );
                obfuscation_key = key.to_vec();
            }
            _ => continue,
        }
    }

    if cli.address_totals {
        address_totals.write_output(cli, writer)?;
    }
    Ok(utxo_count)
}

pub(crate) fn add_utxo_amount(
    cli: &Cli,
    address_totals: &mut AddressTotals,
    value: &mut [u8],
    obfuscation_key: &[u8],
) -> anyhow::Result<()> {
    deobfuscate_in_place(value, obfuscation_key);
    let utxo = UtxoValue::parse(value, cli.include_p2pk)?;

    address_totals.add(&utxo, cli)
}
