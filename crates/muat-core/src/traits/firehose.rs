//! Firehose stream trait.

use futures_core::Stream;

use crate::Result;
use crate::repo::RepoEvent;

/// Firehose stream of repository events.
pub trait Firehose: Stream<Item = Result<RepoEvent>> + Send {}

impl<T> Firehose for T where T: Stream<Item = Result<RepoEvent>> + Send {}
