use core::{fmt::Debug, hash::Hash};
use std::collections::{BTreeMap, BTreeSet};

use brb::BRBDataType;
use serde::Serialize;

use log::{info, warn};

use super::{Money, Op, Transfer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bank<A: Ord + Hash> {
    id: A,
    // The set of dependencies of the next outgoing transfer
    deps: BTreeSet<Transfer<A>>,

    // The initial balances when opening an actor opened an account
    initial_balances: BTreeMap<A, Money>,

    // Set of all transfers impacting a given actor
    hist: BTreeMap<A, BTreeSet<Transfer<A>>>,
}

impl<A: Ord + Hash + Debug + Clone> Bank<A> {
    pub fn open_account(&self, owner: A, balance: Money) -> Op<A> {
        Op::OpenAccount { owner, balance }
    }

    pub fn initial_balance(&self, actor: &A) -> Money {
        self.initial_balances
            .get(&actor)
            .cloned()
            .unwrap_or_else(|| panic!("[ERROR] No initial balance for {:?}", actor))
    }

    pub fn balance(&self, actor: &A) -> Money {
        // TODO: in the paper, when we read from an actor, we union the actor
        //       history with the deps, I don't see a use for this since anything
        //       in deps is already in the actor history. Think this through a
        //       bit more carefully.
        let h = self.history(actor);

        let outgoing: Money = h
            .iter()
            .filter(|t| &t.from == actor)
            .map(|t| t.amount)
            .sum();
        let incoming: Money = h.iter().filter(|t| &t.to == actor).map(|t| t.amount).sum();

        // We compute differences in a larger space since we need to move to signed numbers
        // and hence we lose a bit.
        let balance_delta: i128 = (incoming as i128) - (outgoing as i128);
        let balance: i128 = self.initial_balance(actor) as i128 + balance_delta;

        assert!(balance >= 0); // sanity check that we haven't violated our balance constraint
        assert!(balance <= Money::max_value() as i128); // sanity check that it's safe to downcast

        balance as Money
    }

    fn history(&self, actor: &A) -> BTreeSet<Transfer<A>> {
        self.hist.get(&actor).cloned().unwrap_or_default()
    }

    pub fn transfer(&self, from: A, to: A, amount: Money) -> Option<Op<A>> {
        let balance = self.balance(&from);
        // TODO: we should leave this validation to the self.validate logic, no need to duplicate it here
        if balance < amount {
            warn!(
                "{:?} does not have enough money to transfer ${} to {:?}. (balance: ${})",
                from, amount, to, balance
            );
            None
        } else {
            let deps = self.deps.clone();
            Some(Op::Transfer(Transfer {
                from,
                to,
                amount,
                deps,
            }))
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ValidationError<A: Ord + Hash> {
    NotInitiatedByAccountOwner {
        source: A,
        owner: A,
    },
    FromAccountDoesNotExist(A),
    ToAccountDoesNotExist(A),
    InsufficientFunds {
        balance: Money,
        transfer_amount: Money,
    },
    MissingDependentOps(BTreeSet<Transfer<A>>),
    OwnerAlreadyHasAnAccount,
}

impl<A: Ord + Hash + Debug + Clone + 'static + Serialize> BRBDataType<A> for Bank<A> {
    type Op = Op<A>;
    type ValidationError = ValidationError<A>;

    fn new(id: A) -> Self {
        Bank {
            id,
            deps: Default::default(),
            initial_balances: Default::default(),
            hist: Default::default(),
        }
    }

    /// Protection against Byzantines
    fn validate(&self, source: &A, op: &Self::Op) -> Result<(), Self::ValidationError> {
        match op {
            Op::Transfer(transfer) => {
                if source != &transfer.from {
                    Err(ValidationError::NotInitiatedByAccountOwner {
                        source: source.clone(),
                        owner: transfer.from.clone(),
                    })
                } else if !self.initial_balances.contains_key(&transfer.from) {
                    Err(ValidationError::FromAccountDoesNotExist(
                        transfer.from.clone(),
                    ))
                } else if !self.initial_balances.contains_key(&transfer.to) {
                    Err(ValidationError::ToAccountDoesNotExist(transfer.to.clone()))
                } else if self.balance(&transfer.from) < transfer.amount {
                    Err(ValidationError::InsufficientFunds {
                        balance: self.balance(&transfer.from),
                        transfer_amount: transfer.amount,
                    })
                } else if !transfer.deps.is_subset(&self.history(&transfer.from)) {
                    Err(ValidationError::MissingDependentOps(
                        transfer
                            .deps
                            .difference(&self.history(&transfer.from))
                            .cloned()
                            .collect(),
                    ))
                } else {
                    Ok(())
                }
            }
            Op::OpenAccount { owner, .. } => {
                if source != owner {
                    Err(ValidationError::NotInitiatedByAccountOwner {
                        source: source.clone(),
                        owner: owner.clone(),
                    })
                } else if self.initial_balances.contains_key(owner) {
                    Err(ValidationError::OwnerAlreadyHasAnAccount)
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Executed once an op has been validated
    fn apply(&mut self, op: Self::Op) {
        match op {
            Op::Transfer(transfer) => {
                // Update the history for the outgoing account
                self.hist
                    .entry(transfer.from.clone())
                    .or_default()
                    .insert(transfer.clone());

                // Update the history for the incoming account
                self.hist
                    .entry(transfer.to.clone())
                    .or_default()
                    .insert(transfer.clone());

                if transfer.to == self.id {
                    self.deps.insert(transfer.clone());
                }

                if transfer.from == self.id {
                    // In the paper, deps are cleared after the broadcast completes in
                    // self.transfer.
                    // Here we break up the initiation of the transfer from the completion.
                    // We move the clearing of the deps here since this is where we now know
                    // the transfer was successfully validated and applied by the network.
                    for prior_transfer in transfer.deps.iter() {
                        // for each dependency listed in the transfer
                        // we remove it from the set of dependencies for a transfer
                        self.deps.remove(prior_transfer);
                    }
                }
            }
            Op::OpenAccount { owner, balance } => {
                info!(
                    "[BANK] opening new account for {:?} with ${}",
                    owner, balance
                );
                self.initial_balances.insert(owner, balance);
            }
        }
    }
}
