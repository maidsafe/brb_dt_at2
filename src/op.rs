use core::hash::Hash;

use serde::{Deserialize, Serialize};

use super::{Money, Transfer};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Op<A: Ord + Hash> {
    Transfer(Transfer<A>), // Split out Transfer into it's own struct to get some more type safety in Bank struct
    OpenAccount { owner: A, balance: Money },
}
