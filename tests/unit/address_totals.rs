use std::collections::HashMap;

use crate::address_totals::AddressTotals;
use crate::utils::cli::Cli;

#[test]
fn amount_sort_handles_u64_balances_and_breaks_ties_alphabetically() {
    let low_address = "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH";
    let first_tied_address = "1EHNa6Q4Jz2uvNExL497mE43ikXhwF6kZm";
    let second_tied_address = "1GrLCmVQXoyJXaPJQdqssNqwxvha1eUo2E";
    let totals = AddressTotals::Filtered(HashMap::from([
        (low_address.to_owned(), 1),
        (second_tied_address.to_owned(), u64::MAX),
        (first_tied_address.to_owned(), u64::MAX),
    ]));
    let cli = Cli::try_parse_from([
        "dump",
        "--address-totals",
        "--addresses-only",
        "--sort",
        "amount",
    ])
    .unwrap();
    let mut writer = csv::Writer::from_writer(Vec::new());
    totals.write_output(&cli, &mut writer).unwrap();
    let contents = String::from_utf8(writer.into_inner().unwrap()).unwrap();
    assert_eq!(
        contents,
        format!("{first_tied_address}\n{second_tied_address}\n{low_address}\n")
    );
}
