//! Source-bound permutations for optional search ranking adapters.
use super::{Context, Corpus, Error, Evidence, require};
use std::collections::BTreeSet;

impl Corpus {
    /// Resolves a complete permutation without changing labels, excerpts or source metadata.
    ///
    /// Rejects stale contexts and duplicate, missing or unavailable labels. An empty current
    /// context accepts only an empty order. Relevance is not checked.
    pub fn reorder<'a>(
        &self,
        context: &'a Context,
        order: &[usize],
    ) -> Result<Vec<&'a Evidence>, Error> {
        require(
            context.snapshot == self.snapshot,
            "Context belongs to a different source snapshot",
        )?;
        require(
            order.len() == context.evidence.len(),
            "Order must include every candidate exactly once",
        )?;
        let mut seen = BTreeSet::new();
        order
            .iter()
            .map(|&label| {
                require(seen.insert(label), "Duplicate ranking label")?;
                label
                    .checked_sub(1)
                    .and_then(|n| context.evidence.get(n))
                    .ok_or_else(|| Error("Ranking label is unavailable".into()))
            })
            .collect()
    }
}
