use bumpalo::Bump;
use std::{fs, process::Command};
use tower_lsp::lsp_types::Url;

use phppp::{indexer, parser};

#[test]
fn hello_example() {
    let path = fs::canonicalize("examples/hello.php").unwrap();
    let text = fs::read_to_string(&path).unwrap();
    let bump = Bump::new();
    let ast = parser::parse_php(&text, &bump);
    assert!(!ast.0.root_node().has_error(), "Parse error");

    let uri = Url::from_file_path(&path).unwrap();
    let symbols = indexer::extract_symbols(&text, &ast, &uri);
    assert!(symbols.contains_key("greet"), "function greet not indexed");

    let output = Command::new("php")
        .arg(&path)
        .output()
        .expect("failed to run php");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "Hello, World!");
}
