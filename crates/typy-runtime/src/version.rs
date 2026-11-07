//! The runtime's version, kept in the archive, so `typy` refuses an archive from another build.

const TEXT: &str = concat!("typy-runtime-version:", env!("CARGO_PKG_VERSION"), "\n");

const fn text_bytes<const LENGTH: usize>() -> [u8; LENGTH] {
    let mut bytes = [0; LENGTH];
    let mut index = 0;
    while index < LENGTH {
        bytes[index] = TEXT.as_bytes()[index];
        index += 1;
    }
    bytes
}

/// `typy-runtime-version:`, the package version and a line feed, as bytes in the archive; a
/// program's link drops it, because nothing refers to it.
#[used]
#[unsafe(no_mangle)]
pub static TYPY_RUNTIME_VERSION: [u8; TEXT.len()] = text_bytes();

#[cfg(test)]
mod tests {
    use super::TYPY_RUNTIME_VERSION;

    #[test]
    fn the_marker_names_the_package_version() {
        let text = core::str::from_utf8(&TYPY_RUNTIME_VERSION).unwrap();
        assert_eq!(
            text,
            format!("typy-runtime-version:{}\n", env!("CARGO_PKG_VERSION"))
        );
    }
}
