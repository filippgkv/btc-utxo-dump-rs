mod address_filter;
mod address_totals;
pub mod bitcoin;
mod csv_output;
mod dump;
pub mod utils;

use csv::WriterBuilder;
use std::fs::File;

use bitcoin::chainstate::ChainStateDB;
use dump::export_utxos;
use utils::cli::parse_cli;

fn main() -> anyhow::Result<()> {
    let cli = parse_cli()?;
    let db = ChainStateDB::open(&cli.db)?;
    let outfile = File::create(&cli.output)?;
    let mut writer = WriterBuilder::new()
        .buffer_capacity(2048 * 1024)
        .from_writer(outfile);

    let utxo_count = export_utxos(&cli, db.iter(), &mut writer)?;
    writer.flush()?;
    if !cli.quiet {
        println!("\nTotal UTXOs: {}", utxo_count);
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
