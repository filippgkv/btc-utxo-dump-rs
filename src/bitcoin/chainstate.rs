use anyhow::{ensure, Context, Result};
use db_key::Key;
use leveldb::database::Database;
use leveldb::iterator::Iterable;
use leveldb::options::Options;
use leveldb::options::ReadOptions;
use std::path::Path;

use super::btc_leveldb::read_varint;
use crate::utils::hex::append_hex;

pub struct ChainStateDB(Database<ChainStateKey>);

impl ChainStateDB {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let mut options = Options::new();
        options.create_if_missing = false;

        let db = Database::open(path, options)
            .map_err(|e| anyhow::anyhow!("Failed to open database at {}: {}", path.display(), e))?;

        Ok(Self(db))
    }

    pub fn iter(&self) -> impl Iterator<Item = (ChainStateKey, Vec<u8>)> + '_ {
        let read_options = ReadOptions::new();
        self.0.iter(read_options)
    }
}

#[derive(Debug)]
pub struct ChainStateKey(Vec<u8>);

impl Key for ChainStateKey {
    fn from_u8(key: &[u8]) -> Self {
        Self(key.to_vec())
    }

    fn as_slice<T, F: Fn(&[u8]) -> T>(&self, f: F) -> T {
        f(&self.0)
    }
}

impl ChainStateKey {
    pub fn first_byte(&self) -> Option<u8> {
        self.0.first().copied()
    }

    pub fn txid(&self) -> Result<String> {
        let mut output = String::with_capacity(64);
        self.append_txid(&mut output)?;
        Ok(output)
    }

    pub fn append_txid(&self, output: &mut String) -> Result<()> {
        ensure!(self.first_byte() == Some(b'C'), "Not a UTXO key");
        let bytes = self.0.get(1..33).context("Truncated UTXO transaction ID")?;
        let mut txid = [0; 32];
        txid.copy_from_slice(bytes);
        txid.reverse();
        append_hex(output, &txid);
        Ok(())
    }

    pub fn vout(&self) -> Result<u32> {
        ensure!(self.first_byte() == Some(b'C'), "Not a UTXO key");
        let mut bytes = self.0.get(33..).context("Truncated UTXO key")?;
        let vout = read_varint(&mut bytes).context("Invalid UTXO output index")?;
        ensure!(bytes.is_empty(), "Trailing bytes in UTXO key");
        u32::try_from(vout).context("UTXO output index exceeds u32")
    }
}

pub fn deobfuscate_value(value: &[u8], obfuscation_key: &[u8]) -> Vec<u8> {
    let mut result = value.to_vec();
    deobfuscate_in_place(&mut result, obfuscation_key);
    result
}

pub fn deobfuscate_in_place(value: &mut [u8], obfuscation_key: &[u8]) {
    if obfuscation_key.is_empty() {
        return;
    }

    for (i, byte) in value.iter_mut().enumerate() {
        *byte ^= obfuscation_key[i % obfuscation_key.len()];
    }
}
