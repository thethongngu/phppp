use bumpalo::Bump;
use phppp::{indexer, parser};
use tower_lsp::lsp_types::Url;

#[test]
fn large_project_stress() {
    let base = std::fs::read_to_string("examples/hello.php").unwrap();
    let sample = base.repeat(100);
    for i in 0..20 {
        let bump = Bump::new();
        let ast = parser::parse_php(&sample, &bump);
        assert!(!ast.0.root_node().has_error());
        let uri = Url::parse(&format!("file:///stress{}.php", i)).unwrap();
        let symbols = indexer::extract_symbols(&sample, &ast, &uri);
        assert!(symbols.contains_key("greet"));
    }
}
