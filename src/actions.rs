//!
//! Base logic for filtering rules
//!

use crate::types;
use core::result::Result;
use thiserror::Error;

pub struct ChangeFlags {
    set: types::Flags,
    clear: types::Flags,
}

#[derive(Error, Debug)]
#[error("set and clear flags must not intersect")]
pub struct AmbiguousChangeFlagsError;

/// Make a [`Action::Flags`] using two disjoint set of flags
pub fn make_flags_action(
    context: types::Context,
    set: types::Flags,
    clear: types::Flags,
) -> Result<Action, AmbiguousChangeFlagsError> {
    match set.intersection(&clear).next() {
        Some(_) => Err(AmbiguousChangeFlagsError),
        None => Ok(Action::Flags(ChangeFlags { set, clear })),
    }
}

/// A custom filter that returns one or more other [`Action`] variants.
pub trait LogicAction {
    /// Implement your custom logic here.
    fn process_msg(
        &self,
        msg: &types::Message,
        folder: &types::Folder,
    ) -> Vec<Action>;

    /// Do not override.
    // Helper to forward [types::Context] to the [types::Message]
    fn process(
        &self,
        context: &mut types::Context,
        folder: &types::Folder,
        msg: &types::Message,
    ) -> Vec<Action> {
        self.process_msg(msg, folder)
    }
}

/// A basic filter action that each [`LogicAction`] should return.
pub enum Action {
    /// Custom logic for filtering.
    Logic(Box<dyn LogicAction>),

    /// Move the email to the specified folder.
    Move(types::Folder),

    /// Set or remove flags.
    Flags(ChangeFlags),

    /// Consider this email filtered and do not run filters on it again.
    ///
    /// This is useful if you want to keep the email in the same folder and
    /// avoid re-filtering it when periodically looping over the folder.
    ///
    /// If a [`String`] is provided, it is used as a key to the cache which can
    /// be later invalidated with [`Action::InvalidateCache`].
    ///
    /// The cache applies only if the message is not being moved with
    /// [`Action::Move`]. For moved messages, a filter running on the target
    /// directory will still be called.
    Cache(Option<String>),

    /// Filter emails previously identified by [`Action::Cache`] keyed by
    /// [`String`].
    ///
    /// Unlike ['Action::Cache'], the invalidation happens across all folders.
    InvalidateCache(String),

    /// Stop processing any further filters for this email.
    ///
    /// If you are not moving the email to a different folder, consider also
    /// using [`Action::Cache`] to avoid repeateadly filtering the same email.
    Stop,
}
