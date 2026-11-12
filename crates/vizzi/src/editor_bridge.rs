//! Editor-owned analysis over a bounded, authenticated local connection.
use crate::definitions::{self, Answer, Location, Position, Provider};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use vizzi_core::{
    model::{ContextId, Document, Snapshot},
    structure::{CallLookup, Symbol},
};

const FRAME: usize = 16 * 1024 * 1024;
const FILE_BYTES: usize = 4 * 1024 * 1024;
const WORKSPACE_BYTES: usize = 128 * 1024 * 1024;
const RESULTS: usize = 256;

#[derive(Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Connection {
    version: u32,
    root: PathBuf,
    port: u16,
    token: String,
}
#[derive(Serialize)]
struct Request<'a> {
    version: u32,
    token: &'a str,
    root: &'a Path,
    method: &'a str,
    path: &'a str,
    text: &'a str,
    position: Position,
}
#[derive(Deserialize)]
struct Response {
    version: u32,
    #[serde(flatten)]
    outcome: Outcome,
}
#[derive(Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Outcome {
    Ok {
        session: String,
        epoch: u64,
        locations: Vec<Target>,
        outside: usize,
    },
    Error {
        message: String,
    },
}
#[derive(Deserialize)]
struct Target {
    path: PathBuf,
    position: Position,
    text: String,
}

