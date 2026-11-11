//! Bounded stdio JSON-RPC client used by the native definition adapter.
use crate::definitions::Position;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use url::Url;
const FRAME: usize = 4 * 1024 * 1024;
fn local_sysroot_source(root: &Path) -> PathBuf {
    if let Some(path) = std::env::var_os("RUST_SRC_PATH").map(PathBuf::from)
        && path.is_absolute()
        && path.join("core").is_dir()
    {
        return path;
    }
    let missing = root.join(".vizzi-unavailable-rust-src");
    let Ok(mut child) = Command::new("rustc")
        .args(["--print", "sysroot"])
        .current_dir(root)
        .env("RUSTUP_AUTO_INSTALL", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return missing;
    };
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut value = String::new();
                if status.success()
                    && let Some(output) = child.stdout.take()
                    && output.take(4097).read_to_string(&mut value).is_ok()
                    && value.len() <= 4096
                {
                    let source = PathBuf::from(value.trim()).join("lib/rustlib/src/rust/library");
                    if source.is_absolute() && source.join("core").is_dir() {
                        return source;
                    }
                }
                return missing;
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return missing;
            }
        }
    }
}
fn read_message(reader: &mut impl BufRead) -> Result<Option<Value>, String> {
    let mut length = None;
    let mut total = 0;
    loop {
        let mut line = Vec::new();
        let count = Read::take(&mut *reader, 8193)
            .read_until(b'\n', &mut line)
            .map_err(|e| e.to_string())?;
        if count == 0 {
            return if total == 0 {
                Ok(None)
            } else {
                Err("Incomplete LSP header".into())
            };
        }
        total += count;
        if total > 8192 {
            return Err("LSP header exceeds allowance".into());
        }
        if line == b"\r\n" || line == b"\n" {
            break;
        }
        let header = std::str::from_utf8(&line).map_err(|e| e.to_string())?;
        let (key, value) = header
            .trim_end()
            .split_once(':')
            .ok_or("Invalid LSP header")?;
        if key.eq_ignore_ascii_case("Content-Length") {
            if length.is_some() {
                return Err("Duplicate LSP length".into());
            }
            let bytes = value.trim().parse::<usize>().map_err(|e| e.to_string())?;
            if bytes > FRAME {
                return Err("LSP message exceeds allowance".into());
            }
            length = Some(bytes);
        }
    }
    let mut bytes = vec![0; length.ok_or("Missing LSP length")?];
    reader.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|e| e.to_string())
}
fn write_message(writer: &mut impl Write, value: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("LSP request exceeds allowance".into());
    }
    write!(writer, "Content-Length: {}\r\n\r\n", bytes.len()).map_err(|e| e.to_string())?;
    writer
        .write_all(&bytes)
        .and_then(|()| writer.flush())
        .map_err(|e| e.to_string())
}
pub(crate) fn uri(path: &Path) -> Result<String, String> {
    Url::from_file_path(path)
        .map(|u| u.to_string())
        .map_err(|()| "Invalid language-server file path".into())
}
pub(crate) struct Client {
    child: Child,
    input: ChildStdin,
    messages: Receiver<Result<Value, String>>,
    reader: Option<JoinHandle<()>>,
    next: u64,
    options: Value,
    initialization: Value,
    status_notifications: bool,
    diagnostics: BTreeMap<String, i32>,
    ready: bool,
    pub version: String,
}
pub(crate) struct Setup {
    pub arguments: Vec<String>,
    pub initialization: Value,
    pub configuration: Value,
    pub status_notifications: bool,
}
impl Client {
    pub(crate) fn start(executable: &Path, root: &Path) -> Result<Self, String> {
        let options = json!({"cargo":{"sysrootSrc":local_sysroot_source(root),"buildScripts":{"enable":false},"autoreload":true,"extraArgs":["--offline"]},"procMacro":{"enable":false},"checkOnSave":false,"cachePriming":{"enable":false,"numThreads":1},"numThreads":1,"diagnostics":{"enable":false},"lru":{"capacity":64}});
        Self::configured(
            executable,
            root,
            Setup {
                arguments: vec![],
                initialization: options.clone(),
                configuration: json!({"rust-analyzer":options}),
                status_notifications: true,
            },
        )
    }
    pub(crate) fn configured(executable: &Path, root: &Path, setup: Setup) -> Result<Self, String> {
        let executable = if executable.is_relative() && executable.components().count() > 1 {
            std::env::current_dir()
                .map_err(|e| e.to_string())?
                .join(executable)
        } else {
            executable.to_owned()
        };
        let mut command = if cfg!(unix) {
            let mut c = Command::new("nice");
            c.args(["-n", "10"]).arg(&executable);
            c
        } else {
            Command::new(&executable)
        };
        let mut child = command
            .args(&setup.arguments)
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .env("CARGO_NET_OFFLINE", "true")
            .env("RUSTUP_AUTO_INSTALL", "0")
            .env("CARGO_BUILD_JOBS", "1")
            .env("RAYON_NUM_THREADS", "1")
            .env("CARGO_TARGET_DIR", root.join("target"))
            .env("RUSTC", "rustc")
            .env("CARGO_BUILD_RUSTC", "rustc")
            .env("RUSTC_WRAPPER", "")
            .env("RUSTC_WORKSPACE_WRAPPER", "")
            .env("CARGO_BUILD_RUSTC_WRAPPER", "")
            .env("CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER", "")
            .spawn()
            .map_err(|e| format!("Cannot start language server: {e}"))?;
        let input = child
            .stdin
            .take()
            .ok_or("Language server stdin unavailable")?;
        let output = child
            .stdout
            .take()
            .ok_or("Language server stdout unavailable")?;
        let (tx, messages) = mpsc::sync_channel(32);
        let reader = thread::spawn(move || {
            let mut reader = BufReader::new(output);
            loop {
                match read_message(&mut reader) {
                    Ok(Some(value)) => {
                        if tx.send(Ok(value)).is_err() {
                            break;
                        }
                    }
                    Ok(None) => {
                        let _ = tx.send(Err("Language server closed its output".into()));
                        break;
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        break;
                    }
                }
            }
        });
        let mut client = Self {
            child,
            input,
            messages,
            reader: Some(reader),
            next: 0,
            options: setup.configuration,
            initialization: setup.initialization,
            status_notifications: setup.status_notifications,
            diagnostics: BTreeMap::new(),
            ready: !setup.status_notifications,
            version: String::new(),
        };
        let result=client.request("initialize",json!({"processId":null,"rootUri":uri(root)?,"workspaceFolders":[{"uri":uri(root)?,"name":"captured source"}],"capabilities":{"general":{"positionEncodings":["utf-16"]},"textDocument":{"definition":{"linkSupport":true},"references":{},"publishDiagnostics":{"versionSupport":true}},"experimental":{"serverStatusNotification":client.status_notifications},"workspace":{"configuration":true}},"initializationOptions":client.initialization}),Duration::from_secs(20))?;
        if !result["capabilities"]["definitionProvider"].is_object()
            && result["capabilities"]["definitionProvider"] != true
        {
            return Err("Language server does not support definitions".into());
        }
        if result["capabilities"]["positionEncoding"]
            .as_str()
            .is_some_and(|s| s != "utf-16")
        {
            return Err("Language server did not negotiate UTF-16".into());
        }
        client.version = result["serverInfo"]["version"]
            .as_str()
            .unwrap_or("unknown")
            .to_owned();
        client.notify("initialized", json!({}))?;
        Ok(client)
    }
    pub(crate) fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        if matches!(method, "textDocument/didOpen" | "textDocument/didChange")
            && let Some(uri) = params["textDocument"]["uri"].as_str()
        {
            self.diagnostics.entry(uri.into()).or_insert(i32::MIN);
        }
        if method == "textDocument/didClose"
            && let Some(uri) = params["textDocument"]["uri"].as_str()
        {
            self.diagnostics.remove(uri);
        }
        write_message(
            &mut self.input,
            &json!({"jsonrpc":"2.0","method":method,"params":params}),
        )
    }
    fn event(&mut self, message: Value) -> Result<(), String> {
        let Some(method) = message["method"].as_str() else {
            return Ok(());
        };
        if self.status_notifications && method == "experimental/serverStatus" {
            self.ready = message["params"]["quiescent"] == true;
        }
        if method == "textDocument/publishDiagnostics"
            && let (Some(uri), Some(version)) = (
                message["params"]["uri"].as_str(),
                message["params"]["version"].as_i64(),
            )
            && let Ok(version) = i32::try_from(version)
            && let Some(current) = self.diagnostics.get_mut(uri)
        {
            *current = (*current).max(version);
        }
        if let Some(id) = message.get("id") {
            let result = match method {
                "workspace/configuration" => {
                    let items = message["params"]["items"]
                        .as_array()
                        .ok_or("Invalid configuration request")?;
                    if items.len() > 64 {
                        return Err("Too many configuration sections".into());
                    }
                    Value::Array(
                        items
                            .iter()
                            .map(|item| {
                                let section = item["section"].as_str().unwrap_or("");
                                let mut value = &self.options;
                                for key in section.split('.').filter(|s| !s.is_empty()) {
                                    value = &value[key];
                                }
                                value.clone()
                            })
                            .collect(),
                    )
                }
                "window/workDoneProgress/create"
                | "client/registerCapability"
                | "client/unregisterCapability" => Value::Null,
                _ => {
                    return write_message(
                        &mut self.input,
                        &json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Read-only definition client"}}),
                    );
                }
            };
            write_message(
                &mut self.input,
                &json!({"jsonrpc":"2.0","id":id,"result":result}),
            )?;
        }
        Ok(())
    }
    fn receive(&mut self, deadline: Instant) -> Result<Value, String> {
        self.messages
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .map_err(|_| "Language-server request timed out".to_owned())?
    }
    pub(crate) fn request(
        &mut self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, String> {
        self.next = self
            .next
            .checked_add(1)
            .ok_or("Language-server request identities exhausted")?;
        let id = self.next;
        write_message(
            &mut self.input,
            &json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}),
        )?;
        let deadline = Instant::now() + timeout;
        loop {
            let message = self.receive(deadline)?;
            if message.get("method").is_none() && message["id"] == id {
                if let Some(error) = message.get("error") {
                    return Err(format!(
                        "Language server: {}",
                        error["message"].as_str().unwrap_or("definition failed")
                    ));
                }
                return message
                    .get("result")
                    .cloned()
                    .ok_or_else(|| "Missing LSP result".into());
            }
            self.event(message)?;
        }
    }
    pub(crate) fn wait_ready(&mut self) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(20);
        while !self.ready {
            let message = self.receive(deadline)?;
            self.event(message)?;
        }
        Ok(())
    }
    pub(crate) fn wait_document(&mut self, path: &Path, version: i32) -> Result<(), String> {
        let uri = uri(path)?;
        let deadline = Instant::now() + Duration::from_secs(10);
        while self
            .diagnostics
            .get(&uri)
            .is_none_or(|current| *current < version)
        {
            let message = self.receive(deadline)?;
            self.event(message)?;
        }
        Ok(())
    }
    pub(crate) fn definitions(
        &mut self,
        path: &Path,
        position: Position,
    ) -> Result<Vec<(Option<PathBuf>, Position)>, String> {
        self.wait_ready()?;
        let result = self.request(
            "textDocument/definition",
            json!({"textDocument":{"uri":uri(path)?},"position":position}),
            Duration::from_secs(10),
        )?;
        locations(result)
    }
    pub(crate) fn references(
        &mut self,
        path: &Path,
        position: Position,
    ) -> Result<Vec<(Option<PathBuf>, Position)>, String> {
        self.wait_ready()?;
        let result=self.request("textDocument/references",json!({"textDocument":{"uri":uri(path)?},"position":position,"context":{"includeDeclaration":false}}),Duration::from_secs(10))?;
        locations_with_limit(result, 1024)
    }
}
fn locations(result: Value) -> Result<Vec<(Option<PathBuf>, Position)>, String> {
    locations_with_limit(result, 64)
}
fn locations_with_limit(
    result: Value,
    limit: usize,
) -> Result<Vec<(Option<PathBuf>, Position)>, String> {
    let items = match result {
        Value::Null => vec![],
        Value::Array(v) => v,
        v if v.is_object() => vec![v],
        _ => return Err("Invalid definition result".into()),
    };
    if items.len() > limit {
        return Err("Too many definition destinations".into());
    }
    let mut output = vec![];
    let mut seen = std::collections::BTreeSet::new();
    for item in items {
        let (uri, point) = if item.get("targetUri").is_some() {
            (&item["targetUri"], &item["targetSelectionRange"]["start"])
        } else {
            (&item["uri"], &item["range"]["start"])
        };
        let uri =
            Url::parse(uri.as_str().ok_or("Missing definition URI")?).map_err(|e| e.to_string())?;
        let path = uri.to_file_path().ok();
        let position =
            serde_json::from_value::<Position>(point.clone()).map_err(|e| e.to_string())?;
        if seen.insert((uri.to_string(), position.line, position.character)) {
            output.push((path, position));
        }
    }
    Ok(output)
}
impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.messages = mpsc::sync_channel(0).1;
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utf8_frames_round_trip_and_reject_unbounded_or_incomplete_messages() {
        let value = json!({"jsonrpc":"2.0","id":3,"result":"λ😀"});
        let mut bytes = vec![];
        write_message(&mut bytes, &value).unwrap();
        assert_eq!(read_message(&mut bytes.as_slice()).unwrap(), Some(value));
        for bytes in [
            b"Content-Length: 4194305\r\n\r\n".as_slice(),
            b"Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}",
            b"Content-Type: test\r\n\r\n{}",
            b"Content-Length: 20\r\n\r\n{}",
        ] {
            assert!(read_message(&mut &*bytes).is_err());
        }
        let headers = "Unused: value\r\n".repeat(700) + "Content-Length: 2\r\n\r\n{}";
        assert!(read_message(&mut headers.as_bytes()).is_err());
        let prefix = "Content-Length: 2\r\n";
        let padding = 8192 - prefix.len() - "X: ".len() - 4;
        let exact = format!("{prefix}X: {}\r\n\r\n{{}}", "a".repeat(padding));
        assert_eq!(
            read_message(&mut exact.as_bytes()).unwrap(),
            Some(json!({}))
        );
        let payload = format!("\"{}\"", "a".repeat(FRAME - 2));
        let frame = format!("Content-Length: {FRAME}\r\n\r\n{payload}");
        assert_eq!(
            read_message(&mut frame.as_bytes())
                .unwrap()
                .unwrap()
                .as_str()
                .unwrap()
                .len(),
            FRAME - 2
        );
        assert!(read_message(&mut b"".as_slice()).unwrap().is_none());
    }
    #[test]
    fn definition_links_and_locations_preserve_paths_and_utf16_positions() {
        let path = std::env::temp_dir().join("file with λ.rs");
        let u = uri(&path).unwrap();
        let point = json!({"line":7,"character":5});
        for result in [
            json!({"uri":u,"range":{"start":point}}),
            json!([{"targetUri":u,"targetSelectionRange":{"start":point}}]),
        ] {
            assert_eq!(
                locations(result).unwrap(),
                vec![(
                    Some(path.clone()),
                    Position {
                        line: 7,
                        character: 5
                    }
                )]
            );
        }
        let many = (0..64)
            .map(|line| json!({"uri":u,"range":{"start":{"line":line,"character":0}}}))
            .collect::<Vec<_>>();
        assert_eq!(locations(Value::Array(many.clone())).unwrap().len(), 64);
        let mut too_many = many;
        too_many.push(json!({"uri":u,"range":{"start":point}}));
        assert!(locations(Value::Array(too_many)).is_err());
        assert!(locations(Value::Null).unwrap().is_empty());
        assert_eq!(
            locations(json!({"uri":"https://example.com/a.rs","range":{"start":point}})).unwrap(),
            vec![(
                None,
                Position {
                    line: 7,
                    character: 5
                }
            )]
        );
        assert!(locations(json!([{"uri":u,"range":{"start":{"line":-1,"character":0}}}])).is_err());
    }
}

