use serde::Serialize;

use sb::Actor;

use super::{Money, Transfer};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub enum Op {
    Transfer(Transfer), // Split out Transfer into it's own struct to get some more type safety in Bank struct
    OpenAccount { owner: Actor, balance: Money },
}
