//! JDT LS over explicit source roots in a captured project.
use super::{
    inputs::{InputFiles, Inputs, PreparedInputs},
    project::Profile,
};
use crate::lsp::{DocumentSync, Readiness, Setup};
use serde_json::json;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
use vizzi_core::model::Snapshot;

pub(super) struct JdtLs {
    pub roots: BTreeSet<String>,
}

impl Profile for JdtLs {
    fn languages(&self) -> &'static [&'static str] {
        &["java"]
    }

    fn includes_path(&self, path: &str) -> bool {
        let parent = Path::new(path).parent();
        self.roots
            .iter()
            .any(|root| root == "." || parent.is_some_and(|parent| parent.starts_with(root)))
    }

    fn inputs<'a>(&'a self, _root: &'a Path, snapshot: &'a Snapshot) -> Result<Inputs<'a>, String> {
        let mut captured = blake3::Hasher::new();
        captured.update(
            serde_json::to_string(&self.roots)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        );
        // Rebuild the project index after edits, including changes to unopened files.
        for doc in snapshot
            .documents()
            .iter()
            .filter(|doc| self.includes_document(doc))
        {
            captured.update(&(doc.path().len() as u64).to_le_bytes());
            captured.update(doc.path().as_bytes());
            captured.update(doc.content().as_str().as_bytes());
        }
        Ok(InputFiles::default().freeze(
            &format!("jdtls:{}", captured.finalize().to_hex()),
            &[include_bytes!("java.rs")],
            |mirror| prepare(mirror, &self.roots),
        ))
    }
}

pub(super) fn source_roots(roots: &[String]) -> Result<BTreeSet<String>, String> {
    if roots.is_empty() {
        return Ok(BTreeSet::from([".".into()]));
    }
    roots
        .iter()
        .map(|root| {
            if root.is_empty()
                || root.starts_with('/')
                || root.contains(['\\', ':', '\0'])
                || root.split('/').any(|part| part == "..")
            {
                return Err("--java-source-root requires a workspace-relative path".into());
            }
            let path = root
                .split('/')
                .filter(|part| !matches!(*part, "" | "."))
                .collect::<Vec<_>>()
                .join("/");
            Ok(if path.is_empty() { ".".into() } else { path })
        })
        .collect()
}

fn prepare(mirror: &Path, roots: &BTreeSet<String>) -> Result<PreparedInputs, String> {
    let private_state = tempfile::Builder::new()
        .prefix("jdtls-")
        .tempdir_in(
            mirror
                .parent()
                .ok_or("Java semantic mirror has no parent")?,
        )
        .map_err(|e| e.to_string())?;
    let argument = |name| -> Result<String, String> {
        let path: PathBuf = private_state.path().join(name);
        path.into_os_string()
            .into_string()
            .map_err(|_| "JDT LS state path is not UTF-8".into())
    };
    let settings = json!({
        "java": {
            "autobuild": {"enabled": false},
            "configuration": {"updateBuildConfiguration": "disabled"},
            "import": {
                "eclipse": {"enabled": false},
                "gradle": {"enabled": false},
                "maven": {"enabled": false}
            },
            "project": {"sourcePaths": roots},
            "telemetry": {"enabled": false}
        }
    });
    let setup = Setup {
        arguments: vec![
            "-configuration".into(),
            argument("configuration")?,
            "-data".into(),
            argument("workspace")?,
        ],
        initialization: json!({
            "settings": settings,
            "extendedClientCapabilities": {"classFileContentsSupport": true}
        }),
        configuration: settings,
        readiness: Readiness::JdtLs,
        document_sync: DocumentSync::Ordered,
        environment: ["CLIENT_PORT", "CLIENT_HOST", "CLIENT_PIPE"]
            .into_iter()
            .map(|name| (name.into(), None))
            .collect(),
    };
    Ok(PreparedInputs {
        private_state: Some(private_state),
        ..setup.into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_roots_default_to_the_workspace_and_normalize_explicit_paths() {
        assert_eq!(source_roots(&[]).unwrap(), BTreeSet::from([".".into()]));
        let roots = ["src//main/java", "./src/test/java/", "src/main/java"].map(String::from);
        assert_eq!(
            source_roots(&roots).unwrap(),
            BTreeSet::from(["src/main/java".into(), "src/test/java".into()])
        );
    }

    #[test]
    fn source_roots_reject_absolute_paths_and_parent_traversal() {
        for path in [
            "",
            "/src",
            "src/../outside",
            "C:/src",
            "src\\test",
            "src\0test",
        ] {
            assert!(source_roots(&[path.into()]).is_err(), "{path:?}");
        }
    }
}
