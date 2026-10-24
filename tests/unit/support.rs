pub(super) fn encode_varint(mut value: u64) -> Vec<u8> {
    let mut bytes = vec![(value & 0x7f) as u8];
    while value > 0x7f {
        value = (value >> 7) - 1;
        bytes.push((value & 0x7f) as u8 | 0x80);
    }
    bytes.reverse();
    bytes
}
