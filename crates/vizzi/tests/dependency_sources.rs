//! Captured external source is independent of repository filenames and mutable disk bytes.
use vizzi::dependency_sources::{Catalog, Input};
fn input(root: &std::path::Path) -> Input {
    Input {
        root: root.canonicalize().unwrap(),
        path: "src/lib.rs".into(),
        language: "rust".into(),
        package: "helper".into(),
        version: Some("1.2.3".into()),
        configuration: "a".repeat(64),
    }
}

#[test]
fn provider_function_ranges_preserve_current_rust_syntax_and_restore_exact_source() {
    use vizzi::dependency_sources::Declaration;
    use vizzi_core::model::ContentId;
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("src")).unwrap();
    let text = "impl<T> Option<T> { pub const fn ok_or_else<E, F>(self, err: F) -> Result<T, E> where F: [const] FnOnce() -> E + [const] Destruct { match self { Some(v) => Ok(v), None => Err(err()), } } }";
    std::fs::write(root.path().join("src/lib.rs"), text).unwrap();
    let catalog = Catalog::new(cache.path().join("refs.json"));
    let start = text.find("ok_or_else").unwrap();
    let descriptor = Declaration {
        content: ContentId::derive(&[text.as_bytes()]),
        name: "ok_or_else".into(),
        span: text.find("pub const fn").unwrap()..text.rfind('}').unwrap() - 1,
        identifier: start..start + "ok_or_else".len(),
    };
    let raw = catalog.capture(input(root.path())).unwrap();
    assert!(raw.index.definition(raw.doc().path(), start).is_none());
    let captured = catalog
        .capture_with_declaration(input(root.path()), descriptor.clone())
        .unwrap();
    let target = captured
        .index
        .definition(captured.doc().path(), start)
        .unwrap();
    assert_eq!(target.name, "ok_or_else");
    assert_eq!(captured.doc().resident_source(), Some(text));
    assert!(captured.calls(&target.id, 0).unwrap().items.is_empty());
    let restored = Catalog::new(cache.path().join("refs.json"))
        .get(&captured.origin.capture)
        .unwrap()
        .unwrap();
    assert_eq!(restored.doc().version(), captured.doc().version());
    for bad in [
        Declaration {
            content: ContentId::derive(&[b"different"]),
            ..descriptor.clone()
        },
        Declaration {
            identifier: start..text.len() + 1,
            ..descriptor.clone()
        },
        Declaration {
            name: "unrelated".into(),
            ..descriptor
        },
    ] {
        assert!(
            catalog
                .capture_with_declaration(input(root.path()), bad)
                .is_err()
        );
    }
}

