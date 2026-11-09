//! Local editor adapters consume verified locations, never arbitrary RPC commands.
use std::{
    ffi::OsString,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
/// A local location verified against captured source immediately before handoff.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorLocation {
    /// Absolute path within the configured workspace.
    pub path: PathBuf,
    /// One-based captured line, using column one.
    pub line: usize,
}
/// Refused or failed editor command.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct EditorError(pub String);
/// Native editor seam; validation belongs to the source host, not the editor.
pub trait Editor: Send + Sync {
    /// Requests display of a verified location. Editor buffer confirmation is not implied.
    fn open(&self, location: &EditorLocation) -> Result<(), EditorError>;
    /// User-facing adapter name.
    fn name(&self) -> &str;
}
/// VS Code CLI adapter, with a launch-time executable selected by the operator.
pub struct VsCode {
    executable: PathBuf,
}
impl VsCode {
    /// Configures a trusted executable; HTTP requests cannot change it.
    pub fn new(executable: PathBuf) -> Self {
        Self { executable }
    }
    /// Produces separate literal arguments; source paths are never passed to a shell.
    pub fn arguments(location: &EditorLocation) -> Vec<OsString> {
        let mut target = location.path.as_os_str().to_owned();
        target.push(format!(":{}:1", location.line));
        vec!["--reuse-window".into(), "--goto".into(), target]
    }
}
impl Editor for VsCode {
    fn name(&self) -> &str {
        "VS Code"
    }
    fn open(&self, location: &EditorLocation) -> Result<(), EditorError> {
        let mut child = Command::new(&self.executable)
            .args(Self::arguments(location))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| EditorError(format!("Could not launch VS Code: {e}")))?;
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    return if status.success() {
                        Ok(())
                    } else {
                        Err(EditorError("VS Code refused the handoff. Check the configured executable and editor session.".into()))
                    };
                }
                Ok(None) => {}
                Err(e) => return Err(EditorError(e.to_string())),
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(EditorError(
                    "VS Code did not acknowledge the handoff within 10 seconds".into(),
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
