use clap::Parser;
use std::path::PathBuf;

use super::fields::FieldSelection;

#[derive(Parser)]
#[command(
    name = "btc-utxo-dump",
    about = "Dumps Bitcoin Core's chainstate UTXO set to CSV",
    version,
    author
)]
struct RawCli {
    #[arg(
        short,
        long,
        value_name = "DIR",
        default_value = "~/.bitcoin/chainstate/",
        help = "Bitcoin Core chainstate LevelDB directory"
    )]
    db: PathBuf,

    #[arg(
        short,
        long,
        value_name = "FILE",
        default_value = "utxodump.csv",
        help = "Output CSV file path"
    )]
    output: PathBuf,

    #[arg(
        short,
        long,
        help = "Sums totals for each address. If used, the --fields option is ignored.",
        default_value = "false"
    )]
    address_totals: bool,

    #[arg(
        short,
        long,
        value_name = "PREFIX",
        default_value = "",
        help = "Used with --address-totals. Only include UTXOs for addresses starting with this prefix."
    )]
    prefix: String,

    #[arg(
        short,
        long,
        value_name = "FIELDS",
        default_value = "count,txid,vout,amount,type,address",
        help = "Comma-separated list of fields to include in output: count,txid,vout,height,coinbase,amount,nsize,script,type,address"
    )]
    fields: String,

    #[arg(long, help = "Use testnet network parameters", default_value = "false")]
    testnet: bool,

    #[arg(
        short,
        long,
        help = "Print UTXOs to stdout while processing",
        default_value = "false"
    )]
    verbose: bool,

    #[arg(
        short,
        long,
        value_name = "COUNT",
        default_value = "0",
        help = "Maximum number of UTXOs to process (0 for unlimited)"
    )]
    max_utxos: u64,

    #[arg(short, long, help = "Quiet mode, no output", default_value = "false")]
    quiet: bool,

    #[arg(
        long,
        help = "Convert P2PK scripts to addresses",
        default_value = "false"
    )]
    include_p2pk: bool,
}

pub struct Cli {
    pub db: PathBuf,
    pub output: PathBuf,
    pub address_totals: bool,
    pub prefix: String,
    pub fields: FieldSelection,
    pub testnet: bool,
    pub verbose: bool,
    pub max_utxos: u64,
    pub quiet: bool,
    pub include_p2pk: bool,
}

impl Cli {
    fn from_raw(raw: RawCli) -> anyhow::Result<Self> {
        let fields = if raw.address_totals {
            FieldSelection::default()
        } else {
            raw.fields.parse()?
        };
        Ok(Self {
            db: raw.db,
            output: raw.output,
            address_totals: raw.address_totals,
            prefix: raw.prefix,
            fields,
            testnet: raw.testnet,
            verbose: raw.verbose,
            max_utxos: raw.max_utxos,
            quiet: raw.quiet,
            include_p2pk: raw.include_p2pk,
        })
    }

    pub fn try_parse_from<I, T>(arguments: I) -> anyhow::Result<Self>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        Self::from_raw(RawCli::try_parse_from(arguments)?)
    }
}

pub fn parse_cli() -> anyhow::Result<Cli> {
    Cli::from_raw(RawCli::parse())
}
