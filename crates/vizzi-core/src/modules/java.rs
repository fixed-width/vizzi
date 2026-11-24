//! Package cues from captured Java declarations and module descriptors.

use super::{
    MANIFEST_BYTES, Module, ModuleEvidence, ModuleIndex, ModuleKind, ModuleRule, SOURCE_BYTES,
    TOTAL_BYTES, parent,
};
use crate::model::{ContentId, Coverage, Document, Snapshot, WorkspaceId};
use std::{borrow::Cow, collections::BTreeMap, sync::Arc};
use tree_sitter::{Node, Parser};

const DESCRIPTOR: &str = "module-info.java";
const LABEL_BYTES: usize = 512;

pub(super) const RULE: ModuleRule = ModuleRule {
    languages: &["java"],
    kind: ModuleKind {
        language: "java",
        basis: "java_package",
        tooltip: "Java package",
    },
    fact: |index, doc, _, _| index.java.get(doc.path()).cloned(),
    history: Some(history),
};

pub(super) type Cache = BTreeMap<String, (ContentId, Option<String>)>;

fn qualified_name(mut node: Node<'_>, text: &str) -> Option<String> {
    let mut parts = Vec::new();
    let mut bytes_left = LABEL_BYTES;
    while node.kind() == "scoped_identifier" {
        let name = node.child_by_field_name("name")?;
        let part = text.get(name.byte_range())?;
        bytes_left = bytes_left.checked_sub(part.len() + 1)?;
        parts.push(part);
        node = node.child_by_field_name("scope")?;
    }
    if node.kind() != "identifier" {
        return None;
    }
    let head = text.get(node.byte_range())?;
    if head.len() > bytes_left {
        return None;
    }
    parts.push(head);
    parts.reverse();
    Some(parts.join("."))
}

fn lexical_input(text: &str) -> Option<Cow<'_, str>> {
    if !text.contains(['\\', '\r']) {
        return Some(Cow::Borrowed(text));
    }
    const BACKSLASH: u16 = b'\\' as u16;
    const MARKER: u16 = b'u' as u16;
    const CR: u16 = b'\r' as u16;
    const LF: u16 = b'\n' as u16;
    let mut raw = text.encode_utf16().peekable();
    let mut translated = Vec::with_capacity(text.len());
    let mut previous_escape = false;
    let mut odd_backslashes = false;
    let mut previous_cr = false;
    while let Some(mut unit) = raw.next() {
        // Escape eligibility uses the translated backslash run and the origin of its last unit.
        let escaped = unit == BACKSLASH
            && (previous_escape || !odd_backslashes)
            && raw.next_if_eq(&MARKER).is_some();
        if escaped {
            while raw.next_if_eq(&MARKER).is_some() {}
            unit = 0;
            for _ in 0..4 {
                let digit = char::from_u32(u32::from(raw.next()?))?.to_digit(16)?;
                unit = (unit << 4) | digit as u16;
            }
        }
        previous_escape = escaped;
        odd_backslashes = unit == BACKSLASH && !odd_backslashes;
        if !(previous_cr && unit == LF) {
            translated.push(if unit == CR { LF } else { unit });
        }
        previous_cr = unit == CR;
    }
    Some(Cow::Owned(String::from_utf16_lossy(&translated)))
}

