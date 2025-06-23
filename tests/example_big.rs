use bumpalo::Bump;
use std::{fs, process::Command};
use tower_lsp::lsp_types::Url;
use which::which;

use phppp::{indexer, parser};

#[test]
fn big_example_golden() {
    if which("php").is_err() {
        eprintln!("skipping big_example_golden: php not found");
        return;
    }
    let path = fs::canonicalize("examples/big.php").unwrap();
    let text = fs::read_to_string(&path).unwrap();
    let bump = Bump::new();
    let ast = parser::parse_php(&text, &bump);
    assert!(!ast.0.root_node().has_error(), "Parse error");

    let uri = Url::from_file_path(&path).unwrap();
    let symbols = indexer::extract_symbols(&text, &ast, &uri);

    let func_count = symbols
        .values()
        .filter(|s| matches!(s.kind, indexer::SymbolKind::Function))
        .count();
    assert_eq!(func_count, 50);
    let class_count = symbols
        .values()
        .filter(|s| matches!(s.kind, indexer::SymbolKind::Class))
        .count();
    assert_eq!(class_count, 10);
    assert!(symbols.contains_key("Big\\CONST_VAL"));
    assert!(symbols.contains_key("Big\\$sum"));
    assert!(symbols.contains_key("Big\\$obj"));
    assert!(symbols.contains_key("Big\\$result"));

    let mut names: Vec<_> = symbols.keys().cloned().collect();
    names.sort();
    let actual = names.join("\n");
    let expected = include_str!("golden/big_symbols.txt");
    assert_eq!(actual.trim(), expected.trim());

    let output = Command::new("php")
        .arg(&path)
        .output()
        .expect("failed to run php");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "127510");
}
