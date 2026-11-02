# Raw C/C++ declaration reference

The built-in registry includes `CAdapter` (`c`) and `CppAdapter` (`cpp`) from
`vizzi_core::c_family`. They implement the byte-only `Analyzer` interface with safe
Tree-sitter wrappers. They perform no filesystem reads, preprocessing, compiler
invocation, or project execution.

Source policy still selects inputs. C-family scopes are opt-in; default discovery
remains Rust/Python. `.c` selects C and recognized C++ suffixes select C++ within enabled
scopes. Ambiguous `.h`/`.H` files require explicit assignments. CUDA and unassigned
fragments remain source-only. An explicit assignment can select C++ for a `.c` file.
The [policy reference](SOURCE_POLICY.md) defines precedence.

## Extracted facts

| Input form | Fact kind | Extraction rule |
| --- | --- | --- |
| Function definition with a body | `function` | Named declarator, including supported pointer/reference wrappers |
| Named struct/union/enum definition | `struct`, `union`, `enum` | Requires a body; references and forward declarations are excluded |
| `typedef` declaration | `typedef` | Every declarator, including arrays and function-pointer aliases |
| Named C++ class definition | `class` | Requires a body |
| Named C++ namespace definition | `namespace` | Literal namespace spelling, including `outer::inner` |
| C++ `using Name = Type` | `alias` | The named type alias |

C++ function extraction includes constructors, destructors, methods, operators, and
conversion operators with bodies. Out-of-line names retain literal qualification,
such as `Box<int>::read` or `Item::operator bool`. Parameters, cv/ref qualifiers, and
return types are not part of a function's lexical name. Pointer-returning C functions
are distinguished from function-pointer variables, which are outside this initial
outline capability.

Templates are traversed without inventing declarations for template parameters or
wrappers. The underlying class/function/type alias is extracted. There is no template
instantiation, overload resolution, or unification of different qualified spellings.
Variables, fields, prototypes, bodyless defaulted/deleted methods, and standalone
anonymous types/namespaces have no outline facts. Typedef aliases for anonymous types
are supported. These extraction limits are independent of grammar acceptance.

## Spans, scopes, and identity

Facts retain exclusive-end byte spans in captured source. All names in a multi-alias
typedef share its complete statement span. Other facts use their definition node's
span; surrounding template wrappers are not added. Line coordinates derive from
captured bytes, including UTF-8 names.

Named lexical ancestors qualify identities. Type aliases are not scopes: in
`typedef struct Tag { ... } Alias;`, the struct is not qualified by `Alias`. Anonymous
blocks/types do not introduce named qualification. Literal out-of-line spelling is
retained rather than compiler-resolved. Repeated qualified names, including overloads
and conditional alternatives, use conservative ambiguous identities. Position-only
edits preserve unique identities; ambiguous selections fall back to files.

The shared traversal accepts multiple declarations per node through a reusable buffer
and a separate lexical-scope callback. Rust/Python use the same traversal; cursor
walking, recovery, and duplicate-key handling are shared.

## Coverage and reuse

`syntax` means the raw tree has no grammar recovery, `partial` means recovery occurred,
and `unavailable` includes unsupported encoding. None establishes active build branches,
compiler standard/target acceptance, or complete declaration extraction. Facts can
occur in different conditional alternatives. Macros are not expanded and includes
are not followed. Views report raw syntax with unknown configuration coverage.

Contexts fingerprint language, C-family implementation, shared traversal, registry/model
source, and the lockfile. C and C++ contexts remain distinct for identical bytes. The
shared traversal/registry update conservatively invalidates prior Rust/Python cache
contexts once. Warm caches then reuse facts under the same rules. Header edits do not
invalidate unrelated raw files because no include resolution was performed. Configured
dependency invalidation remains later work.

All four languages share the worker queue. Parsers are created lazily per worker/language;
cache hits create none. Active parsing is bounded by the configured worker count,
not an independent limit for each language. This is not a memory limit.

The direct grammar releases are C **0.24.2** and C++ **0.23.4**, verified with `cargo info`
and published latest documentation when added to `vizzi-core`.
[C grammar reference](https://docs.rs/tree-sitter-c/0.24.2/tree_sitter_c/),
[C++ grammar reference](https://docs.rs/tree-sitter-cpp/0.23.4/tree_sitter_cpp/).
The workspace already used these releases for experiments; no older-release or
prerelease exception is needed. Tests qualify these extraction rules, not every
language-standard feature accepted by a compiler.
