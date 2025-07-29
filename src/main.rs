// Cursor AI used for autocompletion, minimally
use std::collections::HashMap;
use std::env;
use std::io::{self};
use std::fs::File;
use csv::{ReaderBuilder, WriterBuilder};
// Decimal for currency calculations for precision and accuracy, at the cost of performance
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// Transaction type enum
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

// Input transaction struct
#[derive(Debug, Deserialize, Clone)]
struct Transaction {
    #[serde(rename = "type")]
    tx_type: TransactionType,
    client: u16,
    tx: u32,
    amount: Option<Decimal>,
    // disputed added to track, defaults to false
    #[serde(default)]
    disputed: bool
}

// Client account state
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

// Output record struct
#[derive(Serialize)]
struct AccountOutput {
    client: u16,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool,
}

struct TransactionProcessor {
    accounts: HashMap<u16, Account>,
    // transaction hashmap uses both client and tx id to assure both in retrieval
    // ideally, in a production context, transactions would be retrieved through a database
    // ie in a relational database with all tx of a client attached to the client id
    transactions: HashMap<(u16, u32), Transaction>, 
}

impl TransactionProcessor {
    fn new() -> Self {
        TransactionProcessor {
            accounts: HashMap::new(),
            transactions: HashMap::new(),
        }
    }

    fn handle(&mut self, tx: Transaction) {
        match tx.tx_type {
            TransactionType::Deposit => self.handle_deposit(tx),
            TransactionType::Withdrawal => self.handle_withdrawal(tx),
            TransactionType::Dispute => self.handle_dispute(tx),
            TransactionType::Resolve => self.handle_resolve(tx),
            TransactionType::Chargeback => self.handle_chargeback(tx),
        }
    }

    fn handle_deposit(&mut self, tx: Transaction) {
        let Some(amount) = tx.amount else { return };

        let account = self
            .accounts
            .entry(tx.client)
            .or_insert_with(|| Account::new(tx.client));

        if account.locked {
            return;
        }

        account.available += amount;
        account.total += amount;

        self.transactions.insert((tx.client, tx.tx), tx);
    }

    fn handle_withdrawal(&mut self, tx: Transaction) {
        let Some(amount) = tx.amount else { return };

        let account = self
        .accounts
        .entry(tx.client)
        .or_insert_with(|| Account::new(tx.client));

        if account.locked || account.available < amount {
            return;
        }

        account.available -= amount;
        account.total -= amount;

        self.transactions.insert((tx.client, tx.tx), tx);
    }

    fn handle_dispute(&mut self, tx: Transaction) {
        let Some(referenced_tx) = self.transactions.get_mut(&(tx.client, tx.tx)) else { return };
        let Some(amount) = referenced_tx.amount else { return };
        let account = self
            .accounts
            .entry(tx.client)
            .or_insert_with(|| Account::new(tx.client));

        if account.locked || referenced_tx.disputed == true {
            return;
        }

        account.available -= amount;
        account.held += amount;
        referenced_tx.disputed = true;
    }

    fn handle_resolve(&mut self, tx: Transaction) {
        let Some(referenced_tx) = self.transactions.get_mut(&(tx.client, tx.tx)) else { return };
        let Some(amount) = referenced_tx.amount else { return };
        let account = self
            .accounts
            .entry(tx.client)
            .or_insert_with(|| Account::new(tx.client));

        if account.locked || referenced_tx.disputed == false {
            return;
        }

        account.held -= amount;
        account.available += amount;
        referenced_tx.disputed = false;
    }

    fn handle_chargeback(&mut self, tx: Transaction) {
        let Some(referenced_tx) = self.transactions.get(&(tx.client, tx.tx)) else { return };
        let Some(amount) = referenced_tx.amount else { return };
        let account = self
            .accounts
            .entry(tx.client)
            .or_insert_with(|| Account::new(tx.client));

        if account.locked || referenced_tx.disputed == false {
            return;
        }

        account.held -= amount;
        account.total -= amount;
        account.locked = true;
    }
}

fn main() -> io::Result<()> {

    // Streaming data from stdin
    // let mut rdr = ReaderBuilder::new().from_reader(io::stdin());

    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <input.csv>", args[0]);
        std::process::exit(1);
    }
    let input_file = &args[1];

    // Open and read the input CSV
    let file = File::open(input_file)?;
    let mut rdr = ReaderBuilder::new().from_reader(file);

    let mut processor = TransactionProcessor::new();

    for result in rdr.deserialize() {
        let tx: Transaction = result?;
        processor.handle(tx);
    }

    // Write output to stdout
    let mut wtr = WriterBuilder::new().from_writer(io::stdout());
    for account in processor.accounts.values() {
        let output = AccountOutput {
            client: account.client,
            available: account.available,
            held: account.held,
            total: account.total,
            locked: account.locked,
        };
        wtr.serialize(output)?;
    }
    wtr.flush()?;

    Ok(())
}