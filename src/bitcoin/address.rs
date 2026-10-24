use anyhow::Context;
use bech32::{hrp, segwit, Fe32};
use bs58;
use ripemd::Ripemd160;
use sha2::{Digest, Sha256};

pub fn hash160_to_address(hash: &[u8; 20], prefix: u8) -> String {
    let mut address = [0; 25];
    address[0] = prefix;
    address[1..21].copy_from_slice(hash);
    // Calculate double SHA256
    let hash = Sha256::digest(&address[..21]);
    let checksum = Sha256::digest(hash);
    // Append checksum
    address[21..].copy_from_slice(&checksum[..4]);
    // Convert to base58
    bs58::encode(address).into_string()
}

pub fn public_key_to_address(pubkey: &[u8], prefix: u8) -> String {
    let hash160 = public_key_hash160(pubkey);
    hash160_to_address(&hash160, prefix)
}

pub(crate) fn public_key_hash160(pubkey: &[u8]) -> [u8; 20] {
    Ripemd160::digest(Sha256::digest(pubkey)).into()
}

/// Address identity before network prefixes and text encoding are applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum AddressPayload {
    P2PKH([u8; 20]),
    P2SH([u8; 20]),
    P2WPKH([u8; 20]),
    P2WSH([u8; 32]),
    P2TR([u8; 32]),
}

impl AddressPayload {
    pub(crate) fn encode(self, testnet: bool) -> anyhow::Result<String> {
        match self {
            Self::P2PKH(hash) => Ok(hash160_to_address(&hash, if testnet { 0x6f } else { 0x00 })),
            Self::P2SH(hash) => Ok(hash160_to_address(&hash, if testnet { 0xc4 } else { 0x05 })),
            Self::P2WPKH(program) => segwit_address(0, &program, testnet),
            Self::P2WSH(program) => segwit_address(0, &program, testnet),
            Self::P2TR(program) => segwit_address(1, &program, testnet),
        }
    }
}

pub fn segwit_address(version: u8, program: &[u8], testnet: bool) -> anyhow::Result<String> {
    let network = if testnet { hrp::TB } else { hrp::BC };
    let version = Fe32::try_from(version).context("Invalid witness version")?;

    Ok(segwit::encode(network, version, program)?)
}
