//! Validated capture plans and immutable updates over trusted prior observations.

use crate::{
    analysis::Languages,
    model::{CapturePlanId, Context, Document, ModelError, Snapshot, WorkspaceId},
    policy::{PolicyConfig, SourcePolicy},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// Frozen selection and full analyzer contexts used to validate captured observations.
pub struct CapturePlan {
    id: CapturePlanId,
    policy: SourcePolicy,
    contexts: BTreeMap<String, Context>,
}
impl CapturePlan {
    /// Freezes source policy and analyzer configuration independently of workspace paths.
    pub fn new(config: PolicyConfig, languages: &Languages) -> Result<Self, ModelError> {
        let policy = SourcePolicy::new(config, languages.names().map(String::from))?;
        let contexts = languages
            .names()
            .map(|name| {
                languages
                    .select(name)
                    .map(|language| (name.to_owned(), language.context().clone()))
                    .ok_or_else(|| ModelError("registered language disappeared".into()))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let encoded =
            serde_json::to_vec(&contexts).map_err(|error| ModelError(error.to_string()))?;
        let id = CapturePlanId::derive(&[
            b"capture-plan-v1",
            include_bytes!("capture.rs"),
            policy.id().as_str().as_bytes(),
            &encoded,
        ]);
        Ok(Self {
            id,
            policy,
            contexts,
        })
    }
    /// Returns selection/configuration compatibility identity.
    pub fn id(&self) -> &CapturePlanId {
        &self.id
    }
    /// Returns the selection policy associated with these contexts.
    pub fn policy(&self) -> &SourcePolicy {
        &self.policy
    }

    /// Validates all documents and records the plan in an immutable observation.
    pub fn snapshot(
        &self,
        workspace: WorkspaceId,
        documents: Vec<Arc<Document>>,
    ) -> Result<Snapshot, ModelError> {
        for document in &documents {
            self.validate(document)?;
        }
        Snapshot::verified(
            workspace,
            documents,
            self.policy.id().clone(),
            self.id.clone(),
        )
    }

    /// Validates changed documents and merges them with an already verified baseline.
    pub fn update(
        &self,
        previous: &Snapshot,
        mut changed: Vec<Arc<Document>>,
        removed: &[String],
    ) -> Result<Snapshot, ModelError> {
        if previous.capture_plan() != Some(&self.id) {
            return Err(ModelError(
                "snapshot is not verified for this capture plan".into(),
            ));
        }
        let mut deletions = BTreeSet::new();
        for path in removed {
            self.policy.excludes(path)?;
            deletions.insert(path.as_str());
        }
        changed.sort_by(|a, b| a.path().cmp(b.path()));
        if changed
            .windows(2)
            .any(|pair| pair[0].path() == pair[1].path())
        {
            return Err(ModelError("duplicate changed path".into()));
        }
        for document in &changed {
            self.validate(document)?;
            if deletions.contains(document.path()) {
                return Err(ModelError("changed path is also removed".into()));
            }
        }
        let mut documents = Vec::with_capacity(previous.documents().len() + changed.len());
        let mut changes = changed.into_iter().peekable();
        for old in previous.documents() {
            while changes.peek().is_some_and(|new| new.path() < old.path()) {
                if let Some(new) = changes.next() {
                    documents.push(new);
                }
            }
            if changes.peek().is_some_and(|new| new.path() == old.path()) {
                if let Some(new) = changes.next() {
                    documents.push(new);
                }
            } else if !deletions.contains(old.path()) {
                documents.push(old.clone());
            }
        }
        documents.extend(changes);
        Snapshot::verified(
            previous.workspace().clone(),
            documents,
            self.policy.id().clone(),
            self.id.clone(),
        )
    }

    fn validate(&self, document: &Document) -> Result<(), ModelError> {
        let classification = self
            .policy
            .classify(document.path())?
            .ok_or_else(|| ModelError("document is outside capture scope".into()))?;
        if &classification != document.classification() {
            return Err(ModelError(
                "document classification differs from capture plan".into(),
            ));
        }
        let valid = match classification.assigned_language() {
            Some(language) => {
                document.analyses().len() == 1
                    && self.contexts.get(language) == Some(&document.analyses()[0].context)
            }
            None => document.analyses().is_empty(),
        };
        if !valid {
            return Err(ModelError(
                "document analysis differs from capture plan".into(),
            ));
        }
        Ok(())
    }
}
