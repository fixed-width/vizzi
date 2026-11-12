# Editor connection protocol v1

For contributors building editor plugins. This local semantic interface is implemented
first by `editors/vscode`. It is separate from LSP and Vizzi's browser interface.
[Behaviour](PRODUCT.md) defines source ownership; [implementation](TECH.md) describes
native result admission.

## Discovery and transport

The editor publishes a private UTF-8 JSON file outside the source root:

```json
{"version":1,"root":"/absolute/canonical/project","port":12345,"token":"<64 hex characters>"}
```

`port` is a nonzero TCP port on `127.0.0.1`; arbitrary hosts are not accepted. `token`
is a random 256-bit credential. The file is atomically replaced with owner-only Unix
permissions. Vizzi limits it to 4096 bytes and rereads it before each request and before
accepting a response. A change invalidates the reply. Roots must name the same canonical
directory; requests preserve the editor's path spelling and result paths map back onto
the native canonical root. This permits equivalent platform path representations.
Stopping closes the listener but retains the last endpoint file. Sharing again replaces
it; an earlier bridge never removes or rewrites a newer window's endpoint.

Each connection carries one UTF-8 JSON request ending in LF and one JSON response ending
in LF, then closes. Frames including LF cannot exceed 16 MiB. Vizzi allows two seconds
to connect and ten seconds for request/response I/O; the editor closes sockets after
twelve seconds. The editor accepts four connections and one active provider query.
A stalled provider keeps that query slot until it returns; stop/start sharing to
replace a stalled bridge. No additional provider query is silently launched.

## Requests

```json
{"version":1,"root":"/absolute/canonical/project","token":"<credential>","method":"definition","path":"src/main.py","text":"<complete captured query document>","position":{"line":2,"character":11}}
```

`method` is `definition` or `references`. `path` is normalised and root-relative,
with `/` separators, no empty/dot/parent components and no symlink traversal. Positions
use zero-based lines and UTF-16 code units, without splitting a surrogate or exceeding
a line. Query text is limited to 4 MiB. The editor compares it with the actual document
and declines dirty/disk-divergent workspace buffers or concurrent changes.
The VS Code adapter uses bounded file reads and limits open workspace inputs to 8192
files and 128 MiB in aggregate.

## Responses

```json
{"version":1,"status":"ok","session":"<session id>","epoch":3,"outside":0,"locations":[{"path":"/absolute/canonical/project/src/leaf.py","position":{"line":1,"character":4},"text":"<complete target document>"}]}
```

`session` changes on bridge restart and has 1–128 UTF-8 bytes. `epoch` is a nonnegative
integer advanced by editor document/configuration/filesystem events. It stays unchanged
through a query. Native context identity hashes session, epoch and snapshot identity
using `ContextId`; it never includes the token.

Responses have at most 256 locations and 65536 outside results. Each target document is
limited to 4 MiB, with an 8 MiB aggregate text limit in the VS Code adapter. Positions
identify exact identifiers; LSP LocationLink selection ranges are supported. References
exclude the queried declaration. The adapter removes duplicate locations.

Only repository-local captured targets are admitted. The editor counts non-file and
external locations as `outside`; Vizzi also counts local paths absent from capture.
Returned text/positions must match captured documents. Native verification checks all
captured files against disk before and after the request, with bounds of 8192 files,
4 MiB per file and 128 MiB total. Older captures never receive substituted live answers.

An unavailable query returns:

```json
{"version":1,"status":"error","message":"Save workspace buffers before using Vizzi editor analysis"}
```

The first adapter limits messages to 512 characters. Invalid input, authentication,
workspace mismatch, dirty buffers, unsupported capabilities and concurrent changes
use this outcome. Disconnects, deadlines, malformed frames and unsupported versions
are also unavailable. They never authorise a duplicate managed server.

The editor owns server lifecycles, buffers, capabilities and broader project context.
This interface cannot freeze transitive editor configuration/dependency inputs. Plugins
never save/reload buffers, send another LSP initialization or close shared servers on
Vizzi's behalf. Editing, navigation and task operations require their own contracts;
arbitrary command execution is not part of v1.
