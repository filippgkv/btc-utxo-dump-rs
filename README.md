# Bitcoin UTXO Dump (Rust)

A Rust port of [bitcoin-utxo-dump](https://github.com/in3rsha/bitcoin-utxo-dump) -
a tool to get a list of every unspent bitcoin in the blockchain.

**Warning:** The original tool had the possibility of corrupting the chainstate database.
I was not able to replicate that with this project, but it may still be a good idea to clone the
database in order to not need to reindex in the case that that does happen. Also, make sure that
Bitcoin Core / bitcoind is not running when doing the utxo dump as LevelDB can only be accessed
by [one process at a time](https://github.com/google/leveldb?tab=readme-ov-file#limitations).

## About

This program iterates over each entry in Bitcoin Core's `chainstate` [LevelDB](https://github.com/google/leveldb) database. It decompresses and decodes the data, producing a CSV dump of all [UTXO](https://learnmeabitcoin.com/technical/transaction/utxo/)s (unspent transaction outputs). The original [bitcoin-utxo-dump](https://github.com/in3rsha/bitcoin-utxo-dump) repo has many helpful comments and diagrams describing the various fields, definitely check that out if interested in learning more.

### Example CSV Results:

```
count,txid,vout,amount,type,address
1,033e83e3204b0cc28724e147f6fd140529b2537249f9c61c9de9972750030000,0,65279,p2pkh,1KaPHfvVWNZADup3Yc26SfVdkTDvvHySVX
2,e1c9467a885a156e56a29d9c854e65674d581ad75611b02290454b4862060000,1,9466355,p2pkh,1LpCmEejWLNfZigApMPwUY9nZTS8NTJCNS
3,a1f28c43f1f3d4821d0db42707737ea90616613099234f905dfc6ae2b4060000,1,339500,p2pkh,1FuphZ7xVPGrxthQT1S8X7nNQNByYxAT3V
4,818f5b9e3ede69da765d4c24684e813057c9b1f059e098661369b0a2ee060000,0,300000,p2pkh,18Y9yhjU9g2jjJmvaUy7TmUNZH9iPzQ4dd
5,d2f5e439152d076593a145581f8d76ea2e48ed155285b9a245cd42dd06070000,0,100000,p2pkh,1EKHTvovYWHfUJ6i9vsoidyTPQauCPH1qC
```

## Installation

First, ensure you have a full copy of the blockchain

Then, with Rust installed, you can build the tool:

```bash
cargo build --release --locked
```

This will compile the program into `target/release/btc-utxo-dump-rs`

`Cargo.lock` preserves the tested dependency versions. In particular, `cmake`
0.1.54 and `cc` 1.2.17 are retained because newer build dependencies cause the
bundled LevelDB build to enable the unsupported `-Wthread-safety` flag with GCC.
Preserve these versions when updating dependencies.

## Usage

Basic usage:

```bash
./btc-utxo-dump-rs
```

This will dump all UTXOs to `utxodump.csv` in the current directory.

### Options

```bash
./btc-utxo-dump-rs --help

Dumps Bitcoin Core chainstate UTXO set to CSV

Usage: btc-utxo-dump-rs [OPTIONS]

Options:
  -d, --db <DIR>               Bitcoin Core chainstate LevelDB directory [default: ~/.bitcoin/chainstate/]
  -o, --output <FILE>          Output file path [default: utxodump.csv]
  -a, --address-totals         Sums totals for each address. If used, the --fields option is ignored.
      --min-amount <SATOSHIS>  Only output addresses with a summed balance of at least this many satoshis [default: 0]
      --addresses-only         Write one address per line, without a header or amounts
      --sort <SORT>            Sort totals by address (A-Z) or amount (highest first); ties by address [possible values: address, amount]
  -p, --prefix <PREFIX>        Used with --address-totals. Only include UTXOs for addresses starting with this prefix. [default: ""]
  -f, --fields <FIELDS>        Comma-separated list of fields to include in output: count,txid,vout,height,coinbase,amount,nsize,script,type,address [default: count,txid,vout,amount,type,address]
      --testnet                Use testnet network parameters
  -v, --verbose                Print UTXOs to stdout while processing
  -m, --max-utxos <COUNT>      Maximum number of UTXOs to process (0 for unlimited) [default: 0]
  -q, --quiet                  Quiet mode, no output
      --include-p2pk           Convert P2PK scripts to addresses
  -h, --help                   Print help
  -V, --version                Print version
```

Available fields:
- `count`: Number of UTXOs in the database
- `txid`: Transaction ID
- `vout`: Output index in the transaction
- `height`: Block height where the transaction was mined
- `coinbase`: Whether the output is from a coinbase transaction
- `amount`: Value in satoshis
- `script`: Locking script details
- `type`: Type of locking script (P2PK, P2PKH, P2SH, etc.)
- `address`: Bitcoin address the output is locked to

Example with specific fields:
```bash
./btc-utxo-dump-rs --fields count,txid,vout,address
```

## How it Works

The tool reads entries from the LevelDB chainstate database, which contains all unspent transaction outputs. The data is stored in an obfuscated and compressed format to save space and avoid antivirus false positives. This tool:

1. Reads the obfuscation key from the database
2. Iterates through all UTXO entries
3. Deobfuscates and decompresses each entry
4. Decodes the bitcoin script to determine the type and address
5. Writes the results to a CSV file

## Performance

Tested on Feb 16, 2025. Rust version runs in about 1/4 time as compared to Go version.

```bash
# Time Go version
$ time ./bitcoin-utxo-dump -db /mnt/Media/btc/chainstate -f count,txid,vout,height,amount,coinbase,nsize,script,type,address -o utxo-go.csv -quiet

real  13m28.834s
user  15m13.496s
sys   1m2.890s

# Time Rust version
$ time ./btc-utxo-dump-rs --db /mnt/Media/btc/chainstate -f count,txid,vout,height,amount,coinbase,nsize,script,type,address -o utxo-rs.csv --quiet

real  3m41.636s
user  3m17.826s
sys   0m14.865s

# Compare file sizes
$ du -ah
3.9M  ./bitcoin-utxo-dump
1.7M  ./btc-utxo-dump-rs
34G   ./utxo-go.csv
34G   ./utxo-rs.csv
13G   ./chainstate

# Compare output file bytes
$ cmp utxo-go.csv utxo-rs.csv --verbose
$ # no output shows files are identical
```

## Requirements

- Bitcoin Core 0.15.1 or later
- Rust 1.88 or later (tested with Rust 1.98.1)
