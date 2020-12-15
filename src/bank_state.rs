use std::collections::{BTreeSet, HashMap};

use brb::Actor;

use super::{Money, Transfer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankState {
    // When a new account is created, it will be given an initial balance
    pub(crate) initial_balances: HashMap<Actor, Money>,

    // Set of all transfers impacting a given actor
    pub(crate) hist: HashMap<Actor, BTreeSet<Transfer>>, // TODO: Opening an account should be part of history
}
