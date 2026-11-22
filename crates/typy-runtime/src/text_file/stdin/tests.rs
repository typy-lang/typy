//! Tests of standard input.

use super::{Reader, line_from, read_from};

/// Returns the code points of each line `input()` gives from `chunks`, read one chunk at
/// a time.
fn lines_of(chunks: &[&[u8]]) -> Vec<Vec<u32>> {
    let mut reader = Reader::escaping();
    let mut next = chunks.iter();
    let mut source = |buffer: &mut [u8]| {
        let Some(chunk) = next.next() else {
            return Ok(0);
        };
        buffer[..chunk.len()].copy_from_slice(chunk);
        Ok(chunk.len())
    };
    let mut lines = Vec::new();
    while let Some(line) = line_from(&mut reader, &mut source).unwrap() {
        // SAFETY: the line is live, and released once it is copied.
        unsafe {
            lines.push(
                (0..super::length(line))
                    .map(|index| super::code_point(line, index))
                    .collect(),
            );
            crate::refcount::typy_rt_decrement(line.cast());
        }
    }
    lines
}

#[test]
fn input_gives_each_line_without_its_newline_until_the_end() {
    let cases: [(&[&[u8]], &[&str]); 7] = [
        (&[], &[]),
        (&[b"one\n"], &["one"]),
        (&[b"one\n\ntwo\r\n"], &["one", "", "two"]),
        (&[b"a\rb\r", b"\nc"], &["a", "b", "c"]),
        (&[b"last"], &["last"]),
        (
            &[b"sp", b"lit\nacross", b" chunks\n"],
            &["split", "across chunks"],
        ),
        (&[b"caf\xc3", b"\xa9\n"], &["caf\u{e9}"]),
    ];
    for (chunks, expected) in cases {
        let expected: Vec<Vec<u32>> = expected
            .iter()
            .map(|line| line.chars().map(u32::from).collect())
            .collect();
        assert_eq!(lines_of(chunks), expected, "{chunks:?}");
    }
}

#[test]
fn a_byte_outside_utf8_is_escaped_as_cpython_escapes_it() {
    let lines = lines_of(&[b"a\xffb\xe9", b"\n\xe2\x82"]);
    assert_eq!(
        lines,
        [vec![0x61, 0xdcff, 0x62, 0xdce9], vec![0xdce2, 0xdc82]]
    );
}

#[test]
fn input_reads_a_line_longer_than_a_chunk() {
    let long = [b'x'; super::super::reader::CHUNK];
    let lines = lines_of(&[&long, &long[..100], b"\n"]);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].len(), super::super::reader::CHUNK + 100);
}

#[test]
fn read_gives_the_rest_or_up_to_a_limit() {
    for (limit, expected) in [(None, "one\ntwo"), (Some(5), "one\nt")] {
        let mut reader = Reader::escaping();
        let mut chunks = [&b"one\r\n"[..], b"two"].into_iter();
        let mut source = |buffer: &mut [u8]| {
            let Some(chunk) = chunks.next() else {
                return Ok(0);
            };
            buffer[..chunk.len()].copy_from_slice(chunk);
            Ok(chunk.len())
        };
        let text = read_from(&mut reader, limit, false, &mut source).unwrap();
        // SAFETY: the text is live, and released once it is read.
        unsafe {
            let found: String = (0..super::length(text))
                .filter_map(|index| char::from_u32(super::code_point(text, index)))
                .collect();
            crate::refcount::typy_rt_decrement(text.cast());
            assert_eq!(found, expected, "{limit:?}");
        }
    }
}
