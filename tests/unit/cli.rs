use crate::utils::cli::Cli;

#[test]
fn field_whitespace_is_handled_consistently() {
    let cli = Cli::try_parse_from(["dump", "--fields", " count , amount , vout "]).unwrap();
    let indices = cli.fields.indices();
    assert_eq!(indices.count, Some(0));
    assert_eq!(indices.amount, Some(1));
    assert_eq!(indices.vout, Some(2));
}

#[test]
fn rejects_duplicate_unknown_and_empty_fields() {
    for fields in ["amount,amount", "amount, amount ", "unknown", "", "count,"] {
        assert!(
            Cli::try_parse_from(["dump", "--fields", fields]).is_err(),
            "fields={fields:?}"
        );
    }
}

#[test]
fn address_totals_ignore_fields_as_documented() {
    let cli = Cli::try_parse_from(["dump", "--address-totals", "--fields", "ignored"]).unwrap();
    assert!(cli.fields.ordered().is_empty());
}
