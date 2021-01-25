//! AT2 Transfer

use core::hash::Hash;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::Money;

// TODO: introduce decomp. of Account from Actor
// pub type Account = Actor; // In the paper, Actor and Account are synonymous

/// An AT2 transfer between two accounts
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Transfer<A: Ord + Hash> {
    pub(crate) from: A,
    pub(crate) to: A,
    pub(crate) amount: Money,

    // PERF: BTreeSet<Transfer> is conceptually simple and elegant, but bloated in
    //       memory and on the wire as each Transfer recursively includes all Transfers
    //       it depends on, and thus grows very quickly, particularly when there are
    //       many incoming transfers in a row. Room for big improvement here.
    /// set of transactions that need to be applied before this transfer can be validated
    /// ie. a proof of funds
    pub(crate) deps: BTreeSet<Transfer<A>>,
}
