use std::fmt::Write;

use csv::Writer;

use crate::bitcoin::chainstate::{deobfuscate_in_place, ChainStateKey};
use crate::bitcoin::utxo::UtxoValue;
use crate::utils::cli::Cli;
use crate::utils::fields::FieldIndices;
use crate::utils::hex::append_hex;

pub(crate) struct CsvOutput<'a> {
    cli: &'a Cli,
    indices: &'a FieldIndices,
    parse_values: bool,
    decompress_keys: bool,
    record: Vec<String>,
}

impl<'a> CsvOutput<'a> {
    pub(crate) fn new(cli: &'a Cli) -> Self {
        Self {
            cli,
            indices: cli.fields.indices(),
            parse_values: cli.fields.indices().needs_utxo_parsing(),
            decompress_keys: cli.fields.indices().needs_decompression(cli.include_p2pk),
            record: vec![String::new(); cli.fields.ordered().len()],
        }
    }

    pub(crate) fn write_utxo(
        &mut self,
        utxo_count: u64,
        key: &ChainStateKey,
        value: &mut [u8],
        obfuscation_key: &[u8],
        writer: &mut Writer<impl std::io::Write>,
    ) -> anyhow::Result<()> {
        for field in &mut self.record {
            field.clear();
        }

        // Transaction ID and output index come from the database key.
        if let Some(position) = self.indices.txid {
            key.append_txid(&mut self.record[position])?;
        }
        if let Some(position) = self.indices.vout {
            write!(&mut self.record[position], "{}", key.vout()?)?;
        }

        // Decode the value only when a requested column needs it.
        if self.parse_values {
            deobfuscate_in_place(value, obfuscation_key);
            let utxo = UtxoValue::parse(value, self.decompress_keys)?;
            self.fill_value_fields(&utxo)?;
        }

        if let Some(position) = self.indices.count {
            write!(&mut self.record[position], "{}", utxo_count)?;
        }

        if self.cli.verbose && !self.cli.quiet {
            println!("{}", self.record.join(" "));
        }
        writer.write_record(&self.record)?;
        Ok(())
    }

    fn fill_value_fields(&mut self, utxo: &UtxoValue) -> anyhow::Result<()> {
        if let Some(position) = self.indices.height {
            write!(&mut self.record[position], "{}", utxo.height)?;
        }
        if let Some(position) = self.indices.coinbase {
            self.record[position].push(if utxo.coinbase { '1' } else { '0' });
        }
        if let Some(position) = self.indices.amount {
            write!(&mut self.record[position], "{}", utxo.amount)?;
        }
        if let Some(position) = self.indices.nsize {
            write!(&mut self.record[position], "{}", utxo.nsize)?;
        }
        if let Some(position) = self.indices.script {
            append_hex(&mut self.record[position], &utxo.script);
        }

        if self.indices.script_type.is_some() || self.indices.address.is_some() {
            let script_type = utxo.get_script_type();
            if let Some(position) = self.indices.script_type {
                write!(&mut self.record[position], "{}", script_type)?;
            }
            if let Some(position) = self.indices.address {
                let address = utxo.get_address_with_type(
                    script_type,
                    self.cli.testnet,
                    self.cli.include_p2pk,
                );
                if let Some(address) = address {
                    self.record[position].push_str(&address);
                }
            }
        }
        Ok(())
    }
}
