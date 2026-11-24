//! Binding JDT class-file locations to exact local archive members.
use crate::dependency_sources::{FILE_BYTES, Input, matching_archive_member};
use percent_encoding::percent_decode_str;
use std::path::{Path, PathBuf};
use url::Url;
use vizzi_core::model::ContentId;

pub(crate) fn is_source_archive(path: &Path) -> bool {
    path.is_absolute()
        && matches!(
            path.extension().and_then(|ext| ext.to_str()),
            Some("jar" | "zip")
        )
}

pub(crate) fn input(
    uri: &Url,
    source: &Path,
    binary: &Path,
    text: &str,
    configuration: &str,
) -> Result<Input, String> {
    let (library, package, file) = destination(uri).ok_or("Invalid JDT class source URI")?;
    if !is_source_archive(source) || text.is_empty() || text.len() > FILE_BYTES {
        return Err("Java source archive or document is outside its allowance".into());
    }
    let binary_name = binary
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("JDT LS did not identify the library")?;
    let modular_jdk = binary_name == "jrt-fs.jar";
    let mut member = PathBuf::new();
    if modular_jdk {
        member.push(&library);
    }
    if !package.is_empty() {
        member.push(package.replace('.', "/"));
    }
    member.push(file);
    let member = member
        .to_str()
        .ok_or("Archive entry is not UTF-8")?
        .replace('\\', "/");
    let root = source.canonicalize().map_err(|error| error.to_string())?;
    let path = matching_archive_member(&root, &member, text.as_bytes())?;
    Ok(Input {
        root,
        path,
        language: "java".into(),
        package: if modular_jdk {
            library
        } else {
            binary_name.into()
        },
        version: None,
        configuration: configuration.into(),
        mirror_path: None,
        installation: None,
        archive: Some(ContentId::derive(&[text.as_bytes()])),
    })
}

pub(crate) fn destination(uri: &url::Url) -> Option<(String, String, String)> {
    if uri.scheme() != "jdt"
        || uri.host_str() != Some("contents")
        || uri.as_str().len() > 16 * 1024
        || !uri.username().is_empty()
        || uri.password().is_some()
        || uri.port().is_some()
        || uri.fragment().is_some()
    {
        return None;
    }
    let mut segments = uri.path_segments()?;
    let decode = |part| {
        let value = percent_decode_str(part).decode_utf8().ok()?;
        (!value.contains(['/', '\\', ':', '\0']) && !matches!(value.as_ref(), "." | ".."))
            .then(|| value.into_owned())
    };
    let library = decode(segments.next()?)?;
    let next = decode(segments.next()?)?;
    let (package, file) = match segments.next() {
        Some(file) => (next, decode(file)?),
        None => (String::new(), next),
    };
    if library.is_empty()
        || (!package.is_empty() && package.split('.').any(str::is_empty))
        || segments.next().is_some()
    {
        return None;
    }
    let file = if file.ends_with(".java") {
        file
    } else {
        format!("{}.java", file.strip_suffix(".class")?.split('$').next()?)
    };
    Some((library, package, file))
}
