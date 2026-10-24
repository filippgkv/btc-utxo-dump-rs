use std::collections::HashMap;

use anyhow::Context;
use csv::Writer;

use crate::bitcoin::address::AddressPayload;
use crate::bitcoin::utxo::UtxoValue;
use crate::utils::cli::Cli;

pub(crate) enum AddressTotals {
    Compact(HashMap<AddressPayload, u64>),
    Filtered(HashMap<String, u64>),
}

impl AddressTotals {
    pub(crate) fn new(prefix: &str) -> Self {
        if prefix.is_empty() {
            Self::Compact(HashMap::new())
        } else {
            Self::Filtered(HashMap::new())
        }
    }

    pub(crate) fn add(&mut self, utxo: &UtxoValue, cli: &Cli) -> anyhow::Result<()> {
        let script_type = utxo.get_script_type();
        let total = match self {
            Self::Compact(totals) => {
                let Some(address) = utxo.address_payload(script_type, cli.include_p2pk) else {
                    return Ok(());
                };
                totals.entry(address).or_default()
            }
            Self::Filtered(totals) => {
                let Some(address) =
                    utxo.get_address_with_type(script_type, cli.testnet, cli.include_p2pk)
                else {
                    return Ok(());
                };
                if !address.starts_with(&cli.prefix) {
                    return Ok(());
                }
                totals.entry(address).or_default()
            }
        };
        *total = total
            .checked_add(utxo.amount)
            .context("Address total exceeds u64")?;
        Ok(())
    }

    pub(crate) fn write_csv(
        &self,
        testnet: bool,
        writer: &mut Writer<impl std::io::Write>,
    ) -> anyhow::Result<()> {
        match self {
            Self::Compact(totals) => {
                for (address, amount) in totals {
                    writer.write_record([address.encode(testnet)?, amount.to_string()])?;
                }
            }
            Self::Filtered(totals) => {
                for (address, amount) in totals {
                    writer.write_record([address, &amount.to_string()])?;
                }
            }
        }
        Ok(())
    }
}
