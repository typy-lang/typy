//! Tests of the instances a function holds in its frame, through the public API (MEM-03,
//! ADR-0042): each way an instance can outlive its function keeps it on the heap.

use camino::Utf8PathBuf;
use typy_check::TypeChecker;
use typy_source::{Project, ProjectSettings};
use typy_tyir::print;

fn text(source: &str) -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let directory = std::env::temp_dir().join(format!(
        "typy-frames-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.clone()).unwrap();
    std::fs::write(root.join("app.py"), source).unwrap();
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root))
        .expect("the program parses");
    let program = TypeChecker::check(&project)
        .unwrap_or_else(|diagnostics| panic!("rejected: {diagnostics:?}\n{source}"));
    let module = typy_lower::lower(&program)
        .unwrap_or_else(|diagnostics| panic!("{diagnostics:?}\n{source}"));
    let _ = std::fs::remove_dir_all(directory);
    print(&module)
}

/// Returns, for each function of `source` in order, its name and whether it holds a new
/// instance in its frame.
fn held_in_frame(source: &str) -> Vec<(String, bool)> {
    let all = text(source);
    let mut functions = Vec::new();
    for line in all.lines() {
        if line.starts_with("fn ") {
            let name = line.rsplit("# ").next().unwrap_or_default().to_owned();
            functions.push((name, false));
        } else if line.contains(" in frame")
            && let Some(last) = functions.last_mut()
        {
            last.1 = true;
        }
    }
    functions
}

const CLASSES: &str = "\
class Point:
    def __init__(self, x: int, label: str) -> None:
        self.x = x
        self.label = label

    def doubled(self) -> int:
        return self.x * 2


class Holder:
    def __init__(self) -> None:
        self.point: Point | None = None


kept: list[Point] = []
last: Point | None = None


def read(point: Point) -> int:
    return point.x + len(point.label)


def keep(point: Point) -> None:
    kept.append(point)


def pass_on(point: Point) -> None:
    keep(point)

";

fn frames_of(functions: &str) -> Vec<(String, bool)> {
    held_in_frame(&format!("{CLASSES}\n{functions}"))
        .into_iter()
        .filter(|(name, _)| name.starts_with("case_"))
        .collect()
}

#[test]
fn an_instance_used_only_within_its_function_is_held_in_its_frame() {
    let functions = "\
def case_local(n: int) -> int:
    point = Point(n, \"a\")
    return point.x


def case_method(n: int) -> int:
    return Point(n, \"b\").doubled()


def case_argument(n: int) -> int:
    return read(Point(n, \"c\"))


def case_loop(n: int) -> int:
    total = 0
    previous = Point(0, \"start\")
    for index in range(n):
        current = Point(index, \"d\")
        total += current.x + previous.x
        previous = current
    return total


def case_attribute_assigned(n: int) -> str:
    point = Point(n, \"e\")
    point.label = point.label + \"!\"
    return point.label


def case_narrowed(n: int) -> int:
    value: Point | None = Point(n, \"f\") if n > 0 else None
    if value is not None and isinstance(value, Point):
        return value.x
    return 0
";
    for (name, held) in frames_of(functions) {
        assert!(held, "{name} holds its instance on the heap");
    }
}

#[test]
fn an_instance_that_may_outlive_its_function_comes_from_the_heap() {
    let functions = "\
def case_returned(n: int) -> Point:
    return Point(n, \"a\")


def case_returned_through_a_variable(n: int) -> Point | None:
    point = Point(n, \"b\")
    other = point
    return other


def case_appended(n: int) -> None:
    kept.append(Point(n, \"c\"))


def case_in_an_attribute(n: int) -> Holder:
    holder = Holder()
    holder.point = Point(n, \"d\")
    return holder


def case_module_variable(n: int) -> None:
    global last
    last = Point(n, \"e\")


def case_kept_by_a_callee(n: int) -> None:
    keep(Point(n, \"f\"))


def case_kept_by_a_callee_of_a_callee(n: int) -> None:
    pass_on(Point(n, \"g\"))


def case_printed(n: int) -> None:
    print(Point(n, \"h\"))


def case_in_a_tuple(n: int) -> tuple[Point, int]:
    return (Point(n, \"i\"), n)


def case_raised_with(n: int) -> None:
    point = Point(n, \"j\")
    raise ValueError(point)
";
    let found = frames_of(functions);
    assert_eq!(found.len(), 10, "{found:?}");
    for (name, held) in found {
        assert!(!held, "{name} holds an instance that escapes in its frame");
    }
}

#[test]
fn an_exception_instance_comes_from_the_heap() {
    let source = "\
class Failure(Exception):
    def __init__(self, code: int) -> None:
        super().__init__(code)
        self.code = code


def case_exception(n: int) -> int:
    failure = Failure(n)
    return failure.code
";
    let found = held_in_frame(source);
    assert!(
        found.iter().all(|(_, held)| !held),
        "an exception is held in a frame: {found:?}"
    );
}
