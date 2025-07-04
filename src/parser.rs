use bumpalo::Bump;
use tree_sitter::{Parser, Tree};
use tree_sitter_php::LANGUAGE_PHP;

#[derive(Debug, Clone)]
pub struct Ast(pub Tree);

pub fn parse_php(input: &str, _bump: &Bump) -> Ast {
    tracing::debug!("Parsing PHP source ({} bytes)", input.len());
    let mut parser = Parser::new();
    parser
        .set_language(&LANGUAGE_PHP.into())
        .expect("Failed to load PHP grammar");
    let tree = parser.parse(input, None).expect("Failed to parse");
    tracing::debug!("Finished parsing");
    Ast(tree)
}
