use std::collections::HashMap;

use anyhow::Context;
use csv::Writer;

use crate::address_filter::{AddressFilter, AddressTypes};
use crate::bitcoin::address::AddressPayload;
use crate::bitcoin::utxo::UtxoValue;
use crate::utils::cli::{AddressSort, Cli};

pub(crate) enum AddressTotals {
    ByType {
        filter: AddressTypes,
        totals: HashMap<AddressPayload, u64>,
    },
    ByPrefix {
        prefix: String,
        totals: HashMap<String, u64>,
    },
}

impl AddressTotals {
    pub(crate) fn new(filter: &AddressFilter) -> Self {
        match filter {
            AddressFilter::Types(filter) => Self::ByType {
                filter: *filter,
                totals: HashMap::new(),
            },
            AddressFilter::Text(prefix) => Self::ByPrefix {
                prefix: prefix.clone(),
                totals: HashMap::new(),
            },
        }
    }

    pub(crate) fn add(&mut self, utxo: &UtxoValue, cli: &Cli) -> anyhow::Result<()> {
        let script_type = utxo.get_script_type();
        let Some(address) = utxo.address_payload(script_type, cli.include_p2pk) else {
            return Ok(());
        };
        let total = match self {
            Self::ByType { filter, totals } => {
                if !filter.matches(&address) {
                    return Ok(());
                }
                totals.entry(address).or_default()
            }
            Self::ByPrefix { prefix, totals } => {
                let Some(address) = address.encode(cli.testnet).ok() else {
                    return Ok(());
                };
                if !address.starts_with(prefix.as_str()) {
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

    pub(crate) fn write_output(
        self,
        cli: &Cli,
        writer: &mut Writer<impl std::io::Write>,
    ) -> anyhow::Result<()> {
        match self {
            Self::ByType { totals, .. } => {
                let rows = totals
                    .into_iter()
                    .filter(|(_, amount)| *amount >= cli.min_amount)
                    .map(|(address, amount)| {
                        let address = address.encode(cli.testnet)?;
                        Ok((address, amount))
                    });
                write_rows(rows, cli, writer)
            }
            Self::ByPrefix { totals, .. } => {
                let rows = totals
                    .into_iter()
                    .filter(|(_, amount)| *amount >= cli.min_amount)
                    .map(Ok);
                write_rows(rows, cli, writer)
            }
        }
    }
}

fn write_rows(
    rows: impl Iterator<Item = anyhow::Result<(String, u64)>>,
    cli: &Cli,
    writer: &mut Writer<impl std::io::Write>,
) -> anyhow::Result<()> {
    if let Some(order) = cli.sort {
        let mut rows = rows.collect::<anyhow::Result<Vec<_>>>()?;
        rows.sort_unstable_by(
            |(left_address, left_amount), (right_address, right_amount)| match order {
                AddressSort::Address => left_address.cmp(right_address),
                AddressSort::Amount => right_amount
                    .cmp(left_amount)
                    .then_with(|| left_address.cmp(right_address)),
            },
        );
        for (address, amount) in rows {
            write_address(&address, amount, cli.addresses_only, writer)?;
        }
    } else {
        for row in rows {
            let (address, amount) = row?;
            write_address(&address, amount, cli.addresses_only, writer)?;
        }
    }
    Ok(())
}

fn write_address(
    address: &str,
    amount: u64,
    addresses_only: bool,
    writer: &mut Writer<impl std::io::Write>,
) -> anyhow::Result<()> {
    if addresses_only {
        writer.write_record([address])?;
    } else {
        writer.write_record([address, &amount.to_string()])?;
    }
    Ok(())
}
