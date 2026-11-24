//! Local Gradle distributions selected without running the Wrapper.
use super::inputs::{InputFiles, absolute};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use std::{
    collections::HashMap,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};
use vizzi_core::model::Snapshot;

const WRAPPER: &str = "gradle/wrapper/gradle-wrapper.properties";
const URI_PATH: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~')
    .remove(b'!')
    .remove(b'$')
    .remove(b'&')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')')
    .remove(b'*')
    .remove(b'+')
    .remove(b',')
    .remove(b';')
    .remove(b'=')
    .remove(b':')
    .remove(b'@')
    .remove(b'/');
const URI_TAIL: &AsciiSet = &URI_PATH.remove(b'?').remove(b'[').remove(b']');

pub(super) struct Installation {
    pub home: PathBuf,
    pub user_home: PathBuf,
    pub java_home: Option<PathBuf>,
}

impl Installation {
    pub(super) fn fingerprint(&self, hash: &mut blake3::Hasher) {
        for path in [
            Some(self.home.as_path()),
            Some(self.user_home.as_path()),
            self.java_home.as_deref(),
        ] {
            if let Some(path) = path {
                let bytes = path.as_os_str().as_encoded_bytes();
                hash.update(&(bytes.len() as u64).to_le_bytes());
                hash.update(bytes);
            } else {
                hash.update(&0u64.to_le_bytes());
            }
        }
    }
}

pub(super) fn installation(root: &Path, snapshot: &Snapshot) -> Result<Installation, String> {
    let captured = |path| -> Result<Option<HashMap<String, String>>, String> {
        snapshot
            .file_by_path(path)
            .map(|doc| {
                let bytes = doc
                    .captured()
                    .read_all(2 * 1024 * 1024)
                    .map_err(|e| e.to_string())?;
                properties(bytes.bytes())
            })
            .transpose()
    };
    let user_home = std::env::var_os("GRADLE_USER_HOME")
        .map(PathBuf::from)
        .or_else(|| directories::BaseDirs::new().map(|dirs| dirs.home_dir().join(".gradle")))
        .ok_or("Gradle user home is unavailable")?;
    let user_home = if user_home.is_absolute() {
        user_home
    } else {
        absolute(
            &std::env::current_dir().map_err(|e| e.to_string())?,
            &user_home,
        )
    };
    let home = if let Some(wrapper) = captured(WRAPPER)? {
        cached_wrapper(root, &user_home, &wrapper)?
    } else {
        local_gradle_home(
            std::env::var_os("GRADLE_HOME").as_deref(),
            std::env::var_os("PATH").as_deref(),
        )?
    };
    let mut files = InputFiles::default();
    let user = files
        .read(&user_home.join("gradle.properties"))?
        .map(|bytes| properties(&bytes))
        .transpose()?;
    let project = captured("gradle.properties")?;
    let install = files
        .read(&home.join("gradle.properties"))?
        .map(|bytes| properties(&bytes))
        .transpose()?;
    let java_home = [user.as_ref(), project.as_ref(), install.as_ref()]
        .into_iter()
        .flatten()
        .find_map(|settings| settings.get("org.gradle.java.home"))
        .map(|value| gradle_java_home(root, value))
        .transpose()?;
    Ok(Installation {
        home,
        user_home,
        java_home,
    })
}

fn gradle_java_home(root: &Path, value: &str) -> Result<PathBuf, String> {
    let path = absolute(root, Path::new(value))
        .canonicalize()
        .map_err(|_| "Configured Gradle Java home is unavailable")?;
    let executable = if cfg!(windows) { "java.exe" } else { "java" };
    if !path.join("bin").join(executable).is_file() {
        return Err("Configured Gradle Java home needs an installed Java runtime".into());
    }
    Ok(path)
}

fn properties(mut bytes: &[u8]) -> Result<HashMap<String, String>, String> {
    let mut properties = Vec::with_capacity(bytes.len());
    let mut continuing = false;
    while !bytes.is_empty() {
        let end = bytes
            .iter()
            .position(|byte| matches!(byte, b'\r' | b'\n'))
            .unwrap_or(bytes.len());
        let (line, rest) = bytes.split_at(end);
        bytes = rest
            .strip_prefix(b"\r\n")
            .unwrap_or_else(|| rest.get(1..).unwrap_or_default());
        // Java skips comment lines before interpreting escapes in properties.
        let first = line
            .iter()
            .find(|byte| !matches!(byte, b' ' | b'\t' | 0x0c));
        if !continuing && matches!(first, Some(b'#' | b'!')) {
            continue;
        }
        properties.extend_from_slice(line);
        if !rest.is_empty() {
            properties.push(b'\n');
        }
        continuing = line.iter().rev().take_while(|&&byte| byte == b'\\').count() % 2 == 1;
    }
    java_properties::read(properties.as_slice()).map_err(|_| "Invalid Gradle properties".into())
}