#[cfg(all(test, unix))]
mod peer_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn peer_requests_are_read_only_responses_correlate_and_deadlines_close_the_process() {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("peer");
        std::fs::write(&executable,r#"#!/usr/bin/env python3
import json,sys

def read():
 h={}
 while True:
  line=sys.stdin.buffer.readline()
  if not line:sys.exit(0)
  if line==b'\r\n':break
  k,v=line.decode().split(':',1);h[k.lower()]=v.strip()
 return json.loads(sys.stdin.buffer.read(int(h['content-length'])))
def send(m):
 b=json.dumps({'jsonrpc':'2.0',**m}).encode();sys.stdout.buffer.write(('Content-Length: %d\r\n\r\n'%len(b)).encode()+b);sys.stdout.buffer.flush()
request=read()
send({'id':'config','method':'workspace/configuration','params':{'items':[{'section':'rust-analyzer'}]}})
config=read()['result'][0]
assert config['cargo']['buildScripts']['enable']==False and config['procMacro']['enable']==False and config['numThreads']==1
assert isinstance(config['cargo']['sysrootSrc'],str) and config['cargo']['sysrootSrc']
import os
assert os.environ['RUSTUP_AUTO_INSTALL']=='0'
send({'id':'edit','method':'workspace/applyEdit','params':{'edit':{}}})
assert read()['error']['code']==-32601
send({'id':request['id'],'result':{'capabilities':{'definitionProvider':True,'positionEncoding':'utf-16'},'serverInfo':{'version':'fixture'}}})
while True:
 m=read()
 if m.get('method')=='initialized':send({'method':'experimental/serverStatus','params':{'quiescent':True}})
 elif m.get('method')=='ping':
  send({'id':m['id']+10,'result':'unrelated'})
  send({'id':m['id'],'result':'matched'})
"#).unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut client = Client::start(&executable, root.path()).unwrap();
        client.wait_ready().unwrap();
        assert_eq!(client.version, "fixture");
        assert_eq!(
            client
                .request("ping", Value::Null, Duration::from_secs(1))
                .unwrap(),
            "matched"
        );
        assert!(
            client
                .request("stall", Value::Null, Duration::from_millis(20))
                .unwrap_err()
                .contains("timed out")
        );
        drop(client);
    }
}

