use bumpalo::Bump;
use tower_lsp::lsp_types::Url;

use phppp::{indexer, parser};

#[test]
fn extract_top_level_symbols() {
    let src = r#"<?php
namespace Foo;
function bar() {}
class Baz {}
const MYCONST = 1;
$var = 2;
"#;
    let bump = Bump::new();
    let ast = parser::parse_php(src, &bump);
    let uri = Url::parse("file:///test.php").unwrap();
    let symbols = indexer::extract_symbols(src, &ast, &uri);
    assert!(symbols.contains_key("Foo\\bar"));
    assert!(symbols.contains_key("Foo\\Baz"));
    assert!(symbols.contains_key("Foo\\MYCONST"));
    assert!(symbols.contains_key("Foo\\$var"));
}
