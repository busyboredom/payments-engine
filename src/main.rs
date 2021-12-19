#![warn(clippy::pedantic)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]

mod account;
mod transaction;

use std::cmp::min;
use std::fs::File;
use std::io;
use std::path::Path;
use std::{collections::HashMap, env};

use csv::Reader;

use account::Account;
use transaction::{Amount, Transaction, TxType};

fn main() {
    // Get file path from argument.
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        panic!("expected one argument, but received {}", args.len() - 1);
    }
    let transactions_path = &args[1];

    // Determine account balances from transactions.
    let accounts = process_transactions(transactions_path);

    // Write account details to standard output.
    let mut wtr = csv::Writer::from_writer(io::stdout());
    for account in accounts.values() {
        wtr.serialize(account)
            .expect("failed to write account details to stdout");
    }
    wtr.flush().expect("failed to flush output to stdout");
}

/// Reads csv from provided path, and returns account balances resulting from the described
/// transactions.
fn process_transactions<P: AsRef<Path>>(path: P) -> HashMap<u16, Account> {
    // Prepare csv reader.
    let mut transactions_reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(path)
        .expect("failed to read csv");

    let mut accounts: HashMap<u16, Account> = HashMap::new();
    let mut raw_record = csv::ByteRecord::new();
    let headers = transactions_reader
        .byte_headers()
        .expect("failed to read headers from csv")
        .clone();
    let mut disputed_transactions: HashMap<u32, Transaction> = HashMap::new();

    // Read csv line by line, updating account balances as we go.
    while transactions_reader
        .read_byte_record(&mut raw_record)
        .expect("failed to read row of csv")
    {
        let transaction: Transaction = raw_record
            .deserialize(Some(&headers))
            .expect("failed to deserialize transaction from csv");

        match transaction.tx_type {
            TxType::Deposit => deposit(&mut accounts, &transaction),
            TxType::Withdrawal => withdrawal(&mut accounts, &transaction),
            TxType::Dispute => dispute(
                &mut accounts,
                &transaction,
                &mut disputed_transactions,
                &mut transactions_reader,
            ),
            TxType::Resolve => resolve(&mut accounts, &transaction, &mut disputed_transactions),
            TxType::Chargeback => {
                chargeback(&mut accounts, &transaction, &mut disputed_transactions);
            }
        }
    }

    accounts
}

/// Adds specified amount to available account balance.
fn deposit(accounts: &mut HashMap<u16, Account>, transaction: &Transaction) {
    let account = accounts
        .entry(transaction.client)
        .or_insert_with(|| Account::new(transaction.client));

    account.available += transaction.amount.unwrap_or_default();
    account.total += transaction.amount.unwrap_or_default();
}

/// Reduces available account balance by specified amount.
fn withdrawal(accounts: &mut HashMap<u16, Account>, transaction: &Transaction) {
    let account = accounts
        .entry(transaction.client)
        .or_insert_with(|| Account::new(transaction.client));

    // If an overflow occurs (account balance is insufficient), we ignore the withdrawal.
    if let Some(available) = account
        .available
        .checked_sub(transaction.amount.unwrap_or_default())
    {
        account.available = available;
        account.total -= transaction.amount.unwrap_or_default();
    };
}

/// Disputes specified transaction, if it exists.
fn dispute(
    accounts: &mut HashMap<u16, Account>,
    transaction: &Transaction,
    disputed_transactions: &mut HashMap<u32, Transaction>,
    transactions_reader: &mut Reader<File>,
) {
    // Seek back start so we can search for the disputed tx.
    let position = transactions_reader.position().clone();
    transactions_reader
        .seek(csv::Position::new())
        .expect("failed to seek to beginning of csv");

    // Read past header row.
    let mut raw_record = csv::ByteRecord::new();
    transactions_reader
        .read_byte_record(&mut raw_record)
        .expect("failed to read csv headers");

    // If the disputed transaction doesn't exist, we do nothing.
    if let Some(disputed_tx) = transactions_reader
        .deserialize()
        .map(|tx_or_err| tx_or_err.expect("failed to parse csv row as Transaction"))
        .find(|tx: &Transaction| tx.id == transaction.id)
    {
        let account = accounts
            .entry(transaction.client)
            .or_insert_with(|| Account::new(transaction.client));

        // Don't allow disputing someone else's transaction.
        if transaction.client != disputed_tx.client {
            // Ideally we would log an error here.
            return;
        }

        // Only allow disputing Deposits.
        if disputed_tx.tx_type != TxType::Deposit {
            // Ideally we would log an error here.
            return;
        }

        // If the disputed amount is more than the available balance, the best we can do is hold the
        // available balance. Ideally in the real world, this should rarely happen because withdrawals
        // should be disallowed for a suitable holding period.
        account.held += min(account.available, disputed_tx.amount.unwrap_or_default());
        account.available = account
            .available
            .saturating_sub(disputed_tx.amount.unwrap_or_default());

        disputed_transactions.insert(disputed_tx.id, disputed_tx);
    }

    // Return to current row.
    transactions_reader
        .seek(position)
        .expect("failed to seek back to current row of csv");
}