#[test]
fn provider_function_names_have_bounded_single_line_metadata() {
    use vizzi::dependency_sources::Declaration;
    use vizzi_core::model::ContentId;
    for name in ["n".repeat(257), "name\n".into(), "name\0".into()] {
        let root = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("src")).unwrap();
        let text = format!("pub fn {name}() {{}}\n");
        std::fs::write(root.path().join("src/lib.rs"), &text).unwrap();
        let declaration = Declaration {
            content: ContentId::derive(&[text.as_bytes()]),
            name: name.clone(),
            span: 0..text.len(),
            identifier: 7..7 + name.len(),
        };
        assert!(
            Catalog::new(cache.path().join("refs.json"))
                .capture_with_declaration(input(root.path()), declaration)
                .is_err()
        );
    }
}
#[cfg(unix)]
#[test]
fn dependency_roots_require_canonical_native_identity() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("src")).unwrap();
    std::fs::write(root.path().join("src/lib.rs"), "fn entry() {}").unwrap();
    let alias = cache.path().join("alias");
    std::os::unix::fs::symlink(root.path(), &alias).unwrap();
    let catalog = Catalog::new(cache.path().join("refs.json"));
    let native = input(root.path());
    assert!(catalog.capture(native.clone()).is_ok());
    let mut aliased = native;
    aliased.root = alias;
    assert_eq!(
        catalog.capture(aliased).err().as_deref(),
        Some("Dependency root must be canonical")
    );
}
#[test]
fn dependency_captures_are_distinct_frozen_and_restorable_by_native_references() {
    let root = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    for dir in [root.path(), other.path()] {
        std::fs::create_dir(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("src/lib.rs"),
            "pub fn start(){ leaf(); } fn leaf(){}\n",
        )
        .unwrap();
    }
    let catalog = Catalog::new(cache.path().join("refs.json"));
    let first = catalog.capture(input(root.path())).unwrap();
    let second = catalog.capture(input(other.path())).unwrap();
    assert_ne!(first.origin.id, second.origin.id);
    assert_ne!(first.doc().id(), second.doc().id());
    let symbol = first
        .index
        .functions("", "start", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let symbol = first.symbol(symbol);
    assert_eq!(symbol.origin.as_ref().unwrap().label, "helper");
    assert_eq!(
        symbol.origin.as_ref().unwrap().version.as_deref(),
        Some("1.2.3")
    );
    let restored = Catalog::new(cache.path().join("refs.json"))
        .get(&first.origin.capture)
        .unwrap()
        .unwrap();
    assert_eq!(restored.doc().version(), first.doc().version());
    std::fs::write(root.path().join("src/lib.rs"), "fn replacement(){}\n").unwrap();
    assert!(first.verify().is_err());
    assert!(first.doc().resident_source().unwrap().contains("leaf"));
    assert!(
        Catalog::new(cache.path().join("refs.json"))
            .get(&first.origin.capture)
            .unwrap()
            .is_none()
    );
}
#[test]
fn dependency_paths_languages_and_unknown_references_cannot_select_arbitrary_source() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let catalog = Catalog::new(cache.path().join("refs.json"));
    for path in [
        "../outside.rs",
        "/absolute.rs",
        "src/../lib.rs",
        "src\\lib.rs",
    ] {
        let mut i = input(root.path());
        i.path = path.into();
        assert!(catalog.capture(i).is_err());
    }
    let mut i = input(root.path());
    i.language = "unknown".into();
    assert!(catalog.capture(i).is_err());
    assert!(catalog.get("../../outside").is_err());
    assert!(catalog.get(&"0".repeat(64)).unwrap().is_none());
}
#[test]
fn dependency_configuration_version_and_retention_are_separate_from_relative_filenames() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("src")).unwrap();
    std::fs::write(root.path().join("src/lib.rs"), "fn kept(){}").unwrap();
    let catalog = Catalog::new(cache.path().join("refs.json"));
    let first = catalog.capture(input(root.path())).unwrap();
    assert!(first.verify().is_ok());
    let mut next = input(root.path());
    next.configuration = "b".repeat(64);
    let next = catalog.capture(next).unwrap();
    assert_eq!(first.doc().id(), next.doc().id());
    assert_ne!(first.origin.capture, next.origin.capture);
    let mut version = input(root.path());
    version.version = Some("2.0.0".into());
    let version = catalog.capture(version).unwrap();
    assert_ne!(first.origin.id, version.origin.id);
    for n in 0..33 {
        let path = format!("src/f{n}.rs");
        std::fs::write(root.path().join(&path), format!("fn f{n}(){{}}")).unwrap();
        let mut i = input(root.path());
        i.path = path;
        catalog.capture(i).unwrap();
    }
    assert_eq!(catalog.sources().unwrap().len(), 32);
    assert!(catalog.get(&first.origin.capture).unwrap().is_none());
    assert!(first.doc().resident_source().unwrap().contains("kept"));
    let restored = Catalog::new(cache.path().join("refs.json"));
    assert_eq!(restored.sources().unwrap().len(), 32);
}
#[test]
fn dependency_admission_checks_live_files_labels_versions_and_byte_allowances() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("src")).unwrap();
    std::fs::write(root.path().join("src/lib.rs"), "fn valid(){}").unwrap();
    let catalog = Catalog::new(cache.path().join("refs.json"));
    for (name, version, config) in [
        ("".into(), None, "a".repeat(64)),
        ("x".repeat(257), None, "a".repeat(64)),
        ("bad\nname".into(), None, "a".repeat(64)),
        ("valid".into(), Some("x".repeat(129)), "a".repeat(64)),
        ("valid".into(), None, "G".repeat(64)),
    ] {
        let mut i = input(root.path());
        i.package = name;
        i.version = version;
        i.configuration = config;
        assert!(catalog.capture(i).is_err());
    }
    std::fs::write(
        root.path().join("src/lib.rs"),
        format!("/*{}*/", " ".repeat(2 * 1024 * 1024)),
    )
    .unwrap();
    assert!(catalog.capture(input(root.path())).is_err());
    std::fs::write(cache.path().join("refs.json"), "not JSON").unwrap();
    assert!(
        Catalog::new(cache.path().join("refs.json"))
            .sources()
            .is_err()
    );
}
#[test]
fn dependency_large_files_fit_the_boundary_and_total_retention_stays_bounded() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let catalog = Catalog::new(cache.path().join("refs.json"));
    let body = format!(
        "/*{}*/\nfn retained(){{}}\n",
        " ".repeat(2 * 1024 * 1024 - "/**/\nfn retained(){}\n".len())
    );
    assert_eq!(body.len(), 2 * 1024 * 1024);
    let mut first = None;
    for n in 0..17 {
        let path = format!("large{n}.rs");
        std::fs::write(root.path().join(&path), &body).unwrap();
        let mut i = input(root.path());
        i.path = path;
        let source = catalog.capture(i).unwrap();
        if first.is_none() {
            first = Some(source.origin.capture.clone());
        }
    }
    let sources = catalog.sources().unwrap();
    assert_eq!(sources.len(), 16);
    assert!(catalog.get(first.as_deref().unwrap()).unwrap().is_none());
    assert_eq!(
        sources
            .iter()
            .map(|s| s.doc().captured().len())
            .sum::<usize>(),
        32 * 1024 * 1024
    );
}
#[test]
fn dependency_reference_files_have_explicit_size_and_io_error_boundaries() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("src")).unwrap();
    std::fs::write(root.path().join("src/lib.rs"), "fn run(){}").unwrap();
    let path = cache.path().join("refs.json");
    let catalog = Catalog::new(path.clone());
    catalog.capture(input(root.path())).unwrap();
    let mut bytes = std::fs::read(&path).unwrap();
    bytes.resize(1024 * 1024, b' ');
    std::fs::write(&path, &bytes).unwrap();
    assert_eq!(Catalog::new(path.clone()).sources().unwrap().len(), 1);
    bytes.push(b' ');
    std::fs::write(&path, &bytes).unwrap();
    assert!(Catalog::new(path).sources().is_err());
    let bad = cache.path().join("not-a-directory");
    std::fs::write(&bad, "file").unwrap();
    assert!(Catalog::new(bad.join("refs.json")).sources().is_err());
    let mut alias = input(root.path());
    alias.root = root.path().join("src/..");
    assert!(catalog.capture(alias).is_err());
    assert!(catalog.get(&"z".repeat(64)).is_err());
}
#[test]
fn dependency_declaration_allowance_accepts_its_boundary_and_evicts_to_stay_within_it() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("src")).unwrap();
    let path = root.path().join("src/lib.rs");
    let text = (0..32768)
        .map(|n| format!("fn f{n}(){{}}\n"))
        .collect::<String>();
    std::fs::write(&path, &text).unwrap();
    let catalog = Catalog::new(cache.path().join("refs.json"));
    let first = catalog.capture(input(root.path())).unwrap();
    assert_eq!(first.doc().declaration_count(), 32768);
    std::fs::write(&path, format!("{text}\nfn extra(){{}}\n")).unwrap();
    assert!(catalog.capture(input(root.path())).is_err());
    std::fs::write(root.path().join("one.rs"), "fn one(){}").unwrap();
    let mut i = input(root.path());
    i.path = "one.rs".into();
    catalog.capture(i).unwrap();
    assert_eq!(catalog.sources().unwrap().len(), 1);
    assert!(catalog.get(&first.origin.capture).unwrap().is_none());
}
