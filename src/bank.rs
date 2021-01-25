// Copyright 2021 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

//! The Bank represents current AT2 state for a given
//! `Actor` (account), plus all-time transaction history for all
//! actors.
//!
//! It can be thought of as a distributed ledger of accounts
//! where each Bank instance sees and records all accounts and
//! their transfers, but represents a particular account owner
//! and can initiate outgoing transfers for that one account only.
//!
//! A note on terminology:
//! `Actor` and Account are the same thing. Each `Transfer` is
//! associated with an `Actor`.  There is no Account data structure.

use core::{fmt::Debug, hash::Hash};
use std::collections::{BTreeMap, BTreeSet};

use brb::BRBDataType;
use serde::Serialize;

use log::{info, warn};

use thiserror::Error;

use super::{Money, Op, Transfer};

/// AT2 `Bank` for a particular `Actor`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bank<A: Ord + Hash> {
    /// Actor associated with this Bank instance
    id: A,

    /// The set of dependencies of the next outgoing transfer.
    /// Note that we can only initiate an outgoing transfer
    /// for the account identified by Bank::id
    deps: BTreeSet<Transfer<A>>,

    // PERF: Transfer, used in deps and hist, is recursive and grows too quickly.
    /// The initial balances when an actor opened an account
    /// Normally 0, but this enables an application to force
    /// a non-zero starting balance.  Though of course other
    /// nodes must agree.
    initial_balances: BTreeMap<A, Money>,

    /// Set of all transfers, by actor
    hist: BTreeMap<A, BTreeSet<Transfer<A>>>,
}

impl<A: Ord + Hash + Debug + Clone> Bank<A> {
    /// Open a new account.
    ///
    /// The balance field should normally be 0, but this field
    /// enables an application to force a non-zero starting balance.
    /// Though of course other nodes must agree.  This could for
    /// example be used to pre-fund a "MINT" account that spends
    /// money into existence (in other accounts) over time.
    pub fn open_account(&self, owner: A, balance: Money) -> Op<A> {
        Op::OpenAccount { owner, balance }
    }

    /// Returns an account's starting balance, prior to any transfers in or out.
    pub fn initial_balance(&self, actor: &A) -> Money {
        self.initial_balances
            .get(&actor)
            .cloned()
            .unwrap_or_else(|| panic!("[ERROR] No initial balance for {:?}", actor))
    }

    /// Returns an account's present balance.
    ///
    /// This is presently a slow operation as the entire history of all
    /// transfers is iterated.  i.e., it degrades O(n) with the size of the history.
    pub fn balance(&self, actor: &A) -> Money {
        // PERF: Can we make this function faster?  perhaps even O(1)?

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

    /// Returns complete history of transfers for provided actor
    fn history(&self, actor: &A) -> BTreeSet<Transfer<A>> {
        // PERF: can we make this faster, without need to clone?
        self.hist.get(&actor).cloned().unwrap_or_default()
    }

    /// Generates a new Transfer operation (but does not apply it)
    pub fn transfer(&self, from: A, to: A, amount: Money) -> Option<Op<A>> {
        // PERF: balance() is presently an expensive call.
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

/// Enumeration of AT2 validation errors
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ValidationError {
    /// The actor that initiated the operation does not match the account owner
    #[error("The actor that initiated the operation does not match the account owner")]
    NotInitiatedByAccountOwner,

    /// The From account does not exist
    #[error("The From account does not exist")]
    FromAccountDoesNotExist,

    /// The To account does not exist
    #[error("The To account does not exist")]
    ToAccountDoesNotExist,

    /// Insufficient funds
    #[error("Insufficient funds")]
    InsufficientFunds {
        /// Account balance
        balance: Money,
        /// Transfer amount
        transfer_amount: Money,
    },

    /// Missing dependent ops
    #[error("Missing dependent ops")]
    MissingDependentOps,

    /// Owner already has an account
    #[error("Owner already has an account")]
    OwnerAlreadyHasAnAccount,
}

impl<A: Ord + Hash + Debug + Clone + 'static + Serialize> BRBDataType<A> for Bank<A> {
    type Op = Op<A>;
    type ValidationError = ValidationError;

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
                    Err(ValidationError::NotInitiatedByAccountOwner)
                } else if !self.initial_balances.contains_key(&transfer.from) {
                    Err(ValidationError::FromAccountDoesNotExist)
                } else if !self.initial_balances.contains_key(&transfer.to) {
                    Err(ValidationError::ToAccountDoesNotExist)
                } else if self.balance(&transfer.from) < transfer.amount {
                    Err(ValidationError::InsufficientFunds {
                        balance: self.balance(&transfer.from),
                        transfer_amount: transfer.amount,
                    })
                } else if !transfer.deps.is_subset(&self.history(&transfer.from)) {
                    Err(ValidationError::MissingDependentOps)
                } else {
                    Ok(())
                }
            }
            Op::OpenAccount { owner, .. } => {
                if source != owner {
                    Err(ValidationError::NotInitiatedByAccountOwner)
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

                // Add this transfer to self.deps only if we are recipient.
                if transfer.to == self.id {
                    self.deps.insert(transfer.clone());
                }

                // remove transfer.deps from self.deps only if we are sender.
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