fn cached_wrapper(
    root: &Path,
    user_home: &Path,
    props: &HashMap<String, String>,
) -> Result<PathBuf, String> {
    let url = props
        .get("distributionUrl")
        .ok_or("Gradle Wrapper distributionUrl is missing")?;
    let (url, name) = wrapper_url(root, url)?;
    let directory = name
        .rsplit_once('.')
        .map_or(name.as_str(), |(stem, _)| stem);
    if !matches!(
        Path::new(directory).components().next(),
        Some(std::path::Component::Normal(_))
    ) {
        return Err("Gradle Wrapper distribution must name a file".into());
    }
    let key = wrapper_key(url.as_bytes());
    let cache_path = |base: &str, path: &str| {
        let base = match props
            .get(base)
            .map(String::as_str)
            .unwrap_or("GRADLE_USER_HOME")
        {
            "GRADLE_USER_HOME" => user_home,
            "PROJECT" => root,
            _ => return Err("Invalid Gradle Wrapper cache base"),
        };
        let path = Path::new(
            props
                .get(path)
                .map(String::as_str)
                .unwrap_or("wrapper/dists"),
        );
        if !path.is_relative() {
            return Err("Gradle Wrapper cache path must be relative to its base");
        }
        Ok(base.join(path).join(directory).join(&key))
    };
    let extracted = cache_path("distributionBase", "distributionPath")?;
    let marker = cache_path("zipStoreBase", "zipStorePath")?.join(format!("{name}.ok"));
    let unavailable = "The pinned Gradle Wrapper distribution is not cached. Provision the Wrapper distribution and import again";
    if !marker.is_file() {
        return Err(unavailable.into());
    }
    let mut directories = fs::read_dir(extracted)
        .map_err(|_| unavailable)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir());
    let home = directories.next().ok_or(unavailable)?;
    if directories.next().is_some() {
        return Err("Gradle Wrapper cache contains multiple extracted distributions".into());
    }
    valid_gradle_home(&home).ok_or_else(|| unavailable.into())
}

fn wrapper_url(root: &Path, value: &str) -> Result<(String, String), String> {
    let invalid = "Invalid Gradle Wrapper distribution URL";
    let decode = |part| percent_decode_str(part).decode_utf8().map_err(|_| invalid);
    let mut absolute;
    let value = if matches!(
        url::Url::parse(value),
        Err(url::ParseError::RelativeUrlWithoutBase)
    ) {
        // Gradle resolves these as Java File paths and retains literal dot segments in the key.
        let relative = decode(value.split_once('#').map_or(value, |(path, _)| path))?;
        let file = root
            .join("gradle/wrapper")
            .join(relative.trim_start_matches('/'));
        let path = file
            .to_str()
            .ok_or(invalid)?
            .replace(std::path::MAIN_SEPARATOR, "/");
        absolute = String::from("file:");
        for component in path.split('/').filter(|component| !component.is_empty()) {
            absolute.push('/');
            absolute.extend(utf8_percent_encode(component, URI_PATH));
        }
        &absolute
    } else {
        value
    };
    let parsed = url::Url::parse(value).map_err(|_| invalid)?;
    let (scheme, tail) = value.split_once(':').ok_or(invalid)?;
    if !matches!(parsed.scheme(), "http" | "https" | "file") {
        return Err(invalid.into());
    }
    let (authority, tail) = if let Some(rest) = tail.strip_prefix("//") {
        let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let (authority, tail) = rest.split_at(end);
        (authority.rsplit('@').next().unwrap_or_default(), tail)
    } else {
        ("", tail)
    };
    if !authority.is_ascii() || (authority.is_empty() && parsed.scheme() != "file") {
        return Err(invalid.into());
    }
    let (tail, fragment) = tail
        .split_once('#')
        .map_or((tail, None), |(a, b)| (a, Some(b)));
    let (path, query) = tail
        .split_once('?')
        .map_or((tail, None), |(a, b)| (a, Some(b)));
    let path = decode(path)?;
    let name = path
        .rsplit('/')
        .next()
        .filter(|name| !name.contains(['\\', '\0']))
        .ok_or(invalid)?
        .to_owned();
    let mut safe = format!("{scheme}:");
    if !authority.is_empty() {
        safe.push_str("//");
        if let Some((host, port)) = authority
            .rsplit_once(':')
            .filter(|(_, port)| !port.contains(']'))
        {
            safe.push_str(host);
            if !port.is_empty() {
                let port = port.parse::<u16>().map_err(|_| invalid)?;
                safe.push_str(&format!(":{port}"));
            }
        } else {
            safe.push_str(authority);
        }
    }
    safe.push_str(&utf8_percent_encode(&path, URI_PATH).to_string());
    for (delimiter, value) in [('?', query), ('#', fragment)] {
        if let Some(value) = value {
            safe.push(delimiter);
            safe.push_str(&utf8_percent_encode(&decode(value)?, URI_TAIL).to_string());
        }
    }
    Ok((safe, name))
}

fn wrapper_key(url: &[u8]) -> String {
    let mut value = u128::from_be_bytes(md5::compute(url).0);
    let digits = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut reversed = Vec::new();
    loop {
        reversed.push(char::from(digits[(value % 36) as usize]));
        value /= 36;
        if value == 0 {
            return reversed.into_iter().rev().collect();
        }
    }
}

