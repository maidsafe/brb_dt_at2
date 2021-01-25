// Copyright 2021 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

//! This library contains:
//!
//! 1. An implementation of AT2: Asynchronous Trustworthy Transfers
//! 2. A BRBDataType wrapper around AT2
//!
//! The wrapper enables AT2 operations to be transmitted in a BFT manner using
//! Byzantine Reliable Broadcast.
//!
//! AT2 is described formally in:
//! https://arxiv.org/pdf/1812.10844.pdf
//!
//! BRB is a modified form of the Deterministic Secure Broadcast defined in the
//! above paper.
//!
//! This AT2 implementation has been coded for simplicity and clarity.
//! It has not been optimized for a large number of transfers and is presently
//! best-suited for testing out the concept.  It will certainly suffer
//! performance bottlenecks if run with any significant number of transfers.
//! Some of these bottlenecks are commented in the code.

#![deny(missing_docs)]

pub mod money;
pub use money::Money;

pub mod bank;
pub use bank::Bank;

pub mod op;
pub use op::Op;

pub mod transfer;
pub use transfer::Transfer;
