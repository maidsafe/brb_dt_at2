use core::hash::Hash;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::Money;

// TODO: introduce decomp. of Account from Actor
// pub type Account = Actor; // In the paper, Actor and Account are synonymous

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Transfer<A: Ord + Hash> {
    pub(crate) from: A,
    pub(crate) to: A,
    pub(crate) amount: Money,

    /// set of transactions that need to be applied before this transfer can be validated
    /// ie. a proof of funds
    pub(crate) deps: BTreeSet<Transfer<A>>,
}
