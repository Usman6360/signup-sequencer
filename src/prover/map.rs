use serde::Serialize;
use std::collections::BTreeMap;

use crate::prover::batch_insertion;

use tokio::sync::{RwLock, RwLockReadGuard};

/// The type of a map containing a mapping from a usize to a locked item.
type SharedProverMap<P> = RwLock<ProverMap<P>>;

/// A prover that can have read-only operations performed on it.
pub type ReadOnlyProver<'a, P> = RwLockReadGuard<'a, P>;

/// A map that contains a prover for each batch size.
///
/// Provides utility methods for getting the appropriate provers
///
/// The struct is generic over P for testing purposes.
#[derive(Debug)]
pub struct ProverMap<P> {
    map: BTreeMap<usize, P>,
}

impl<P> ProverMap<P> {
    /// Get the smallest prover that can handle the given batch size.
    pub fn get(&self, batch_size: usize) -> Option<&P> {
        for (size, prover) in &self.map {
            if batch_size <= *size {
                return Some(prover);
            }
        }

        None
    }

    /// Registers the provided `prover` for the given `batch_size` in the map.
    pub fn add(&mut self, batch_size: usize, prover: P) {
        self.map.insert(batch_size, prover);
    }

    /// Removes the prover for the provided `batch_size` from the prover map.
    pub fn remove(&mut self, batch_size: usize) -> Option<P> {
        self.map.remove(&batch_size)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn max_batch_size(&self) -> usize {
        self.map.iter().next_back().map_or(0, |(size, _)| *size)
    }

    pub fn batch_size_exists(&self, batch_size: usize) -> bool {
        self.map.contains_key(&batch_size)
    }
}

impl ProverMap<batch_insertion::Prover> {
    pub fn as_batch_size_vec(&self) -> Vec<BatchSize> {
        self.map
            .iter()
            .map(|(k, v)| BatchSize::new(*k, v.url()))
            .collect()
    }
}

impl<P> From<BTreeMap<usize, P>> for ProverMap<P> {
    fn from(map: BTreeMap<usize, P>) -> Self {
        Self { map }
    }
}

/// A representation of a batch size by configuration.
#[derive(Serialize)]
pub struct BatchSize {
    batch_size: usize,
    prover_url: String,
}

impl BatchSize {
    pub fn new(batch_size: usize, url: impl ToString) -> Self {
        let prover_url = url.to_string();

        Self {
            batch_size,
            prover_url,
        }
    }
}

/// A map of provers for batch insertion operations.
pub type InsertionProverMap = SharedProverMap<batch_insertion::Prover>;

/// The type of provers that can only be read from for insertion operations.
pub type ReadOnlyInsertionProver<'a> = ReadOnlyProver<'a, batch_insertion::Prover>;

/// Builds an insertion prover map from the provided configuration.
pub fn make_insertion_map(
    options: &batch_insertion::Options,
) -> anyhow::Result<InsertionProverMap> {
    let mut map = BTreeMap::new();

    for url in &options.prover_urls.0 {
        map.insert(url.batch_size, batch_insertion::Prover::new(url)?);
    }

    let insertion_map = ProverMap::from(map);

    Ok(RwLock::new(insertion_map))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn prover_map_tests() {
        let prover_map: ProverMap<usize> = ProverMap::from(maplit::btreemap! {
            3 => 3,
            5 => 5,
            7 => 7,
        });

        assert_eq!(prover_map.max_batch_size(), 7);

        assert_eq!(prover_map.get(1), Some(&3));
        assert_eq!(prover_map.get(2), Some(&3));
        assert_eq!(prover_map.get(3), Some(&3));
        assert_eq!(prover_map.get(4), Some(&5));
        assert_eq!(prover_map.get(7), Some(&7));
        assert!(prover_map.get(8).is_none());
    }
}
