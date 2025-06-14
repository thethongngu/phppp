use bumpalo::Bump;
use std::{fs, process::Command};
use tower_lsp::lsp_types::Url;
use which::which;

use phppp::{indexer, parser};

#[test]
fn calculator_example() {
    if which("php").is_err() {
        eprintln!("skipping calculator_example: php not found");
        return;
    }
    let path = fs::canonicalize("examples/calculator.php").unwrap();
    let text = fs::read_to_string(&path).unwrap();
    let bump = Bump::new();
    let ast = parser::parse_php(&text, &bump);
    assert!(!ast.0.root_node().has_error(), "Parse error");

    let uri = Url::from_file_path(&path).unwrap();
    let symbols = indexer::extract_symbols(&text, &ast, &uri);
    assert!(symbols.contains_key("App\\add"), "function add not indexed");
    assert!(
        symbols.contains_key("App\\Circle"),
        "class Circle not indexed"
    );
    assert!(symbols.contains_key("App\\PI"), "constant PI not indexed");
    assert!(
        symbols.contains_key("App\\$circle"),
        "variable $circle not indexed"
    );

    let output = Command::new("php")
        .arg(&path)
        .output()
        .expect("failed to run php");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "5");
}
