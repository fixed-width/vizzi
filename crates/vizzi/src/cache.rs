//! A disposable content-and-context addressed analysis cache.

use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use vizzi_core::model::{Analysis, ContentId, ContextId};

/// Resolves existing symlinks and rejects caches inside the read-only source root.
pub fn external_path(path: &Path, root: &Path) -> std::io::Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    let ancestor = absolute
        .ancestors()
        .find(|path| path.exists())
        .ok_or_else(|| std::io::Error::other("cache has no existing ancestor"))?;
    let mut resolved = ancestor.canonicalize()?;
    for component in absolute
        .strip_prefix(ancestor)
        .map_err(std::io::Error::other)?
        .components()
    {
        match component {
            std::path::Component::ParentDir => {
                resolved.pop();
            }
            std::path::Component::CurDir => {}
            other => resolved.push(other.as_os_str()),
        }
    }
    if resolved.starts_with(root.canonicalize()?) {
        return Err(std::io::Error::other(
            "cache must be outside the analyzed workspace",
        ));
    }
    Ok(resolved)
}

#[derive(Serialize, Deserialize)]
struct Record {
    schema: u32,
    checksum: String,
    payload: Analysis,
}

/// Native JSON persistence; cache data is never authoritative source text.
pub struct Cache {
    directory: PathBuf,
}
impl Cache {
    /// Configures a cache directory without requiring it to be writable.
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }
    fn path(&self, context: &ContextId, content: &ContentId) -> PathBuf {
        self.directory
            .join(format!("{}-{}.json", context.as_str(), content.as_str()))
    }
    /// Loads a checksummed record, treating invalid or absent records as misses.
    pub fn load(&self, context: &ContextId, content: &ContentId) -> Option<Analysis> {
        let bytes = std::fs::read(self.path(context, content)).ok()?;
        let record: Record = serde_json::from_slice(&bytes).ok()?;
        let payload = serde_json::to_vec(&record.payload).ok()?;
        (record.schema == 1
            && &record.payload.context.id == context
            && &record.payload.content == content
            && record.checksum == blake3::hash(&payload).to_hex().as_str())
        .then_some(record.payload)
    }
    /// Writes through a temporary file and atomically replaces the final record.
    pub fn store(
        &self,
        analysis: &Analysis,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::fs::create_dir_all(&self.directory)?;
        let payload = serde_json::to_vec(analysis)?;
        let record = Record {
            schema: 1,
            checksum: blake3::hash(&payload).to_hex().to_string(),
            payload: analysis.clone(),
        };
        let mut file = tempfile::NamedTempFile::new_in(&self.directory)?;
        file.write_all(&serde_json::to_vec(&record)?)?;
        file.persist(self.path(&analysis.context.id, &analysis.content))?;
        Ok(())
    }
    /// Returns the configured directory for diagnostics.
    pub fn directory(&self) -> &Path {
        &self.directory
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vizzi_core::rust::RustAnalyzer;
    #[test]
    fn cache_separation_handles_parent_components_and_existing_symlinks() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("source");
        std::fs::create_dir(&root).unwrap();
        assert!(external_path(&root.join("nested/../cache"), &root).is_err());
        assert_eq!(
            external_path(&temp.path().join("cache/../cache"), &root).unwrap(),
            temp.path().join("cache")
        );
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&root, temp.path().join("alias")).unwrap();
            assert!(external_path(&temp.path().join("alias/cache"), &root).is_err());
        }
        assert!(!root.join("nested").exists());
    }
    #[test]
    fn cache_persists_but_rejects_corruption_and_mismatched_identity() {
        let temp = tempfile::tempdir().unwrap();
        let cache = Cache::new(temp.path().into());
        let analysis = RustAnalyzer::new()
            .unwrap()
            .analyze(b"fn cached() {}")
            .unwrap();
        assert!(
            cache
                .load(&analysis.context.id, &analysis.content)
                .is_none()
        );
        cache.store(&analysis).unwrap();
        assert_eq!(
            Cache::new(temp.path().into()).load(&analysis.context.id, &analysis.content),
            Some(analysis.clone())
        );
        let path = cache.path(&analysis.context.id, &analysis.content);
        let mut record: Record = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        record.payload.facts[0].name = "corrupted".into();
        std::fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
        assert!(
            cache
                .load(&analysis.context.id, &analysis.content)
                .is_none()
        );
        cache.store(&analysis).unwrap();
        let other = ContentId::derive(&[b"changed"]);
        std::fs::copy(&path, cache.path(&analysis.context.id, &other)).unwrap();
        assert!(cache.load(&analysis.context.id, &other).is_none());
        let other_context = ContextId::derive(&[b"new analyzer"]);
        std::fs::copy(&path, cache.path(&other_context, &analysis.content)).unwrap();
        assert!(cache.load(&other_context, &analysis.content).is_none());
        std::fs::write(path, b"not json").unwrap();
        assert!(
            cache
                .load(&analysis.context.id, &analysis.content)
                .is_none()
        );
    }
}
