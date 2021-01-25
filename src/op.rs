//! AT2 Op

use core::hash::Hash;

use serde::{Deserialize, Serialize};

use super::{Money, Transfer};

/// An AT2 operation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Op<A: Ord + Hash> {
    /// Transfer money between 2 accounts
    Transfer(Transfer<A>), // Split out Transfer into it's own struct to get some more type safety in Bank struct
    /// Open a new account
    OpenAccount {
        /// Account owner
        owner: A,
        /// Account initial balance.  typically 0.
        balance: Money,
    },
}
