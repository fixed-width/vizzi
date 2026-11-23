//! Exact local Git selection for one repository declaration.
use super::*;

/// Opens an exact commit from either an observed or an owned declaration.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectRevision {
    revision: String,
    origin: Origin,
}

/// Confirms that the browser applied a prepared revision.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmRevision {
    token: String,
}

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    content = "source",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum Origin {
    Current(Request),
    Pinned(Request),
    Captured(FrozenSource),
}

impl FunctionHistory {
    /// Prepares history at the exact resolved commit without expiring retained sessions.
    pub fn select_revision(&self, request: &SelectRevision) -> Result<Value, CompanionError> {
        let revision = request
            .revision
            .trim_matches(|c: char| c.is_ascii_whitespace());
        if revision.is_empty()
            || revision.len() > 256
            || revision.starts_with('-')
            || revision.contains(['~', '^', ':', '?', '*', '[', '\\'])
            || revision.contains("..")
            || revision.contains("@{")
            || revision
                .bytes()
                .any(|b| b.is_ascii_whitespace() || b.is_ascii_control())
        {
            return Err(invalid("Enter a local Git commit ID, branch or tag"));
        }

        let (doc, tracking) = match &request.origin {
            Origin::Current(source) => (
                self.observed_document(source, false)?,
                source.tracking.clone(),
            ),
            Origin::Pinned(source) => (
                self.observed_document(source, true)?,
                source.tracking.clone(),
            ),
            Origin::Captured(source) => self.with_captured_file(source, |capture, _| {
                let index = capture.structure();
                let tracking = index
                    .resolve(None, &source.symbol)
                    .filter(|symbol| symbol.kind == "function")
                    .map(|symbol| symbol.tracking.clone())
                    .ok_or_else(|| {
                        invalid("Revision selection needs an unambiguous captured function")
                    })?;
                let doc = capture
                    .snapshot
                    .file(&source.file)
                    .cloned()
                    .ok_or_else(|| conflict("Captured source file differs"))?;
                Ok((doc, tracking))
            })?,
        };
        let mut sessions = self.lock()?;
        let mut session = self.session(&doc, &tracking, Some(revision))?;
        let commit = session.tip.clone();
        let (declaration, digest) = self.declaration(&session, &commit)?;
        let same_text = digest == session.digest;
        self.accept(&mut session, commit, declaration, digest)?;
        if let Origin::Current(source) = &request.origin {
            self.verify_current(&source.snapshot)?;
        }
        let mut answer = frame(&session)?;
        answer["same_text"] = json!(same_text);
        sessions.prepare(session);
        Ok(answer)
    }

    /// Admits a prepared selection to ordinary history retention. Repeated confirmations are safe.
    pub fn confirm_revision(&self, request: &ConfirmRevision) -> Result<(), CompanionError> {
        let mut sessions = self.lock()?;
        if let Some(session) = take_session(&mut sessions.prepared, &request.token) {
            admit(&mut sessions.retained, session);
        } else if sessions.get(&request.token).is_none() {
            return Err(conflict("Prepared revision expired; choose it again"));
        }
        Ok(())
    }
}
