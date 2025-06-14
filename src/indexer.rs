use crate::parser::Token;

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
}

pub fn extract_symbols(ast: &crate::parser::Ast) -> Vec<Symbol> {
    ast.0
        .iter()
        .filter_map(|tok| match tok {
            Token::Ident => Some(Symbol {
                name: "identifier".to_string(),
            }),
            _ => None,
        })
        .collect()
}
