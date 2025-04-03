use db_key::Key;
use leveldb::database::Database;
use leveldb::iterator::Iterable;
use leveldb::options::Options;
use leveldb::options::ReadOptions;
use std::path::Path;

use super::btc_leveldb::varint128_decode;

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
    pub fn first_byte(&self) -> u8 {
        self.0[0]
    }
    pub fn txid(&self) -> String {
        let txid_le = &self.0[1..33];
        let txid: Vec<u8> = txid_le.iter().rev().copied().collect();
        hex::encode(txid)
    }
    pub fn vout(&self) -> i64 {
        let vout_bytes = &self.0[33..];
        varint128_decode(vout_bytes)
    }
}

pub fn deobfuscate_value(value: &[u8], obfuscation_key: &[u8]) -> Vec<u8> {
    if obfuscation_key.is_empty() {
        return value.to_vec();
    }

    let mut result = Vec::with_capacity(value.len());
    for (i, &b) in value.iter().enumerate() {
        result.push(b ^ obfuscation_key[i % obfuscation_key.len()]);
    }
    result
}
