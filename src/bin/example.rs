use std::env;
use std::fs;

use bumpalo::Bump;
use phppp::{indexer, parser};

fn main() {
    let path = env::args().nth(1).expect("Missing PHP file");
    let text = fs::read_to_string(&path).expect("Failed to read file");
    let bump = Bump::new();
    let ast = parser::parse_php(&text, &bump);
    let symbols = indexer::extract_symbols(&ast);
    println!("AST: {}", ast.0.root_node().to_sexp());
    println!("Symbols: {:?}", symbols);
}
