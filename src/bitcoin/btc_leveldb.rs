use anyhow::{ensure, Context, Result};

/// Read Bitcoin Core's MSB base-128 VARINT, advancing the input slice.
/// Continuation bytes add one to the accumulated value; this is not LEB128.
pub fn read_varint(input: &mut &[u8]) -> Result<u64> {
    let mut value = 0u64;

    loop {
        let byte = *input.first().context("Truncated VARINT")?;
        *input = &input[1..];

        // Make room for the next seven bits without discarding existing bits.
        ensure!(value <= u64::MAX >> 7, "VARINT exceeds u64");
        value = (value << 7) | u64::from(byte & 0x7f);

        if byte & 0x80 == 0 {
            return Ok(value);
        }

        // Core's encoding adds one for each continuation byte.
        ensure!(value < u64::MAX, "VARINT exceeds u64");
        value += 1;
    }
}

pub fn decompress_amount(compressed: u64) -> Result<u64> {
    if compressed == 0 {
        return Ok(0);
    }

    // The last decimal digit stores the number of trailing zeroes.
    let value = compressed - 1;
    let exponent = value % 10;
    let mantissa = value / 10;

    let mut amount;
    if exponent < 9 {
        // Restore the nonzero digit removed during compression.
        let last_digit = mantissa % 9 + 1;
        amount = (mantissa / 9)
            .checked_mul(10)
            .context("Decompressed amount exceeds u64")?;
        amount = amount
            .checked_add(last_digit)
            .context("Decompressed amount exceeds u64")?;
    } else {
        amount = mantissa
            .checked_add(1)
            .context("Decompressed amount exceeds u64")?;
    }

    for _ in 0..exponent {
        amount = amount
            .checked_mul(10)
            .context("Decompressed amount exceeds u64")?;
    }
    Ok(amount)
}