#[cfg(all(test, unix))]
mod standard_peer_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn python_configuration_and_stdio_startup_do_not_require_rust_status_notifications() {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("peer");
        std::fs::write(&executable,r#"#!/usr/bin/env python3
import json,sys
assert sys.argv[1:]==['--stdio']
def read():
 h={}
 while True:
  line=sys.stdin.buffer.readline()
  if not line:sys.exit(0)
  if line==b'\r\n':break
  k,v=line.decode().split(':',1);h[k.lower()]=v.strip()
 return json.loads(sys.stdin.buffer.read(int(h['content-length'])))
def send(m):
 b=json.dumps({'jsonrpc':'2.0',**m}).encode();sys.stdout.buffer.write(('Content-Length: %d\r\n\r\n'%len(b)).encode()+b);sys.stdout.buffer.flush()
m=read()
assert m['params']['initializationOptions']=={'mode':'python'}
assert m['params']['capabilities']['experimental']['serverStatusNotification']==False
send({'id':'cfg','method':'workspace/configuration','params':{'items':[{'section':'python'},{'section':'python.analysis'},{'section':'unknown'},{}]}})
cfg=read()['result'];assert cfg==[{'analysis':{'useLibraryCodeForTypes':True}},{'useLibraryCodeForTypes':True},None,{'python':{'analysis':{'useLibraryCodeForTypes':True}}}]
send({'id':m['id'],'result':{'capabilities':{'definitionProvider':True},'serverInfo':{'version':'python-peer'}}})
assert read()['method']=='initialized'
while True:
 m=read()
 if m.get('method')=='textDocument/didOpen':
  doc=m['params']['textDocument'];assert doc['languageId']=='python'
  send({'method':'textDocument/publishDiagnostics','params':{'uri':doc['uri'],'version':doc['version']-1,'diagnostics':[]}})
  send({'method':'textDocument/publishDiagnostics','params':{'uri':doc['uri'],'version':doc['version'],'diagnostics':[]}})
 if m.get('method')=='textDocument/definition':send({'id':m['id'],'result':[]})
"#).unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut client = Client::configured(
            &executable,
            root.path(),
            Setup {
                arguments: vec!["--stdio".into()],
                initialization: json!({"mode":"python"}),
                configuration: json!({"python":{"analysis":{"useLibraryCodeForTypes":true}}}),
                status_notifications: false,
            },
        )
        .unwrap();
        let path = root.path().join("app.py");
        client.notify("textDocument/didOpen",json!({"textDocument":{"uri":uri(&path).unwrap(),"languageId":"python","version":7,"text":"value=1"}})).unwrap();
        client.wait_document(&path, 7).unwrap();
        assert!(
            client
                .definitions(
                    &root.path().join("app.py"),
                    Position {
                        line: 0,
                        character: 0
                    }
                )
                .unwrap()
                .is_empty()
        );
        assert_eq!(client.version, "python-peer");
    }
}
