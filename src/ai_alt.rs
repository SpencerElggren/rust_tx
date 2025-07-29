// ChatGPT generated solution, with edits to make it actually work
// main.rs made with minimal cursor AI help, then this generated from
// PDF and main.rs solution
use std::collections::HashMap;
use std::env;
use std::io::{self};
use std::fs::File;
use csv::{ReaderBuilder, WriterBuilder};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Deserialize, Clone)]
struct Transaction {
    #[serde(rename = "type")]
    tx_type: TransactionType,
    client: u16,
    tx: u32,
    amount: Option<Decimal>,
}

// AI wanted to add disputed: bool
// unnecessary as if held > 0, it is disputed
#[derive(Debug)]
struct Account {
    client: u16,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool,
}

impl Account {
    fn new(client_id: u16) -> Self {
        Account {
            client: client_id,
            available: Decimal::new(0, 4),
            held: Decimal::new(0, 4),
            total: Decimal::new(0, 4),
            locked: false,
        }
    }
}

#[derive(Serialize)]
struct AccountOutput {
    client: u16,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool,
}

// Main() too busy/long
// harder to debug/add in future

fn alt_main() -> io::Result<()> {

    // no change to input
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <input.csv>", args[0]);
        std::process::exit(1);
    }
    let input_file = &args[1];

    let file = File::open(input_file)?;
    let mut rdr = ReaderBuilder::new().from_reader(file);

    // These are used to track accounts and previous tx within
    // main as simple hashmaps, rather than being contained within
    // a struct. It lengthens main() and is disconnected from other data
    let mut accounts: HashMap<u16, Account> = HashMap::new();
    let mut client_tx_map: HashMap<u32, Transaction> = HashMap::new();

    for result in rdr.deserialize() {
        let tx: Transaction = result?;
        
        // A lot of nested if and if let, readability not fantastic
        match tx.tx_type {
            TransactionType::Deposit => {
                if let Some(amount) = tx.amount {
                    let account = accounts.entry(tx.client).or_insert_with(|| Account::new(tx.client));
                    if !account.locked {
                        account.available += amount;
                        client_tx_map.insert(tx.tx, tx);
                    }
                }
            },

            TransactionType::Withdrawal => {
                if let Some(amount) = tx.amount {
                    if let Some(account) = accounts.get_mut(&tx.client) {
                        if !account.locked {
                            if account.available >= amount {
                                account.available -= amount;
                                client_tx_map.insert(tx.tx, tx);
                            }
                        }
                    }
                }
            },

            TransactionType::Dispute => {

                if let Some(referenced_tx) = client_tx_map.get(&tx.tx) {
                        if let Some(account) = accounts.get_mut(&tx.client) {
                            if !account.locked {
                                if let Some(amount) = referenced_tx.amount {
                                    account.available -= amount;
                                    account.held += amount;
                                }
                            }
                        }
                    }
                },

            TransactionType::Resolve => {
                if let Some(referenced_tx) = client_tx_map.get(&tx.tx) {
                        if let Some(account) = accounts.get_mut(&tx.client) {
                            if !account.locked {
                                if let Some(amount) = referenced_tx.amount {
                                    account.held -= amount;
                                    account.available += amount;
                                }
                            }
                        }
                    }
                },

            TransactionType::Chargeback => {
                if let Some(referenced_tx) = client_tx_map.get(&tx.tx) {
                        if let Some(account) = accounts.get_mut(&tx.client) {
                            if !account.locked {
                                if let Some(amount) = referenced_tx.amount {
                                    account.held -= amount;
                                    account.locked = true;
                                }
                            }
                        }
                    }
                },
        }
    }

    // writer dependent on the previous hashmaps made within main()
    // confirms total as adding held and available, rather than relying on math
    // done within transactions.
    let mut wtr = WriterBuilder::new().from_writer(io::stdout());
    for (client, account) in accounts {
        let total = account.available + account.held;
        let output = AccountOutput {
            client,
            available: account.available,
            held: account.held,
            total,
            locked: account.locked,
        };
        wtr.serialize(output)?;
    }
    wtr.flush()?;

    Ok(())
}
