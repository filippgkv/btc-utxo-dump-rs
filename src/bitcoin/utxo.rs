use anyhow::{ensure, Context};
use std::borrow::Cow;
use std::fmt;

use super::address::{public_key_hash160, AddressPayload};
use super::btc_leveldb::{decompress_amount, read_varint};
use super::pubkey::decompress_public_key;
use super::script::is_multisig;

#[derive(Debug, PartialEq)]
pub enum ScriptType {
    P2PKH,
    P2SH,
    P2PK,
    P2MS,
    P2WPKH,
    P2WSH,
    P2TR,
    NonStandard,
}

impl fmt::Display for ScriptType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptType::P2PKH => write!(f, "p2pkh"),
            ScriptType::P2SH => write!(f, "p2sh"),
            ScriptType::P2PK => write!(f, "p2pk"),
            ScriptType::P2MS => write!(f, "p2ms"),
            ScriptType::P2WPKH => write!(f, "p2wpkh"),
            ScriptType::P2WSH => write!(f, "p2wsh"),
            ScriptType::P2TR => write!(f, "p2tr"),
            ScriptType::NonStandard => write!(f, "non-standard"),
        }
    }
}

pub struct UtxoValue<'a> {
    pub height: u32,
    pub coinbase: bool,
    pub amount: u64,
    /// Compressed script tag (0..=5), or uncompressed script length plus six.
    pub nsize: u64,
    pub script: Cow<'a, [u8]>,
}

impl<'a> UtxoValue<'a> {
    pub fn parse(data: &'a [u8], need_decompression: bool) -> Result<Self, anyhow::Error> {
        let mut remaining = data;

        // Height and coinbase share the first VARINT.
        let height_and_coinbase =
            read_varint(&mut remaining).context("Invalid UTXO height/coinbase")?;
        let height = u32::try_from(height_and_coinbase >> 1).context("UTXO height exceeds u32")?;
        let coinbase = height_and_coinbase & 1 != 0;

        // The second VARINT stores the compressed satoshi amount.
        let compressed_amount = read_varint(&mut remaining).context("Invalid UTXO amount")?;
        let amount = decompress_amount(compressed_amount)?;

        // The final VARINT determines the script encoding and payload size.
        let script_encoding = remaining;
        let nsize = read_varint(&mut remaining).context("Invalid UTXO script size")?;

        let expected_size = match nsize {
            0 | 1 => 20,
            2..=5 => 32,
            size => usize::try_from(size - 6).context("UTXO script size exceeds usize")?,
        };
        ensure!(
            remaining.len() == expected_size,
            "Invalid UTXO script length: expected {}, got {}",
            expected_size,
            remaining.len()
        );

        // P2PK tags are single bytes that also serve as the public key prefix.
        let script_bytes = if (2..=5).contains(&nsize) {
            script_encoding
        } else {
            remaining
        };

        // Most scripts can borrow their bytes; expanded public keys need owned storage.
        let script = if need_decompression && (nsize == 4 || nsize == 5) {
            Cow::Owned(decompress_public_key(script_bytes)?)
        } else {
            Cow::Borrowed(script_bytes)
        };

        Ok(Self {
            height,
            coinbase,
            amount,
            nsize,
            script,
        })
    }

    pub fn get_script_type(&self) -> ScriptType {
        match self.nsize {
            0 => ScriptType::P2PKH,
            1 => ScriptType::P2SH,
            2..=5 => ScriptType::P2PK,
            28 if self.script.len() == 22 && self.script.starts_with(&[0, 20]) => {
                ScriptType::P2WPKH
            }
            40 if self.script.len() == 34 && self.script.starts_with(&[0, 32]) => ScriptType::P2WSH,
            40 if self.script.len() == 34 && self.script.starts_with(&[0x51, 32]) => {
                ScriptType::P2TR
            }
            _ if is_multisig(&self.script) => ScriptType::P2MS,
            _ => ScriptType::NonStandard,
        }
    }

    pub fn get_address_with_type(
        &self,
        script_type: ScriptType,
        testnet: bool,
        p2pk_addresses: bool,
    ) -> Option<String> {
        self.address_payload(script_type, p2pk_addresses)?
            .encode(testnet)
            .ok()
    }

    pub(crate) fn address_payload(
        &self,
        script_type: ScriptType,
        p2pk_addresses: bool,
    ) -> Option<AddressPayload> {
        let script = self.script.as_ref();
        match script_type {
            ScriptType::P2PKH => Some(AddressPayload::P2PKH(script.try_into().ok()?)),
            ScriptType::P2SH => Some(AddressPayload::P2SH(script.try_into().ok()?)),
            ScriptType::P2PK if p2pk_addresses => {
                Some(AddressPayload::P2PKH(public_key_hash160(script)))
            }
            ScriptType::P2WPKH => Some(AddressPayload::P2WPKH(script.get(2..)?.try_into().ok()?)),
            ScriptType::P2WSH => Some(AddressPayload::P2WSH(script.get(2..)?.try_into().ok()?)),
            ScriptType::P2TR => Some(AddressPayload::P2TR(script.get(2..)?.try_into().ok()?)),
            _ => None,
        }
    }
}
