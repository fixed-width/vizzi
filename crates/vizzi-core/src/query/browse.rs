use super::MapFilter;
use crate::{
    model::{Document, FileId, Snapshot},
    policy::is_readme,
};
use serde::Serialize;
use std::{collections::BTreeMap, sync::Arc};

const PAGE_SIZE: usize = 100;

/// Visible contents of a directory or a page of repository-wide path matches.
#[derive(Debug, Serialize)]
pub struct BrowsePage {
    /// Current relative directory, or `.` for the root.
    pub directory: String,
    /// Number of captured files in the subtree or matching the search.
    pub files: usize,
    /// Number of immediate children or search matches across all pages.
    pub entries: usize,
    /// Offset of the returned page, clamped to the last available page.
    pub offset: usize,
    /// Maximum combined number of directories and files per page.
    pub size: usize,
    /// Directory cards on this page.
    pub directories: Vec<DirectorySummary>,
    /// Recognized files directly in this directory, independent of paging.
    pub landmarks: Vec<Landmark>,
}

/// A child directory represented by all its captured descendant files.
#[derive(Debug, Serialize)]
pub struct DirectorySummary {
    /// Workspace-relative directory path.
    pub path: String,
    /// Full subtree file count.
    pub files: usize,
}

/// Recognized repository documentation or project manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LandmarkKind {
    /// Conventional README filename.
    Readme,
    /// A recognized project manifest.
    Manifest,
}

impl LandmarkKind {
    pub(super) fn for_path(path: &str) -> Option<Self> {
        let filename = path.rsplit('/').next().unwrap_or(path);
        if is_readme(filename) {
            Some(Self::Readme)
        } else if matches!(
            filename,
            "Cargo.toml"
                | "package.json"
                | "pyproject.toml"
                | "Gemfile"
                | "pom.xml"
                | "settings.gradle"
                | "settings.gradle.kts"
                | "build.gradle"
                | "build.gradle.kts"
        ) {
            Some(Self::Manifest)
        } else {
            None
        }
    }
}

/// A direct link to a captured landmark.
#[derive(Debug, Serialize)]
pub struct Landmark {
    /// Captured file identity.
    pub id: FileId,
    /// Workspace-relative file path.
    pub path: String,
    /// Reason the file appears as a landmark.
    pub kind: LandmarkKind,
}

pub(super) fn project<'a>(
    snapshot: &'a Snapshot,
    directory: &str,
    filter: &mut MapFilter<'_>,
    offset: usize,
) -> (BrowsePage, Vec<&'a Document>) {
    let (directory, docs, prefix_len) = scope(snapshot.documents(), directory);
    let mut page = BrowsePage {
        directory: directory.into(),
        files: 0,
        size: PAGE_SIZE,
        entries: 0,
        offset: 0,
        directories: vec![],
        landmarks: vec![],
    };
    if !filter.path.is_empty() {
        let matches: Vec<_> = snapshot
            .documents()
            .iter()
            .map(Arc::as_ref)
            .filter(|doc| filter.collect_type_and_match(doc))
            .collect();
        return file_page(directory, matches, offset, None);
    }

    let mut directories = BTreeMap::<&str, usize>::new();
    let mut files = Vec::new();
    let mut readme = None;
    for doc in docs
        .iter()
        .map(Arc::as_ref)
        .filter(|doc| filter.collect_type_and_match(doc))
    {
        page.files += 1;
        let path = doc.path();
        if let Some(slash) = path[prefix_len..].find('/') {
            *directories.entry(&path[..prefix_len + slash]).or_default() += 1;
        } else {
            files.push(doc);
            match LandmarkKind::for_path(path) {
                Some(LandmarkKind::Readme) => {
                    readme.get_or_insert(doc);
                }
                Some(LandmarkKind::Manifest) => {
                    page.landmarks.push(landmark(doc, LandmarkKind::Manifest))
                }
                None => {}
            }
        }
    }
    if let Some(doc) = readme {
        page.landmarks
            .insert(0, landmark(doc, LandmarkKind::Readme));
    }

    let directory_count = directories.len();
    page.entries = directory_count + files.len();
    page.offset = page_offset(offset, page.entries);
    page.directories = directories
        .into_iter()
        .skip(page.offset)
        .take(PAGE_SIZE)
        .map(|(path, files)| DirectorySummary {
            path: path.into(),
            files,
        })
        .collect();
    let files = files
        .into_iter()
        .skip(page.offset.saturating_sub(directory_count))
        .take(PAGE_SIZE - page.directories.len())
        .collect();
    (page, files)
}

fn page_offset(offset: usize, entries: usize) -> usize {
    (offset.min(entries.saturating_sub(1)) / PAGE_SIZE) * PAGE_SIZE
}

pub(super) fn file_page<'a>(
    directory: &str,
    matches: Vec<&'a Document>,
    offset: usize,
    reveal: Option<&str>,
) -> (BrowsePage, Vec<&'a Document>) {
    let count = matches.len();
    let offset = reveal
        .and_then(|id| matches.iter().position(|doc| doc.id().as_str() == id))
        .unwrap_or(offset);
    let offset = page_offset(offset, count);
    let page = BrowsePage {
        directory: directory.into(),
        files: count,
        entries: count,
        offset,
        size: PAGE_SIZE,
        directories: vec![],
        landmarks: vec![],
    };
    let files = matches.into_iter().skip(offset).take(PAGE_SIZE).collect();
    (page, files)
}

fn landmark(doc: &Document, kind: LandmarkKind) -> Landmark {
    Landmark {
        id: doc.id().clone(),
        path: doc.path().into(),
        kind,
    }
}

fn scope<'a, 'p>(
    docs: &'a [Arc<Document>],
    mut path: &'p str,
) -> (&'p str, &'a [Arc<Document>], usize) {
    while !matches!(path, "" | ".") {
        let scoped = exact_scope(docs, path);
        if !scoped.is_empty() {
            return (path, scoped, path.len() + 1);
        }
        path = path.rsplit_once('/').map_or(".", |(parent, _)| parent);
    }
    (".", docs, 0)
}

pub(super) fn exact_scope<'a>(docs: &'a [Arc<Document>], path: &str) -> &'a [Arc<Document>] {
    if matches!(path, "" | ".") {
        return docs;
    }
    let prefix = format!("{path}/");
    let start = docs.partition_point(|doc| doc.path() < prefix.as_str());
    let scoped = &docs[start..];
    let len = scoped.partition_point(|doc| doc.path().starts_with(&prefix));
    &scoped[..len]
}
