//! Tests of `syntax::relocate`.

use crate::{PythonFloor, SourceFile, parse};
use camino::Utf8PathBuf;
use typy_diagnostics::{FileId, Span, TextOffset};

#[test]
fn every_node_moves_to_the_place() {
    let text = "import a.b as c\nimport d\nfrom e import g as h, i\nglobal j\ntype T = list[int]\ndef f(self, other: int = 1) -> bool:\n    if not (self.x, f'{self.y!r:>{w}}') < [o for o in other if o]:\n        return {1: 2} == {3}\n    try:\n        del items[1:2]\n    except E as error:\n        raise X from error\n    with a as b:\n        assert c, d\n    for i in r:\n        x += lambda: 1\n    while y:\n        break\n";
    let file = SourceFile::from_text(FileId(7), Utf8PathBuf::from("generated.py"), text);
    let mut tree = parse(&file, PythonFloor::default()).unwrap();
    let place = Span::empty(FileId(3), TextOffset(5));
    for statement in &mut tree.body {
        statement.relocate(place);
    }
    let dumped = format!("{tree:?}");
    assert!(!dumped.contains("FileId(7)"), "{dumped}");
}