/// Resolves disputed transaction, if it exists.
fn resolve(
    accounts: &mut HashMap<u16, Account>,
    transaction: &Transaction,
    disputed_transactions: &mut HashMap<u32, Transaction>,
) {
    // If transaction is not disputed, do nothing.
    if disputed_transactions.remove(&transaction.id).is_some() {
        let account = accounts
            .entry(transaction.client)
            .or_insert_with(|| Account::new(transaction.client));

        // Get the total remaining amount disputed for the given account.
        let amount_disputed = disputed_transactions.values().fold(Amount(0), |acc, tx| {
            if tx.client == transaction.client {
                acc + tx.amount.unwrap_or_default()
            } else {
                Amount(0)
            }
        });

        // Set held equal to the amount disputed, unless total balance is smaller.
        account.held = min(amount_disputed, account.total);
        account.available = account.total.saturating_sub(account.held);
    }
}

/// Charges back disputed transaction, if it exists.
fn chargeback(
    accounts: &mut HashMap<u16, Account>,
    transaction: &Transaction,
    disputed_transactions: &mut HashMap<u32, Transaction>,
) {
    // If transaction is not disputed, do nothing.
    if let Some(disputed_tx) = disputed_transactions.remove(&transaction.id) {
        let account = accounts
            .entry(transaction.client)
            .or_insert_with(|| Account::new(transaction.client));

        // Reduce amount held by amount charged back.
        account.held = account
            .held
            .saturating_sub(disputed_tx.amount.unwrap_or_default());

        // Recalculate total.
        account.total = account.held + account.available;

        // Lock account.
        account.locked = true;
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, path::Path};

    use crate::transaction::{Amount, Transaction, TxType};
    use crate::Account;
    use crate::{deposit, process_transactions, withdrawal};

    #[test]
    fn deposit_success() {
        let mut accounts: HashMap<u16, Account> = HashMap::new();
        let transaction = Transaction {
            tx_type: TxType::Deposit,
            client: 1,
            id: 1,
            amount: Some(Amount::from(12345.67891)),
        };

        deposit(&mut accounts, &transaction);

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
            amount: Some(Amount::from(12345.67891)),
        };
        let withdrawal_transaction = Transaction {
            tx_type: TxType::Withdrawal,
            client: 1,
            id: 2,
            amount: Some(Amount::from(2345.97891)),
        };

        deposit(&mut accounts, &deposit_transaction);
        withdrawal(&mut accounts, &withdrawal_transaction);

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
            amount: Some(Amount::from(12345.67891)),
        };
        let withdrawal_transaction = Transaction {
            tx_type: TxType::Withdrawal,
            client: 1,
            id: 2,
            amount: Some(Amount::from(12345.67901)),
        };

        deposit(&mut accounts, &deposit_transaction);
        withdrawal(&mut accounts, &withdrawal_transaction);

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
    fn dispute_available() {
        let accounts = process_transactions(Path::new("test/data/dispute_available.csv"));

        assert_eq!(
            accounts,
            HashMap::from([(
                1,
                Account {
                    client: 1,
                    available: Amount(0),
                    held: Amount(123456789),
                    total: Amount(123456789),
                    locked: false,
                }
            )])
        );
    }

    #[test]
    fn dispute_unavailable() {
        let accounts = process_transactions(Path::new("test/data/dispute_unavailable.csv"));

        assert_eq!(
            accounts,
            HashMap::from([(
                1,
                Account {
                    client: 1,
                    available: Amount(0),
                    held: Amount(99997000),
                    total: Amount(99997000),
                    locked: false,
                }
            )])
        );
    }

    #[test]
    fn resolve_available() {
        let accounts = process_transactions(Path::new("test/data/resolve_available.csv"));

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
    fn resolve_unavailable() {
        let accounts = process_transactions(Path::new("test/data/resolve_unavailable.csv"));

        assert_eq!(
            accounts,
            HashMap::from([(
                1,
                Account {
                    client: 1,
                    available: Amount(123446789),
                    held: Amount(0),
                    total: Amount(123446789),
                    locked: false,
                }
            )])
        );
    }

    #[test]
    fn chargeback_available() {
        let accounts = process_transactions(Path::new("test/data/chargeback_available.csv"));

        assert_eq!(
            accounts,
            HashMap::from([(
                1,
                Account {
                    client: 1,
                    available: Amount(10000),
                    held: Amount(0),
                    total: Amount(10000),
                    locked: true,
                }
            )])
        );
    }

    #[test]
    fn chargeback_unavailable() {
        let accounts = process_transactions(Path::new("test/data/chargeback_unavailable.csv"));

        assert_eq!(
            accounts,
            HashMap::from([(
                1,
                Account {
                    client: 1,
                    available: Amount(0),
                    held: Amount(0),
                    total: Amount(0),
                    locked: true,
                }
            )])
        );
    }
}
