//! The release part of a version, which a library's file names carry (CLI-13, ADR-0045).

/// Returns the release of `version`, as PEP 440 reads it: the numbers and dots it starts
/// with, after an optional `v`, so `5.0` of `5.0`, `v5.0` and `5.0rc1`, and `5` of `5.`; or
/// `None` when it starts with no number.
#[must_use]
pub fn version_release(version: &str) -> Option<&str> {
    let text = version.strip_prefix(['v', 'V']).unwrap_or(version);
    let mut end = 0;
    let bytes = text.as_bytes();
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        let start = end;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        debug_assert!(end > start);
        // A dot belongs to the release only when a number follows it.
        if end + 1 < bytes.len() && bytes[end] == b'.' && bytes[end + 1].is_ascii_digit() {
            end += 1;
        } else {
            break;
        }
    }
    (end > 0).then(|| &text[..end])
}

#[cfg(test)]
mod tests {
    use super::version_release;

    #[test]
    fn the_release_is_the_numbers_and_dots_a_version_starts_with() {
        for (version, release) in [
            ("5.0", Some("5.0")),
            ("5", Some("5")),
            ("2024.11.17", Some("2024.11.17")),
            ("v5.0", Some("5.0")),
            ("V1.2.3", Some("1.2.3")),
            ("5.0rc1", Some("5.0")),
            ("5.0.dev0", Some("5.0")),
            ("1.0.post2", Some("1.0")),
            ("5.", Some("5")),
            ("5..1", Some("5")),
            ("10.20+local", Some("10.20")),
            ("", None),
            ("v", None),
            ("abc", None),
            (".5", None),
            ("1!2.0", Some("1")),
        ] {
            assert_eq!(version_release(version), release, "{version:?}");
        }
    }

    #[test]
    fn a_release_holds_only_digits_and_dots_between_numbers() {
        for version in ["0", "1.2.3.4.5", "9a", "3.x", "77.0.0-beta"] {
            let release = version_release(version).unwrap();
            assert!(
                release
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || byte == b'.')
            );
            assert!(!release.ends_with('.') && !release.starts_with('.'));
            assert!(!release.contains(".."));
        }
    }
}
