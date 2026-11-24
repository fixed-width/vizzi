//! Editor-owned analysis over a bounded, authenticated local connection.
use crate::definitions::{self, Answer, Location, Position, Provider};
use crate::dependency_sources::{Capture, Catalog, Declaration, EditorArchive};
use crate::source::{BROWSE_JSON_BYTES, BROWSE_JSON_FILES};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use vizzi_core::{
    model::{ContentId, ContextId, Document, Snapshot},
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
    #[serde(rename = "providerUri", skip_serializing_if = "Option::is_none")]
    provider_uri: Option<&'a str>,
    #[serde(rename = "archiveBinary", skip_serializing_if = "Option::is_none")]
    archive_binary: Option<&'a Path>,
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
    #[serde(default)]
    declaration: Option<EditorDeclaration>,
    #[serde(rename = "providerUri", default)]
    provider_uri: Option<String>,
    #[serde(rename = "archiveBinary", default)]
    archive_binary: Option<PathBuf>,
}

#[derive(Deserialize)]
struct EditorRange {
    start: Position,
    end: Position,
}
#[derive(Deserialize)]
struct EditorDeclaration {
    name: String,
    range: EditorRange,
    selection: EditorRange,
}

pub(crate) struct Bridge {
    root: PathBuf,
    connection: PathBuf,
    active: Mutex<()>,
    sources: Arc<Catalog>,
}
fn open_regular(path: &Path) -> Result<File, String> {
    let file = File::open(path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Editor connection/source must be a regular file".into());
    }
    Ok(file)
}
fn read_file(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    open_regular(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("Editor connection/source exceeds allowance".into());
    }
    Ok(bytes)
}
fn browse_json_matches(path: &Path, doc: &Document) -> Result<bool, String> {
    let limit = doc.bytes().saturating_add(1) as u64;
    let mut disk = BufReader::new(open_regular(path)?.take(limit));
    let mut verifier = doc.captured().verifier();
    loop {
        let bytes = disk.fill_buf().map_err(|e| e.to_string())?;
        if bytes.is_empty() {
            return Ok(verifier.matches());
        }
        verifier.update(bytes);
        let count = bytes.len();
        disk.consume(count);
    }
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
    pub(crate) fn new(root: PathBuf, connection: PathBuf, sources: Arc<Catalog>) -> Self {
        Self {
            root,
            connection,
            active: Mutex::new(()),
            sources,
        }
    }
    fn archive_target(
        &self,
        target: &Target,
        binary: &Path,
        configuration: &str,
    ) -> Result<Option<Arc<Capture>>, String> {
        let Some(uri) = target
            .provider_uri
            .as_ref()
            .and_then(|uri| url::Url::parse(uri).ok())
        else {
            return Ok(None);
        };
        if !binary.is_absolute()
            || binary.extension().is_none_or(|ext| ext != "jar")
            || binary.canonicalize().ok().as_deref() != Some(binary)
            || !binary.is_file()
            || target.path.canonicalize().ok().as_deref() != Some(target.path.as_path())
        {
            return Ok(None);
        }
        let Ok(input) =
            crate::java_archive::input(&uri, &target.path, binary, &target.text, configuration)
        else {
            return Ok(None);
        };
        definitions::byte_offset(&target.text, target.position)?;
        self.sources
            .capture_editor_archive(
                input,
                EditorArchive {
                    uri: target
                        .provider_uri
                        .clone()
                        .ok_or("Editor archive URI is unavailable")?,
                    binary: binary.to_owned(),
                },
            )
            .map(Some)
    }
    fn dependency_target(
        &self,
        target: &Target,
        configuration: &str,
        editor_root: &Path,
    ) -> Result<Option<Arc<Capture>>, String> {
        if target.path.canonicalize().ok().as_ref() != Some(&target.path) {
            return Ok(None);
        }
        let rust = target
            .path
            .extension()
            .and_then(|extension| extension.to_str())
            == Some("rs")
            && !target.path.starts_with(&self.root);
        let input = if rust {
            crate::rust_dependencies::input(&target.path, configuration)
        } else {
            crate::ecmascript_dependencies::input(&self.root, &target.path, configuration)
        };
        let Ok(mut input) = input else {
            return Ok(None);
        };
        if let (Some(package), Some(uri)) = (&mut input.installation, &target.provider_uri) {
            if self
                .verify_package_uri(uri, &target.path, editor_root)
                .is_err()
            {
                return Ok(None);
            }
            package.editor_uri = Some(uri.clone());
        }
        let Ok(mut captured) = self.sources.capture(input.clone()) else {
            return Ok(None);
        };
        if source(captured.doc())? != target.text {
            return Err("Editor dependency target differs from saved source".into());
        }
        let byte = definitions::byte_offset(&target.text, target.position)?;
        if rust
            && captured
                .index
                .definition(captured.doc().path(), byte)
                .is_none()
            && let Some(range) = &target.declaration
        {
            let start = definitions::byte_offset(&target.text, range.range.start)?;
            let end = definitions::byte_offset(&target.text, range.range.end)?;
            if !(start..end).contains(&byte) {
                return Err("Editor declaration does not contain the returned target".into());
            }
            let declaration = Declaration {
                content: ContentId::derive(&[target.text.as_bytes()]),
                name: range.name.clone(),
                span: start..end,
                identifier: definitions::byte_offset(&target.text, range.selection.start)?
                    ..definitions::byte_offset(&target.text, range.selection.end)?,
            };
            captured = self.sources.capture_with_declaration(input, declaration)?;
        }
        if source(captured.doc())? != target.text {
            return Err("Editor dependency target differs from captured source".into());
        }
        captured.verify()?;
        Ok(Some(captured))
    }

    fn verify_package_uri(&self, uri: &str, file: &Path, editor_root: &Path) -> Result<(), String> {
        let path = url::Url::parse(uri)
            .ok()
            .and_then(|url| url.to_file_path().ok())
            .ok_or("Editor package URI is not a local file")?;
        if (!crate::ecmascript_dependencies::installed(&self.root, &path)
            && !crate::ecmascript_dependencies::installed(editor_root, &path))
            || path.canonicalize().ok().as_deref() != Some(file)
        {
            return Err("Editor package resolution path changed; reopen it from its caller".into());
        }
        Ok(())
    }

    fn verify_dependency(&self, capture: &Capture, editor_root: &Path) -> Result<(), String> {
        capture.verify()?;
        if let Some(editor) = &capture.editor_archive
            && (editor.binary.canonicalize().ok().as_ref() != Some(&editor.binary)
                || !editor.binary.is_file())
        {
            return Err("Editor archive library changed; reopen it from its caller".into());
        }
        if let Some(uri) = capture
            .input
            .installation
            .as_ref()
            .and_then(|package| package.editor_uri.as_deref())
        {
            let path = capture
                .path()
                .ok_or("Editor package source needs a local file")?;
            self.verify_package_uri(uri, &path, editor_root)?;
        }
        Ok(())
    }
    fn verify(&self, snapshot: &Snapshot) -> Result<(), String> {
        let browse_files = snapshot
            .documents()
            .iter()
            .filter(|doc| doc.classification().is_browse_only_json())
            .count();
        if snapshot.documents().len() - browse_files > 8192 || browse_files > BROWSE_JSON_FILES {
            return Err("Editor workspace exceeds file allowance".into());
        }
        let mut total = 0usize;
        let mut browse_total = 0usize;
        for doc in snapshot.documents() {
            let browse_json = doc.classification().is_browse_only_json();
            let (charged, limit) = if browse_json {
                (&mut browse_total, BROWSE_JSON_BYTES)
            } else {
                (&mut total, WORKSPACE_BYTES)
            };
            *charged = charged.saturating_add(doc.bytes());
            if *charged > limit {
                return Err("Editor workspace exceeds byte allowance".into());
            }
            let path = self.root.join(doc.path());
            if path.canonicalize().map_err(|e| e.to_string())? != path {
                return Err("Editor source path changed or uses a symlink".into());
            }
            let matches = if browse_json {
                browse_json_matches(&path, doc)?
            } else {
                let current = read_file(&path, FILE_BYTES)?;
                let captured = doc
                    .captured()
                    .read_all(FILE_BYTES)
                    .map_err(|e| e.to_string())?;
                current == captured.bytes()
            };
            if !matches {
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
        dependency: Option<&Capture>,
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
        if let Some(source) = dependency {
            self.verify_dependency(source, &connection.root)?;
        }
        let text = source(doc)?;
        let archive = dependency.and_then(|source| source.editor_archive.as_ref());
        let external_path = dependency
            .map(|source| {
                if archive.is_some() {
                    Ok(source.input.root.clone())
                } else {
                    source
                        .path()
                        .ok_or("Editor dependency source needs a local file")
                }
            })
            .transpose()?;
        let query_path = external_path
            .as_ref()
            .map(|path| path.to_str().ok_or("Editor dependency path is not UTF-8"))
            .transpose()?
            .unwrap_or(doc.path());
        let query_method = if dependency.is_some() {
            format!("dependency-{method}")
        } else {
            method.into()
        };
        let response = self.request(
            &connection,
            &Request {
                version: 1,
                token: &connection.token,
                root: &connection.root,
                method: &query_method,
                path: query_path,
                text: &text,
                position: definitions::position(&text, byte)?,
                provider_uri: archive.map(|archive| archive.uri.as_str()).or_else(|| {
                    dependency
                        .and_then(|source| source.input.installation.as_ref())
                        .and_then(|package| package.editor_uri.as_deref())
                }),
                archive_binary: archive.map(|archive| archive.binary.as_path()),
            },
        )?;
        if response.version != 1 {
            return Err("Unsupported editor response version".into());
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
        let configuration = ContextId::derive(&[
            b"editor-dependencies-v1",
            self.root.as_os_str().as_encoded_bytes(),
        ]);
        let mut locations = vec![];
        let mut captured_targets = vec![];
        for target in targets {
            if let Some(binary) = &target.archive_binary {
                let captured = if doc.classification().assigned_language() == Some("java") {
                    self.archive_target(&target, binary, configuration.as_str())?
                } else {
                    None
                };
                if let Some(captured) = captured {
                    locations.push(Location {
                        capture: Some(captured.origin.capture.clone()),
                        path: captured.doc().path().into(),
                        position: target.position,
                    });
                    captured_targets.push(captured);
                } else {
                    outside += 1;
                }
                continue;
            }
            if !target.path.is_absolute() {
                return Err("Editor target must be an absolute file path".into());
            }
            let relative = target
                .path
                .strip_prefix(&connection.root)
                .or_else(|_| target.path.strip_prefix(&self.root))
                .ok()
                .map(|path| {
                    path.to_str()
                        .map(|path| path.replace('\\', "/"))
                        .ok_or("Editor target is not UTF-8")
                })
                .transpose()?;
            let Some(doc) = relative
                .as_deref()
                .and_then(|path| snapshot.file_by_path(path))
            else {
                if let Some(captured) =
                    self.dependency_target(&target, configuration.as_str(), &connection.root)?
                {
                    locations.push(Location {
                        capture: Some(captured.origin.capture.clone()),
                        path: captured.doc().path().into(),
                        position: target.position,
                    });
                    captured_targets.push(captured);
                } else {
                    outside += 1;
                }
                continue;
            };
            let captured = source(doc)?;
            if target.text != captured {
                return Err("Editor target differs from captured source".into());
            }
            definitions::byte_offset(&captured, target.position)?;
            locations.push(Location {
                capture: None,
                path: doc.path().into(),
                position: target.position,
            });
        }
        self.verify(snapshot)?;
        if let Some(source) = dependency {
            self.verify_dependency(source, &connection.root)?;
        }
        for captured in captured_targets {
            self.verify_dependency(&captured, &connection.root)?;
        }
        let current: Connection = serde_json::from_slice(&read_file(&connection_path, 4096)?)
            .map_err(|e| format!("Invalid VS Code connection file: {e}"))?;
        if current != connection {
            return Err("VS Code connection changed during analysis; retry navigation".into());
        }
        Ok(Answer {
            locations,
            outside,
            context: ContextId::derive(&[
                b"editor-provider-v1",
                session.as_bytes(),
                &epoch.to_le_bytes(),
                snapshot.id().as_str().as_bytes(),
                doc.version().as_str().as_bytes(),
            ])
            .as_str()
            .to_owned(),
        })
    }
}
impl Provider for Bridge {
    fn dependencies(&self) -> Option<&Catalog> {
        Some(&self.sources)
    }
    fn resolve(&self, snapshot: Arc<Snapshot>, call: &CallLookup) -> Result<Answer, String> {
        let external = if snapshot.file(&call.invocation.source.file).is_none() {
            self.sources
                .sources()?
                .into_iter()
                .find(|source| source.index.call(&call.invocation.id).is_some())
        } else {
            None
        };
        let doc = external
            .as_ref()
            .map(|source| source.doc())
            .or_else(|| {
                snapshot
                    .file(&call.invocation.source.file)
                    .map(AsRef::as_ref)
            })
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
        self.lookup(
            &snapshot,
            doc,
            identifier.start,
            "definition",
            external.as_deref(),
        )
    }
    fn references(
        &self,
        snapshot: Arc<Snapshot>,
        symbol: &Symbol,
        identifier: usize,
    ) -> Result<Answer, String> {
        let external = symbol
            .origin
            .as_ref()
            .map(|origin| self.sources.get(&origin.capture))
            .transpose()?
            .flatten();
        if symbol.origin.is_some() && external.is_none() {
            return Err("Captured editor dependency is unavailable".into());
        }
        let doc = external
            .as_ref()
            .map(|source| source.doc())
            .or_else(|| snapshot.file(&symbol.source.file).map(AsRef::as_ref))
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
            external.as_deref(),
        )
    }
}
