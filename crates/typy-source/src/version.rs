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
mod tests;