fn module_declaration(root: Node<'_>) -> Option<Node<'_>> {
    if root.has_error() {
        return None;
    }
    let mut cursor = root.walk();
    let mut declarations = root
        .children(&mut cursor)
        .filter(|child| !matches!(child.kind(), "line_comment" | "block_comment"));
    let declaration = declarations.find(|child| child.kind() != "import_declaration")?;
    (declaration.kind() == "module_declaration" && declarations.next().is_none())
        .then_some(declaration)
}

fn declared_name(text: &str, descriptor: bool) -> Option<String> {
    let text = lexical_input(text)?;
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .ok()?;
    let tree = parser.parse(text.as_ref(), None)?;
    let root = tree.root_node();
    if descriptor {
        let name = module_declaration(root)?.child_by_field_name("name")?;
        return qualified_name(name, &text);
    }
    let mut cursor = root.walk();
    let mut declarations = root
        .named_children(&mut cursor)
        .filter(|child| !child.is_extra());
    let declaration = declarations.next()?;
    if declaration.has_error()
        || declaration.kind() != "package_declaration"
        || declarations
            .any(|child| matches!(child.kind(), "package_declaration" | "module_declaration"))
    {
        return None;
    }
    let name = declaration
        .named_children(&mut declaration.walk())
        .find(|child| matches!(child.kind(), "identifier" | "scoped_identifier"))?;
    qualified_name(name, &text)
}

fn is_descriptor(path: &str) -> bool {
    path.rsplit('/').next() == Some(DESCRIPTOR)
}

fn name(doc: &Document, cache: &mut Cache, budget: &mut usize) -> Result<Option<String>, ()> {
    if doc.classification().assigned_language() != Some(RULE.kind.language) {
        return Ok(None);
    }
    let descriptor = is_descriptor(doc.path());
    let max_bytes = if descriptor {
        MANIFEST_BYTES
    } else {
        SOURCE_BYTES
    };
    let Some(text) = doc.resident_source().filter(|text| text.len() <= max_bytes) else {
        return Ok(None);
    };
    *budget = budget.checked_sub(text.len()).ok_or(())?;
    if let Some((content, name)) = cache.get(doc.path())
        && content == doc.content()
    {
        return Ok(name.clone());
    }
    let name = declared_name(text, descriptor);
    cache.insert(doc.path().into(), (doc.content().clone(), name.clone()));
    Ok(name)
}

fn descriptor_path(directory: &str, package: &str) -> Option<String> {
    let package_directory = package.replace('.', "/");
    let prefix = directory.strip_suffix(&package_directory)?;
    if prefix.is_empty() {
        Some(DESCRIPTOR.into())
    } else {
        Some(format!("{}/{}", prefix.strip_suffix('/')?, DESCRIPTOR))
    }
}

fn package_binding(
    workspace: &WorkspaceId,
    path: &str,
    package: &str,
    module_name: impl FnOnce(&str) -> Result<Option<String>, ()>,
) -> Option<Module> {
    let directory = parent(path);
    let descriptor = if let Some(path) = descriptor_path(directory, package) {
        module_name(&path).ok()?.map(|module| (module, path))
    } else {
        None
    };
    let (root, label) = descriptor.map_or_else(
        || {
            (
                if directory.is_empty() { "." } else { directory }.into(),
                package.into(),
            )
        },
        |(module, path)| (path, format!("{module}/{package}")),
    );
    if label.len() > LABEL_BYTES || root.len() > 4096 {
        return None;
    }
    let workspace = workspace.as_str();
    let basis = RULE.kind.basis;
    let id = blake3::hash(format!("{workspace}\0{basis}\0{root}\0{package}").as_bytes());
    Some(RULE.kind.describe(id.to_hex().to_string(), label, root))
}

pub(super) fn bindings<'a>(
    snapshot: &Snapshot,
    documents: impl Iterator<Item = &'a Document>,
    cache: &mut Cache,
) -> BTreeMap<String, Module> {
    let mut budget = TOTAL_BYTES;
    let mut descriptors = BTreeMap::new();
    documents
        .filter(|doc| {
            !is_descriptor(doc.path())
                && doc.analyses().iter().any(|analysis| {
                    analysis.context.language == RULE.kind.language
                        && analysis.coverage == Coverage::Syntax
                        && !analysis.facts.is_empty()
                })
        })
        .filter_map(|doc| {
            let package = name(doc, cache, &mut budget).ok()??;
            let binding = package_binding(snapshot.workspace(), doc.path(), &package, |path| {
                descriptors
                    .entry(path.to_owned())
                    .or_insert_with(|| {
                        snapshot
                            .file_by_path(path)
                            .map_or(Ok(None), |descriptor| name(descriptor, cache, &mut budget))
                    })
                    .clone()
            })?;
            Some((doc.path().into(), binding))
        })
        .collect()
}

fn history(
    workspace: &WorkspaceId,
    selected: &Arc<Document>,
    evidence: &dyn ModuleEvidence,
) -> Option<ModuleIndex> {
    let mut cache = Cache::new();
    let mut budget = TOTAL_BYTES;
    let package = name(selected, &mut cache, &mut budget).ok()??;
    let binding = package_binding(workspace, selected.path(), &package, |path| {
        evidence
            .captured_file(path, MANIFEST_BYTES)
            .map_or(Ok(None), |descriptor| {
                name(&descriptor, &mut cache, &mut budget)
            })
    })?;
    let mut index = ModuleIndex::empty(workspace);
    index.java.insert(selected.path().into(), binding);
    Some(index)
}
