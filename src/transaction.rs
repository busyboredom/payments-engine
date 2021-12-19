use std::ops::{Add, AddAssign, Div, Rem, SubAssign};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub tx_type: TxType,
    pub client: u16,
    #[serde(rename = "tx")]
    pub id: u32,
    pub amount: Option<Amount>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
pub enum TxType {
    #[serde(rename = "deposit")]
    Deposit,
    #[serde(rename = "withdrawal")]
    Withdrawal,
    #[serde(rename = "dispute")]
    Dispute,
    #[serde(rename = "resolve")]
    Resolve,
    #[serde(rename = "chargeback")]
    Chargeback,
}

// Amounts in the input file are fixed-precision (4 decimal places), so using a float can cause
// inaccuracies in edge cases. We will use a custom fixed-precision datatype instead.
#[derive(PartialEq, Debug, Clone, Copy, Deserialize, Serialize, Default, Eq, PartialOrd, Ord)]
#[serde(from = "f64", into = "f64")]
pub struct Amount(pub u64);

impl Amount {
    pub fn checked_sub(self, rhs: Amount) -> Option<Amount> {
        self.0.checked_sub(rhs.0).map(Amount)
    }

    pub fn saturating_sub(self, rhs: Amount) -> Amount {
        Amount(self.0.saturating_sub(rhs.0))
    }
}

// Convert from float to fixed precision Amount. Rounds float down to 4 decimal places.
impl From<f64> for Amount {
    fn from(float: f64) -> Amount {
        if float < 0.0 || float > u64::MAX as f64 / 10_000.0 {
            panic!("cannot represent transaction amount with fixed precision of 4 decimal places")
        }
        Amount((float * 10_000.0).floor() as u64)
    }
}

// Convert from fixed precision amount to f64.
impl From<Amount> for f64 {
    fn from(amount: Amount) -> f64 {
        (amount.0 / 10_000) as f64 + (amount.0 % 10_000) as f64 / 10_000.0
    }
}

// Allow modulo operator between Amount and u64.
impl Rem<u64> for Amount {
    type Output = Amount;

    fn rem(self, modulus: u64) -> Self {
        self.0
            .checked_rem(modulus)
            .map(Amount)
            .expect("Amount remainder error")
    }
}

// Allow division operator between Amount and u64.
impl Div<u64> for Amount {
    type Output = Amount;

    fn div(self, rhs: u64) -> Self {
        self.0
            .checked_div(rhs)
            .map(Amount)
            .expect("Amount division error")
    }
}

// Allow addition between Amount and Amount.
impl Add<Amount> for Amount {
    type Output = Amount;

    fn add(self, rhs: Amount) -> Self {
        Amount(self.0 + rhs.0)
    }
}

// Allow += operation between Amount and Amount.
impl AddAssign for Amount {
    fn add_assign(&mut self, other: Self) {
        *self = Self(self.0 + other.0);
    }
}

// Allow -= operation between Amount and Amount.
impl SubAssign for Amount {
    fn sub_assign(&mut self, rhs: Self) {
        *self = Self(self.0 - rhs.0);
    }
}

#[cfg(test)]
mod test {
    use crate::transaction::Amount;

    #[test]
    fn amount_from_float() {
        assert_eq!(Amount::from(123_456.78912345), Amount(1_234_567_891));
    }

    #[test]
    fn float_from_amount() {
        let amount = Amount(1_234_567_891);
        assert_eq!(f64::from(amount), 123_456.7891)
    }
}
