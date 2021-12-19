use serde::Serialize;

use crate::transaction::Amount;

#[derive(Clone, Copy, Serialize, PartialEq, Debug)]
pub struct Account {
    pub client: u16,
    pub available: Amount,
    pub held: Amount,
    pub total: Amount,
    pub locked: bool,
}

impl Account {
    pub fn new(client: u16) -> Account {
        Account {
            client,
            available: Amount(0),
            held: Amount(0),
            total: Amount(0),
            locked: false,
        }
    }
}
