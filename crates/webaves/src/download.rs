//! URL and PathBuf tools related to downloading files.

use std::path::{Path, PathBuf};

use regex::Regex;
use url::Url;

/// Creates a safe `PathBuf` from a URL.
pub fn url_to_path_buf(url: &Url) -> PathBuf {
    let mut path = PathBuf::new();

    path.push(sanitize_component(normalize_scheme(url.scheme())));

    if let Some(host) = url.host_str() {
        match url.port() {
            Some(port) => path.push(sanitize_component(&format!("{host},{port}"))),
            None => path.push(sanitize_component(host)),
        }
    }

    if let Some(segments) = url.path_segments() {
        for segment in segments {
            if !segment.is_empty() {
                path.push(sanitize_component(segment))
            }
        }
    }

    if let Some(query) = url.query() {
        path.push(sanitize_component(query))
    }

    if path.components().count() == 1 {
        let other = url
            .as_str()
            .split_once(':')
            .unwrap_or_else(|| ("", url.as_str()))
            .1;
        path.push(sanitize_component(other));
    }

    path
}

fn normalize_scheme(scheme: &str) -> &str {
    match scheme {
        "https" => "http",
        "wss" => "ws",
        _ => scheme,
    }
}

fn sanitize_component(part: &str) -> String {
    let hash = mx3::v3::hash(part.as_bytes(), 1);
    let is_dots = part.chars().all(|c| c == '.');

    let mut part = part.replace(
        |c: char| is_dots || c.is_control() || "<>:\"/\\|?*".contains(c),
        "_",
    );

    // https://devblogs.microsoft.com/oldnewthing/20031022-00/?p=42073
    lazy_static::lazy_static! {
        static ref DOS_DEVICES: Regex = Regex::new(r"^(con|prn|aux|nul|com[1-9]|lpt[0-9])(\.[^.]+)?$").unwrap();
    }

    if DOS_DEVICES.is_match(&part) {
        match part.find('.') {
            Some(index) => part.insert(index, '_'),
            None => part.push('_'),
        }
    }

    if part.ends_with(|c: char| " .".contains(c)) {
        part.pop();
        part.push('_');
    }

    if part.len() > 200 {
        while part.len() > 200 {
            part.pop();
        }
        part.push_str(&format!("_{:016x}", hash));
    }

    if part.is_empty() {
        part.push('_')
    }

    part
}

/// Modifies a path to include numbering when conflicting with existing files.
pub fn remove_path_conflict<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut components = path.as_ref().components().peekable();
    let mut new_path = PathBuf::new();

    while let Some(component) = components.next() {
        let is_last = components.peek().is_none();

        if is_last {
            new_path.push(component);
            let mut count = 1u64;

            while new_path.exists() {
                new_path.pop();
                new_path.push(format!(
                    "{}_{}",
                    component.as_os_str().to_string_lossy(),
                    count
                ));

                count += 1;
            }
        } else {
            new_path.push(component);
            let mut count = 1u64;

            while new_path.is_file() {
                new_path.pop();
                new_path.push(format!(
                    "{}_{}",
                    component.as_os_str().to_string_lossy(),
                    count
                ));

                count += 1;
            }
        }
    }

    new_path
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;

    use super::*;

    #[test]
    fn test_sanitize_component() {
        assert_eq!(sanitize_component(""), "_");
        assert_eq!(sanitize_component("."), "_");
        assert_eq!(sanitize_component(".."), "__");
        assert_eq!(sanitize_component("..."), "___");
        assert_eq!(sanitize_component("\x00"), "_");
        assert_eq!(sanitize_component("\x7f"), "_");
        assert_eq!(sanitize_component("\"* /: <> ?\\ |"), "__ __ __ __ _");
        assert_eq!(sanitize_component("file "), "file_");
        assert_eq!(sanitize_component("file."), "file_");
        assert_eq!(sanitize_component("nul"), "nul_");
        assert_eq!(sanitize_component("nul.txt"), "nul_.txt");
        assert_eq!(sanitize_component("nul.abc.txt"), "nul.abc.txt");
        assert_eq!(
            sanitize_component(&"😀".repeat(200)),
            format!(
                "{}_{:016x}",
                "😀".repeat(50),
                mx3::v3::hash("😀".repeat(200).as_bytes(), 1)
            )
        );
    }

    #[test]
    fn test_url_to_path() {
        let url = Url::parse("http://example.com/").unwrap();
        assert_eq!(url_to_path_buf(&url), PathBuf::from("http/example.com"));

        let url = Url::parse("https://example.com:8080/a/b/c.html").unwrap();
        assert_eq!(
            url_to_path_buf(&url),
            PathBuf::from("http/example.com,8080/a/b/c.html")
        );

        let url = Url::parse("http://|.com/123:456/").unwrap();
        assert_eq!(url_to_path_buf(&url), PathBuf::from("http/_.com/123_456"));

        let url = Url::parse("other:abc").unwrap();
        assert_eq!(url_to_path_buf(&url), PathBuf::from("other/abc"));

        let url = Url::parse("other:../abc").unwrap();
        assert_eq!(url_to_path_buf(&url), PathBuf::from("other/.._abc"));
    }

    fn test_remove_path_conflict_impl(
        input_path: &str,
        output_path: &str,
        dirs: &[&str],
        files: &[&str],
    ) {
        let temp_dir = TempDir::new("webaves-test-").unwrap();

        for dir in dirs {
            let path = temp_dir.path().join(dir);
            std::fs::create_dir_all(path).unwrap();
        }

        for file in files {
            let path = temp_dir.path().join(file);
            std::fs::write(path, b"").unwrap();
        }

        assert_eq!(
            remove_path_conflict(temp_dir.path().join(input_path)),
            temp_dir.path().join(output_path)
        );
    }

    #[test]
    fn test_remove_path_conflict() {
        test_remove_path_conflict_impl("a.txt", "a.txt", &[], &[]);
        test_remove_path_conflict_impl("a/b.txt", "a/b.txt", &["a"], &[]);
        test_remove_path_conflict_impl("a.txt", "a.txt_1", &[], &["a.txt"]);
        test_remove_path_conflict_impl("a.txt/b.txt", "a.txt_1/b.txt", &[], &["a.txt"]);
    }
}
