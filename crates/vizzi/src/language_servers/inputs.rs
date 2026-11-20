//! Bounded reads and fingerprints for frozen project inputs.
use crate::lsp::Setup;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};

type Prepare<'a> = Box<dyn FnOnce(&Path) -> Result<Setup, String> + 'a>;

pub(super) struct Inputs<'a> {
    digest: String,
    prepare: Prepare<'a>,
}
impl Inputs<'_> {
    pub(super) fn digest(&self) -> &str {
        &self.digest
    }

    pub(super) fn prepare(self, mirror: &Path) -> Result<Setup, String> {
        (self.prepare)(mirror)
    }
}

#[derive(Default)]
pub(super) struct InputFiles {
    raw: BTreeMap<PathBuf, Option<Vec<u8>>>,
}
impl InputFiles {
    pub(super) fn read(&mut self, path: &Path) -> Result<Option<Vec<u8>>, String> {
        if let Some(value) = self.raw.get(path) {
            return Ok(value.clone());
        }
        let metadata = match fs::metadata(path) {
            Ok(value) => value,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.raw.insert(path.into(), None);
                return Ok(None);
            }
            Err(e) => return Err(e.to_string()),
        };
        if !metadata.is_file() {
            return Err("Language-server configuration must be a regular file".into());
        }
        let mut bytes = vec![];
        fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 8 * 1024 * 1024
            || self.raw.values().flatten().map(Vec::len).sum::<usize>() + bytes.len()
                > 16 * 1024 * 1024
            || self.raw.len() >= 64
        {
            return Err("Language-server configuration exceeds allowance".into());
        }
        self.raw.insert(path.into(), Some(bytes.clone()));
        Ok(Some(bytes))
    }

    pub(super) fn freeze<'a>(
        self,
        profile: &str,
        implementation: &[&[u8]],
        prepare: impl FnOnce(&Path) -> Result<Setup, String> + 'a,
    ) -> Inputs<'a> {
        let mut hash = blake3::Hasher::new();
        for implementation in implementation {
            hash.update(implementation);
        }
        hash.update(include_bytes!("inputs.rs"));
        hash.update(profile.as_bytes());
        for (path, bytes) in self.raw {
            let path = path.as_os_str().as_encoded_bytes();
            hash.update(&(path.len() as u64).to_le_bytes());
            hash.update(path);
            hash.update(&[u8::from(bytes.is_some())]);
            if let Some(bytes) = bytes {
                hash.update(&(bytes.len() as u64).to_le_bytes());
                hash.update(&bytes);
            }
        }
        Inputs {
            digest: hash.finalize().to_hex().to_string(),
            prepare: Box::new(prepare),
        }
    }
}

pub(super) fn absolute(base: &Path, path: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path.to_owned()
    } else {
        base.join(path)
    };
    let mut result = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            _ => result.push(component.as_os_str()),
        }
    }
    result
}

pub(super) fn save(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, serde_json::to_vec(value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprints_include_missing_inputs_and_profile_implementation() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("project.config");
        let digest = |implementation: &[u8]| {
            let mut files = InputFiles::default();
            files.read(&path).unwrap();
            files
                .freeze("fixture", &[implementation], |_| Err("no mirror".into()))
                .digest()
                .to_owned()
        };
        let missing = digest(b"implementation");
        fs::write(&path, b"").unwrap();
        let empty = digest(b"implementation");
        assert_ne!(missing, empty);
        fs::write(&path, b"configuration").unwrap();
        let configured = digest(b"implementation");
        assert_ne!(empty, configured);
        assert_ne!(configured, digest(b"new implementation"));
    }

    #[test]
    fn absolute_paths_keep_parent_semantics() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(
            absolute(root.path(), Path::new("src/../include/./value.h")),
            root.path().join("include/value.h")
        );
        assert_eq!(
            absolute(root.path(), root.path().join("lib/../worker.cpp").as_path()),
            root.path().join("worker.cpp")
        );
    }
}
