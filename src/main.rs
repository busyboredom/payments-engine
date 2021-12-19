#![warn(clippy::pedantic)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]

mod account;
mod transaction;

use std::io;
use std::{collections::HashMap, env};

use account::Account;
use transaction::{Transaction, TxType};

fn main() {
    // Get file path from argument.
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        panic!("expected one argument, but received {}", args.len() - 1);
    }
    let transactions_path = &args[1];

    // Prepare csv reader.
    let mut transactions_reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(transactions_path)
        .expect("failed to read csv");

    // Read csv line by line, updating account balances as we go.
    let mut accounts: HashMap<u16, Account> = HashMap::new();
    for csv_row in transactions_reader.deserialize() {
        let transaction: Transaction = csv_row.expect("failed to parse csv row as Transaction");

        apply_transaction(&mut accounts, transaction);
    }

    // Write account details to standard output.
    let mut wtr = csv::Writer::from_writer(io::stdout());
    for account in accounts.values() {
        wtr.serialize(account)
            .expect("failed to write account details to stdout");
    }
    wtr.flush().expect("failed to flush output to stdout");
}

fn apply_transaction(accounts: &mut HashMap<u16, Account>, transaction: Transaction) {
    match transaction.tx_type {
        TxType::Deposit => {
            let account = accounts
                .entry(transaction.client)
                .or_insert_with(|| Account::new(transaction.client));

            account.available += transaction.amount;
            account.total += transaction.amount;
        }
        TxType::Withdrawal => {
            let account = accounts
                .entry(transaction.client)
                .or_insert_with(|| Account::new(transaction.client));

            if let Some(available) = account.available.checked_sub(transaction.amount) {
                account.available = available;
                account.total -= transaction.amount;
            };
        }
        _ => {}
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::apply_transaction;
    use crate::transaction::{Amount, Transaction, TxType};
    use crate::Account;

    #[test]
    fn deposit() {
        let mut accounts: HashMap<u16, Account> = HashMap::new();
        let transaction = Transaction {
            tx_type: TxType::Deposit,
            client: 1,
            id: 1,
            amount: Amount::from(12345.67891),
        };

        apply_transaction(&mut accounts, transaction);

        assert_eq!(
            accounts,
            HashMap::from([(
                1,
                Account {
                    client: 1,
                    available: Amount(123456789),
                    held: Amount(0),
                    total: Amount(123456789),
                    locked: false,
                }
            )])
        );
    }

    #[test]
    fn withdrawal_success() {
        let mut accounts: HashMap<u16, Account> = HashMap::new();
        let deposit_transaction = Transaction {
            tx_type: TxType::Deposit,
            client: 1,
            id: 1,
            amount: Amount::from(12345.67891),
        };
        let withdrawal_transaction = Transaction {
            tx_type: TxType::Withdrawal,
            client: 1,
            id: 1,
            amount: Amount::from(2345.97891),
        };

        apply_transaction(&mut accounts, deposit_transaction);
        apply_transaction(&mut accounts, withdrawal_transaction);

        assert_eq!(
            accounts,
            HashMap::from([(
                1,
                Account {
                    client: 1,
                    available: Amount(99997000),
                    held: Amount(0),
                    total: Amount(99997000),
                    locked: false,
                }
            )])
        );
    }

    #[test]
    fn withdrawal_failure() {
        let mut accounts: HashMap<u16, Account> = HashMap::new();
        let deposit_transaction = Transaction {
            tx_type: TxType::Deposit,
            client: 1,
            id: 1,
            amount: Amount::from(12345.67891),
        };
        let withdrawal_transaction = Transaction {
            tx_type: TxType::Withdrawal,
            client: 1,
            id: 1,
            amount: Amount::from(12345.67901),
        };

        apply_transaction(&mut accounts, deposit_transaction);
        apply_transaction(&mut accounts, withdrawal_transaction);

        assert_eq!(
            accounts,
            HashMap::from([(
                1,
                Account {
                    client: 1,
                    available: Amount(123456789),
                    held: Amount(0),
                    total: Amount(123456789),
                    locked: false,
                }
            )])
        );
    }
}