fn valid_gradle_home(home: &Path) -> Option<PathBuf> {
    let launcher = if cfg!(windows) {
        "gradle.bat"
    } else {
        "gradle"
    };
    let home = home.canonicalize().ok()?;
    if !home.join("bin").join(launcher).is_file() {
        return None;
    }
    fs::read_dir(home.join("lib"))
        .ok()?
        .flatten()
        .any(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with("gradle-launcher-") && name.ends_with(".jar"))
                && entry.path().is_file()
        })
        .then_some(home)
}

pub(super) fn local_gradle_home(
    configured: Option<&OsStr>,
    path: Option<&OsStr>,
) -> Result<PathBuf, String> {
    let launcher = if cfg!(windows) {
        "gradle.bat"
    } else {
        "gradle"
    };
    if let Some(configured) = configured {
        return valid_gradle_home(Path::new(configured))
            .ok_or_else(|| "GRADLE_HOME is not a local Gradle distribution".into());
    }
    path.into_iter()
        .flat_map(std::env::split_paths)
        .find_map(|directory| {
            let executable = directory.join(launcher).canonicalize().ok()?;
            valid_gradle_home(executable.parent()?.parent()?)
        })
        .ok_or_else(|| {
            "Install Gradle locally and set GRADLE_HOME or add its bin directory to PATH".into()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn properties_ignore_comments_and_preserve_continued_values() {
        let values = properties(b" \t# Example: C:\\users\\dev\\jdk\\\r\n! \\uTEXT\rcontinued=some\\\r\n  #value\norg.gradle.java.home=C:\\\\jdk\\\\jre\n").unwrap();
        assert_eq!(values["continued"], "some#value");
        assert_eq!(values["org.gradle.java.home"], r"C:\jdk\jre");
    }

    #[test]
    fn gradle_accepts_a_runtime_home_without_a_compiler() {
        let root = tempfile::tempdir().unwrap();
        let jre = root.path().join("jre");
        fs::create_dir_all(jre.join("bin")).unwrap();
        assert!(gradle_java_home(root.path(), "jre").is_err());
        let executable = if cfg!(windows) { "java.exe" } else { "java" };
        fs::write(jre.join("bin").join(executable), "").unwrap();
        assert_eq!(
            gradle_java_home(root.path(), "jre").unwrap(),
            jre.canonicalize().unwrap()
        );
    }

    #[test]
    fn wrapper_uses_exact_distribution_url_and_the_configured_zip_store_marker() {
        let root = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        let config = properties(b"distributionUrl=https\\://services.gradle.org/distributions/gradle-8.10.2-bin.zip\nzipStoreBase=PROJECT\nzipStorePath=downloads\n").unwrap();
        let key = "gradle-8.10.2-bin/a04bxjujx95o3nb99gddekhwo";
        let dist = user
            .path()
            .join("wrapper/dists")
            .join(key)
            .join("gradle-8.10.2");
        fs::create_dir_all(dist.join("bin")).unwrap();
        fs::create_dir_all(dist.join("lib")).unwrap();
        fs::write(
            dist.join("bin").join(if cfg!(windows) {
                "gradle.bat"
            } else {
                "gradle"
            }),
            "",
        )
        .unwrap();
        fs::write(dist.join("lib/gradle-launcher-8.10.2.jar"), "").unwrap();
        assert!(cached_wrapper(root.path(), user.path(), &config).is_err());
        let markers = root.path().join("downloads").join(key);
        fs::create_dir_all(&markers).unwrap();
        fs::write(markers.join("gradle-8.10.2-bin.zip.ok"), "").unwrap();
        assert_eq!(
            cached_wrapper(root.path(), user.path(), &config).unwrap(),
            dist.canonicalize().unwrap()
        );

        let mut other_url = config;
        other_url.insert(
            "distributionUrl".into(),
            "https://downloads.gradle.org/distributions/gradle-8.10.2-bin.zip".into(),
        );
        assert!(cached_wrapper(root.path(), user.path(), &other_url).is_err());
    }

    #[test]
    fn file_url_uses_gradles_cache_key() {
        let (url, name) = wrapper_url(Path::new("/project"), "file:///cache/gradle.zip").unwrap();
        assert_eq!(name, "gradle.zip");
        assert_eq!(wrapper_key(url.as_bytes()), "dc2xfcd0tosvpxq8axr79ysnc");
    }

    #[test]
    #[cfg(unix)]
    fn relative_wrapper_distribution_uses_the_properties_directory() {
        let (url, name) =
            wrapper_url(Path::new("/project"), "../dists//./gradle%2520x.zip").unwrap();
        assert_eq!(
            url,
            "file:/project/gradle/wrapper/../dists/./gradle%2520x.zip"
        );
        assert_eq!(name, "gradle%20x.zip");
    }

    #[test]
    fn wrapper_url_matches_gradles_credential_free_url() {
        let (url, name) = wrapper_url(
            Path::new("/project"),
            "https://a:b@DOMAIN:00443/dists/gradle%20x.zip",
        )
        .unwrap();
        assert_eq!(url, "https://DOMAIN:443/dists/gradle%20x.zip");
        assert_eq!(name, "gradle x.zip");
    }
}
