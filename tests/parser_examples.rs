use bumpalo::Bump;
use phppp::parser;
use std::fs;

#[test]
fn parse_all_examples() {
    for entry in fs::read_dir("examples").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map(|e| e == "php").unwrap_or(false) {
            let text = fs::read_to_string(&path).unwrap();
            let bump = Bump::new();
            let ast = parser::parse_php(&text, &bump);
            assert!(
                !ast.0.root_node().has_error(),
                "{} failed to parse",
                path.display()
            );
        }
    }
}