pub(crate) struct Bridge {
    root: PathBuf,
    connection: PathBuf,
    active: Mutex<()>,
}
fn read_file(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Editor connection/source must be a regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("Editor connection/source exceeds allowance".into());
    }
    Ok(bytes)
}
fn source(doc: &Document) -> Result<String, String> {
    doc.captured()
        .read_text(FILE_BYTES)
        .map_err(|e| e.to_string())?
        .map(|text| text.into_owned())
        .ok_or_else(|| "Editor source is not UTF-8".into())
}
fn time_left(deadline: Instant) -> Result<Duration, String> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|duration| !duration.is_zero())
        .ok_or_else(|| "Editor analysis timed out".into())
}
impl Bridge {
    pub(crate) fn new(root: PathBuf, connection: PathBuf) -> Self {
        Self {
            root,
            connection,
            active: Mutex::new(()),
        }
    }
    fn verify(&self, snapshot: &Snapshot) -> Result<(), String> {
        if snapshot.documents().len() > 8192 {
            return Err("Editor workspace exceeds file allowance".into());
        }
        let mut total = 0usize;
        for doc in snapshot.documents() {
            total = total.saturating_add(doc.bytes());
            if total > WORKSPACE_BYTES {
                return Err("Editor workspace exceeds byte allowance".into());
            }
            let path = self.root.join(doc.path());
            if path.canonicalize().map_err(|e| e.to_string())? != path {
                return Err("Editor source path changed or uses a symlink".into());
            }
            let current = read_file(&path, FILE_BYTES)?;
            let captured = doc
                .captured()
                .read_all(FILE_BYTES)
                .map_err(|e| e.to_string())?;
            if current != captured.bytes() {
                return Err("Editor analysis requires captured source to match saved files; refresh or use a managed server".into());
            }
        }
        Ok(())
    }
    fn request(&self, connection: &Connection, request: &Request<'_>) -> Result<Response, String> {
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, connection.port));
        let mut socket = TcpStream::connect_timeout(&address, Duration::from_secs(2))
            .map_err(|e| format!("VS Code bridge unavailable: {e}"))?;
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut frame = serde_json::to_vec(request).map_err(|e| e.to_string())?;
        frame.push(b'\n');
        if frame.len() > FRAME {
            return Err("Editor request exceeds allowance".into());
        }
        let mut remaining = frame.as_slice();
        while !remaining.is_empty() {
            socket
                .set_write_timeout(Some(time_left(deadline)?))
                .map_err(|e| e.to_string())?;
            let count = socket
                .write(remaining)
                .map_err(|e| format!("Editor write failed: {e}"))?;
            if count == 0 {
                return Err("Editor connection closed during write".into());
            }
            remaining = &remaining[count..];
        }
        frame.clear();
        let mut buffer = [0; 8192];
        loop {
            socket
                .set_read_timeout(Some(time_left(deadline)?))
                .map_err(|e| e.to_string())?;
            let count = socket
                .read(&mut buffer)
                .map_err(|e| format!("Editor response unavailable: {e}"))?;
            if count == 0 {
                return Err("Editor connection closed before a complete response".into());
            }
            frame.extend_from_slice(&buffer[..count]);
            if frame.len() > FRAME {
                return Err("Editor response exceeds allowance".into());
            }
            if let Some(end) = buffer[..count].iter().position(|byte| *byte == b'\n') {
                if end + 1 != count {
                    return Err("Invalid trailing editor response data".into());
                }
                return serde_json::from_slice(&frame)
                    .map_err(|e| format!("Invalid editor response: {e}"));
            }
        }
    }
    fn lookup(
        &self,
        snapshot: &Snapshot,
        doc: &Document,
        byte: usize,
        method: &str,
    ) -> Result<Answer, String> {
        let _active = self
            .active
            .try_lock()
            .map_err(|_| "Editor analysis is busy; retry navigation")?;
        let connection_path =
            crate::cache::external_path(&self.connection, &self.root).map_err(|e| e.to_string())?;
        let connection: Connection = serde_json::from_slice(&read_file(&connection_path, 4096)?)
            .map_err(|e| format!("Invalid VS Code connection file: {e}"))?;
        if connection.version != 1
            || connection.port == 0
            || connection.token.len() != 64
            || !connection
                .token
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("Unsupported or invalid VS Code connection file".into());
        }
        if !connection.root.is_absolute()
            || connection.root.canonicalize().ok().as_ref() != Some(&self.root)
        {
            return Err("VS Code connection belongs to another workspace root".into());
        }
        self.verify(snapshot)?;
        let text = source(doc)?;
        let response = self.request(
            &connection,
            &Request {
                version: 1,
                token: &connection.token,
                root: &connection.root,
                method,
                path: doc.path(),
                text: &text,
                position: definitions::position(&text, byte)?,
            },
        )?;
        if response.version != 1 {
            return Err("Unsupported editor response version".into());
        }
        let current: Connection = serde_json::from_slice(&read_file(&connection_path, 4096)?)
            .map_err(|e| format!("Invalid VS Code connection file: {e}"))?;
        if current != connection {
            return Err("VS Code connection changed during analysis; retry navigation".into());
        }
        let (session, epoch, targets, mut outside) = match response.outcome {
            Outcome::Ok {
                session,
                epoch,
                locations,
                outside,
            } => (session, epoch, locations, outside),
            Outcome::Error { message } => {
                return Err(format!("VS Code analysis unavailable: {message}"));
            }
        };
        if session.is_empty() || session.len() > 128 || targets.len() > RESULTS || outside > 65536 {
            return Err("Invalid or oversized editor result".into());
        }
        self.verify(snapshot)?;
        let mut locations = vec![];
        for target in targets {
            if !target.path.is_absolute() {
                return Err("Editor target must be an absolute file path".into());
            }
            let Ok(relative) = target
                .path
                .strip_prefix(&connection.root)
                .or_else(|_| target.path.strip_prefix(&self.root))
            else {
                outside += 1;
                continue;
            };
            let relative = relative
                .to_str()
                .ok_or("Editor target is not UTF-8")?
                .replace('\\', "/");
            let Some(doc) = snapshot.file_by_path(&relative) else {
                outside += 1;
                continue;
            };
            let captured = source(doc)?;
            if target.text != captured {
                return Err("Editor target differs from captured source".into());
            }
            definitions::byte_offset(&captured, target.position)?;
            locations.push(Location {
                capture: None,
                path: relative,
                position: target.position,
            });
        }
        Ok(Answer {
            locations,
            outside,
            context: ContextId::derive(&[
                b"editor-provider-v1",
                session.as_bytes(),
                &epoch.to_le_bytes(),
                snapshot.id().as_str().as_bytes(),
            ])
            .as_str()
            .to_owned(),
        })
    }
}
impl Provider for Bridge {
    fn resolve(&self, snapshot: Arc<Snapshot>, call: &CallLookup) -> Result<Answer, String> {
        let doc = snapshot
            .file(&call.invocation.source.file)
            .ok_or("Captured editor caller is unavailable")?;
        if doc.version().as_str() != call.invocation.source.version {
            return Err("Captured caller version differs".into());
        }
        let identifier = call
            .identifier
            .as_ref()
            .ok_or("Captured call identifier is unavailable")?;
        if source(doc)?.get(identifier.clone()) != call.invocation.name.as_deref() {
            return Err("Captured call identifier differs".into());
        }
        self.lookup(&snapshot, doc, identifier.start, "definition")
    }
    fn references(
        &self,
        snapshot: Arc<Snapshot>,
        symbol: &Symbol,
        identifier: usize,
    ) -> Result<Answer, String> {
        if symbol.origin.is_some() {
            return Err("Editor dependency references are outside capture".into());
        }
        let doc = snapshot
            .file(&symbol.source.file)
            .ok_or("Captured editor declaration is unavailable")?;
        if doc.version().as_str() != symbol.source.version {
            return Err("Captured declaration version differs".into());
        }
        let fact = doc
            .analyses()
            .iter()
            .filter(|analysis| analysis.context.id.as_str() == symbol.source.context)
            .flat_map(|analysis| &analysis.facts)
            .find(|fact| {
                fact.kind == symbol.kind
                    && fact
                        .name_span
                        .as_ref()
                        .is_some_and(|span| span.start == identifier)
            })
            .ok_or("Captured declaration identifier is unavailable")?;
        if !fact.name.starts_with(&symbol.name) {
            return Err("Captured declaration name differs".into());
        }
        let language = doc
            .analyses()
            .first()
            .map(|analysis| analysis.context.language.as_str());
        self.lookup(
            &snapshot,
            doc,
            identifier + definitions::reference_offset(language, &fact.name),
            "references",
        )
    }
}
