//! Launch-pinned editor callback; semantic discovery never redirects navigation.
use super::{Editor, EditorError, EditorLocation};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const FRAME: usize = 64 * 1024;
const SOURCE_BYTES: usize = 16 * 1024 * 1024;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Connection {
    protocol: String,
    version: u8,
    root: PathBuf,
    editor: String,
    session: String,
    port: u16,
    token: String,
}
#[derive(Serialize)]
struct Request<'a> {
    protocol: &'static str,
    version: u8,
    root: &'a Path,
    session: &'a str,
    token: &'a str,
    request: &'a str,
    path: &'a Path,
    line: usize,
    source: &'a super::SourceWitness,
    expires_at_ms: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    protocol: String,
    version: u8,
    session: String,
    request: String,
    status: String,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

/// A verified-location adapter permanently bound to one launching editor instance.
pub struct OriginEditor {
    connection: Connection,
}
fn error(message: impl Into<String>) -> EditorError {
    EditorError(message.into())
}
fn hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn remaining(deadline: Instant) -> Result<Duration, EditorError> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or_else(|| {
            error("Could not confirm navigation in the originating editor before its deadline")
        })
}
impl OriginEditor {
    /// Reads a private descriptor once; later replacement cannot change the origin.
    pub fn from_file(root: &Path, file: &Path) -> Result<Self, EditorError> {
        let root = root.canonicalize().map_err(|e| error(e.to_string()))?;
        let file = file
            .canonicalize()
            .map_err(|e| error(format!("Cannot read originating editor connection: {e}")))?;
        if file.starts_with(&root) {
            return Err(error(
                "Originating editor connection must stay outside captured source",
            ));
        }
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32);
        }
        let input = options.open(file).map_err(|e| error(e.to_string()))?;
        let metadata = input.metadata().map_err(|e| error(e.to_string()))?;
        if !metadata.is_file() || metadata.len() > 4096 {
            return Err(error(
                "Originating editor connection must be a regular file of at most 4096 bytes",
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(error(
                    "Originating editor connection must have private permissions",
                ));
            }
        }
        let mut bytes = Vec::new();
        input
            .take(4097)
            .read_to_end(&mut bytes)
            .map_err(|e| error(e.to_string()))?;
        if bytes.len() > 4096 {
            return Err(error("Originating editor connection exceeds allowance"));
        }
        let connection: Connection = serde_json::from_slice(&bytes)
            .map_err(|e| error(format!("Invalid originating editor connection: {e}")))?;
        if connection.protocol != "vizzi-origin"
            || connection.version != 1
            || connection.root != root
            || connection.port == 0
            || !hex(&connection.token)
            || !hex(&connection.session)
            || connection.editor.is_empty()
            || connection.editor.len() > 64
            || connection.editor.chars().any(char::is_control)
        {
            return Err(error(
                "Invalid originating editor protocol, root or identity",
            ));
        }
        Ok(Self { connection })
    }
}
impl Editor for OriginEditor {
    fn name(&self) -> &str {
        &self.connection.editor
    }
    fn open(&self, location: &EditorLocation) -> Result<(), EditorError> {
        if !location.path.is_absolute()
            || location.path.as_os_str().as_encoded_bytes().len() > 4096
            || location.line == 0
            || location.source.bytes > SOURCE_BYTES
            || !hex(&location.source.sha256)
        {
            return Err(error("Invalid verified originating editor destination"));
        }
        let mut random = [0; 32];
        getrandom::fill(&mut random).map_err(|e| error(e.to_string()))?;
        let request_id = blake3::Hash::from_bytes(random).to_hex().to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| error(e.to_string()))?
            .as_millis();
        let expires_at_ms = u64::try_from(now + 8000).map_err(|e| error(e.to_string()))?;
        let connection = &self.connection;
        let deadline = Instant::now() + Duration::from_secs(10);
        let request = Request {
            protocol: "vizzi-origin",
            version: 1,
            root: &connection.root,
            session: &connection.session,
            token: &connection.token,
            request: &request_id,
            path: &location.path,
            line: location.line,
            source: &location.source,
            expires_at_ms,
        };
        let mut frame = serde_json::to_vec(&request).map_err(|e| error(e.to_string()))?;
        frame.push(b'\n');
        if frame.len() > FRAME {
            return Err(error("Originating editor request exceeds frame allowance"));
        }
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, connection.port));
        let mut stream =
            TcpStream::connect_timeout(&address, remaining(deadline)?.min(Duration::from_secs(2)))
                .map_err(|e| {
                    error(format!(
                        "The originating editor is unavailable; reopen Vizzi from that window. {e}"
                    ))
                })?;
        stream
            .set_write_timeout(Some(remaining(deadline)?))
            .map_err(|e| error(e.to_string()))?;
        stream.write_all(&frame).map_err(|e| {
            error(format!(
                "Could not confirm navigation in the originating editor: {e}"
            ))
        })?;
        let mut bytes = Vec::new();
        let mut chunk = [0; 4096];
        loop {
            stream
                .set_read_timeout(Some(remaining(deadline)?))
                .map_err(|e| error(e.to_string()))?;
            let n = stream.read(&mut chunk).map_err(|e| {
                error(format!(
                    "Could not confirm navigation in the originating editor: {e}"
                ))
            })?;
            if n == 0 {
                return Err(error(
                    "Could not confirm navigation in the originating editor; its connection closed",
                ));
            }
            bytes.extend_from_slice(&chunk[..n]);
            if bytes.len() > FRAME {
                return Err(error("Originating editor reply exceeds frame allowance"));
            }
            if let Some(end) = bytes.iter().position(|&c| c == b'\n') {
                if end + 1 != bytes.len() {
                    return Err(error("Invalid originating editor reply frame"));
                }
                break;
            }
        }
        let response: Response = serde_json::from_slice(&bytes)
            .map_err(|e| error(format!("Invalid originating editor reply: {e}")))?;
        if response.protocol != "vizzi-origin"
            || response.version != 1
            || response.session != connection.session
            || response.request != request_id
        {
            return Err(error(
                "Could not confirm navigation in the originating editor: reply identity differs",
            ));
        }
        if response.code.as_ref().is_some_and(|s| s.len() > 64)
            || response.message.as_ref().is_some_and(|s| s.len() > 512)
        {
            return Err(error("Invalid originating editor refusal"));
        }
        match response.status.as_str() {
            "opened" => Ok(()),
            "error" => Err(error(
                response
                    .message
                    .filter(|m| m.len() <= 512)
                    .unwrap_or_else(|| {
                        format!(
                            "Originating editor refused navigation ({})",
                            response.code.unwrap_or_else(|| "unavailable".into())
                        )
                    }),
            )),
            _ => Err(error("Invalid originating editor acknowledgment")),
        }
    }
}
