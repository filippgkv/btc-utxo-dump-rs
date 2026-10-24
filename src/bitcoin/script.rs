const OP_1: u8 = 0x51;
const OP_16: u8 = 0x60;
const OP_PUSHDATA1: u8 = 0x4c;
const OP_PUSHDATA2: u8 = 0x4d;
const OP_PUSHDATA4: u8 = 0x4e;
const OP_CHECKMULTISIG: u8 = 0xae;
const MAX_MULTISIG_KEYS: usize = 20;

/// Recognize: <required signatures> <public keys...> <key count> OP_CHECKMULTISIG.
pub(super) fn is_multisig(script: &[u8]) -> bool {
    if script.last() != Some(&OP_CHECKMULTISIG) {
        return false;
    }

    let Some((required_signatures, mut remaining)) = read_multisig_count(script) else {
        return false;
    };

    let mut actual_key_count = 0;
    while let Some(rest) = skip_public_key(remaining) {
        remaining = rest;
        actual_key_count += 1;
        if actual_key_count > MAX_MULTISIG_KEYS {
            return false;
        }
    }

    let Some((declared_key_count, remaining)) = read_multisig_count(remaining) else {
        return false;
    };

    let counts_match = actual_key_count == declared_key_count;
    let enough_keys = required_signatures <= actual_key_count;
    let ends_with_checkmultisig = remaining == [OP_CHECKMULTISIG];

    counts_match && enough_keys && ends_with_checkmultisig
}

/// Read a positive multisig count and return the rest of the script.
fn read_multisig_count(script: &[u8]) -> Option<(usize, &[u8])> {
    let opcode = *script.first()?;

    // Counts 1 through 16 use dedicated opcodes.
    if (OP_1..=OP_16).contains(&opcode) {
        let count = usize::from(opcode - OP_1 + 1);
        return Some((count, &script[1..]));
    }

    // Counts 17 through 20 push a single byte instead.
    if opcode == 1 {
        let count = usize::from(*script.get(1)?);
        if (17..=MAX_MULTISIG_KEYS).contains(&count) {
            return Some((count, &script[2..]));
        }
    }

    None
}

/// Skip one public key push, returning the remaining script if its shape is valid.
fn skip_public_key(script: &[u8]) -> Option<&[u8]> {
    let opcode = *script.first()?;
    let mut remaining = &script[1..];

    // A key can be pushed directly or with an explicit little-endian length.
    let key_size = match opcode {
        33 | 65 => usize::from(opcode),
        OP_PUSHDATA1 => {
            let size = usize::from(*remaining.first()?);
            remaining = &remaining[1..];
            size
        }
        OP_PUSHDATA2 => {
            let bytes = remaining.get(..2)?;
            let size = u16::from_le_bytes([bytes[0], bytes[1]]);
            remaining = &remaining[2..];
            usize::from(size)
        }
        OP_PUSHDATA4 => {
            let bytes = remaining.get(..4)?;
            let size = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            remaining = &remaining[4..];
            usize::try_from(size).ok()?
        }
        _ => return None,
    };

    let public_key = remaining.get(..key_size)?;
    let prefix = *public_key.first()?;
    let is_compressed = key_size == 33 && matches!(prefix, 0x02 | 0x03);
    let is_uncompressed = key_size == 65 && matches!(prefix, 0x04 | 0x06 | 0x07);
    if !is_compressed && !is_uncompressed {
        return None;
    }

    Some(&remaining[key_size..])
}
