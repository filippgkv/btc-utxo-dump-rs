use anyhow::{anyhow, Result};
use secp256k1::PublicKey;

/// Decompress a public key that was compressed for storage in the UTXO database.
/// This is specifically for P2PK scripts where nsize is 4 or 5, indicating the key
/// was originally uncompressed but was compressed for storage.
///
/// nsize == 4: Y coordinate is even
/// nsize == 5: Y coordinate is odd
pub fn decompress_public_key(compressed: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
    if compressed.len() < 33 {
        return Err(anyhow!(
            "Compressed pubkey too short: {} bytes",
            compressed.len()
        ));
    }

    // Create a compressed public key in format: 02/03 || x
    let mut compressed_key = [0; 33];
    // 0x05 means odd Y, use 0x03 prefix. 0x04 means even Y, use 0x02 prefix
    compressed_key[0] = if compressed[0] == 0x05 { 0x03 } else { 0x02 };
    compressed_key[1..].copy_from_slice(&compressed[1..33]);

    let pubkey = PublicKey::from_byte_array_compressed(compressed_key)?;
    Ok(pubkey.serialize_uncompressed().to_vec())
}
