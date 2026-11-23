//! Tests of `main_source`.

use camino::Utf8PathBuf;

use super::MainSource;
use crate::module::ModuleName;

#[test]
fn a_file_is_rooted_at_its_directory_and_the_rest_at_the_current_one() {
    let cases = [
        (MainSource::File("app.py".into()), ".", "app", "app.py"),
        (
            MainSource::File("tools/run.py".into()),
            "tools",
            "run",
            "tools/run.py",
        ),
        (
            MainSource::Module(ModuleName::new("tools.run").unwrap()),
            ".",
            "tools.run",
            "-m tools.run",
        ),
        (MainSource::Code("print(1)".to_owned()), ".", "c", "-c"),
    ];
    for (source, root, name, written) in cases {
        assert_eq!(
            (source.root(), source.program_name(), source.to_string()),
            (Utf8PathBuf::from(root), name, written.to_owned())
        );
    }
}
