use crate::utils::hex::append_hex;

#[test]
fn appends_all_byte_values_without_replacing_existing_text() {
    let bytes: Vec<u8> = (0..=255).collect();
    let mut output = String::from("prefix:");
    append_hex(&mut output, &bytes);
    assert_eq!(output, format!("prefix:{}", hex::encode(&bytes)));
    append_hex(&mut output, &[]);
    assert_eq!(output, format!("prefix:{}", hex::encode(&bytes)));
}
