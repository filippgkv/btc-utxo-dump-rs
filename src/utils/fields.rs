use std::str::FromStr;

use anyhow::{bail, ensure, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Count,
    Txid,
    Vout,
    Height,
    Coinbase,
    Amount,
    Nsize,
    Script,
    Type,
    Address,
}

impl Field {
    pub fn name(self) -> &'static str {
        match self {
            Self::Count => "count",
            Self::Txid => "txid",
            Self::Vout => "vout",
            Self::Height => "height",
            Self::Coinbase => "coinbase",
            Self::Amount => "amount",
            Self::Nsize => "nsize",
            Self::Script => "script",
            Self::Type => "type",
            Self::Address => "address",
        }
    }
}

impl FromStr for Field {
    type Err = anyhow::Error;

    fn from_str(name: &str) -> Result<Self> {
        match name {
            "count" => Ok(Self::Count),
            "txid" => Ok(Self::Txid),
            "vout" => Ok(Self::Vout),
            "height" => Ok(Self::Height),
            "coinbase" => Ok(Self::Coinbase),
            "amount" => Ok(Self::Amount),
            "nsize" => Ok(Self::Nsize),
            "script" => Ok(Self::Script),
            "type" => Ok(Self::Type),
            "address" => Ok(Self::Address),
            _ => bail!("Invalid field: {}", name),
        }
    }
}

#[derive(Debug, Default)]
pub struct FieldSelection {
    ordered: Vec<Field>,
    indices: FieldIndices,
}

impl FieldSelection {
    pub fn ordered(&self) -> &[Field] {
        &self.ordered
    }

    pub fn headers(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.ordered.iter().map(|field| field.name())
    }

    pub fn indices(&self) -> &FieldIndices {
        &self.indices
    }
}

impl FromStr for FieldSelection {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self> {
        let mut ordered = Vec::new();
        for name in input.split(',').map(str::trim) {
            let field = name.parse::<Field>()?;
            ensure!(!ordered.contains(&field), "Duplicate field: {}", name);
            ordered.push(field);
        }
        let indices = FieldIndices::new(&ordered);
        Ok(Self { ordered, indices })
    }
}

#[derive(Debug, Default)]
pub struct FieldIndices {
    pub count: Option<usize>,
    pub txid: Option<usize>,
    pub vout: Option<usize>,
    pub height: Option<usize>,
    pub coinbase: Option<usize>,
    pub amount: Option<usize>,
    pub nsize: Option<usize>,
    pub script: Option<usize>,
    pub script_type: Option<usize>,
    pub address: Option<usize>,
}

impl FieldIndices {
    fn new(fields: &[Field]) -> Self {
        let mut indices = Self::default();
        for (position, field) in fields.iter().enumerate() {
            let destination = match field {
                Field::Count => &mut indices.count,
                Field::Txid => &mut indices.txid,
                Field::Vout => &mut indices.vout,
                Field::Height => &mut indices.height,
                Field::Coinbase => &mut indices.coinbase,
                Field::Amount => &mut indices.amount,
                Field::Nsize => &mut indices.nsize,
                Field::Script => &mut indices.script,
                Field::Type => &mut indices.script_type,
                Field::Address => &mut indices.address,
            };
            *destination = Some(position);
        }
        indices
    }

    pub fn needs_utxo_parsing(&self) -> bool {
        self.height.is_some()
            || self.coinbase.is_some()
            || self.amount.is_some()
            || self.nsize.is_some()
            || self.script.is_some()
            || self.script_type.is_some()
            || self.address.is_some()
    }

    pub fn needs_decompression(&self, include_p2pk: bool) -> bool {
        self.script.is_some() || (self.address.is_some() && include_p2pk)
    }
}
