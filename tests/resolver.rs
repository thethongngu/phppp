use bumpalo::Bump;
use tower_lsp::lsp_types::{Position, Url};

use phppp::{indexer, parser, resolver};

#[test]
fn resolve_use_alias() {
    let src_a = r#"<?php
namespace Bar;
function someFunc() {}
"#;
    let src_b = r#"<?php
namespace Foo;
use Bar\someFunc as aliasFunc;

aliasFunc();
"#;
    let bump = Bump::new();
    let ast_a = parser::parse_php(src_a, &bump);
    let uri_a = Url::parse("file:///a.php").unwrap();
    let symbols_a = indexer::extract_symbols(src_a, &ast_a, &uri_a);

    let ast_b = parser::parse_php(src_b, &bump);
    let uri_b = Url::parse("file:///b.php").unwrap();
    let symbols_b = indexer::extract_symbols(src_b, &ast_b, &uri_b);

    let index = indexer::GlobalIndex::new();
    index.insert(uri_a.clone(), symbols_a.clone());
    index.insert(uri_b.clone(), symbols_b.clone());
    let pos = Position {
        line: 4,
        character: 0,
    };
    let resolved =
        resolver::resolve_symbol("aliasFunc", &uri_b, pos, src_b, &ast_b, &symbols_b, &index)
            .expect("symbol not resolved");

    assert_eq!(resolved.name, "Bar\\someFunc");
    assert_eq!(resolved.location.uri, uri_a);
}

#[test]
fn resolve_parameter_variable() {
    let src = r#"<?php
function foo($bar) {
    echo $bar;
}
"#;
    let bump = Bump::new();
    let ast = parser::parse_php(src, &bump);
    let uri = Url::parse("file:///c.php").unwrap();
    let symbols = indexer::extract_symbols(src, &ast, &uri);
    let index = indexer::GlobalIndex::new();
    index.insert(uri.clone(), symbols.clone());

    let pos = Position {
        line: 2,
        character: 10,
    }; // inside echo $bar
    let resolved = resolver::resolve_symbol("$bar", &uri, pos, src, &ast, &symbols, &index)
        .expect("param not resolved");

    assert_eq!(resolved.kind, indexer::SymbolKind::Variable);
    assert_eq!(resolved.location.uri, uri);
}
