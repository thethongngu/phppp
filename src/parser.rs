use bumpalo::Bump;
use logos::Logos;

#[derive(Debug, Logos, PartialEq)]
pub enum Token {
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Ident,

    #[regex(r"[0-9]+")]
    Number,

    #[token("<?php")]
    PhpStart,

    #[token("?>")]
    PhpEnd,

    #[regex(r"\s+", logos::skip)]
    Whitespace,
}

#[derive(Debug)]
pub struct Ast(pub Vec<Token>);

pub fn parse_php(input: &str, _bump: &Bump) -> Ast {
    let lexer = Token::lexer(input);
    Ast(lexer.filter_map(Result::ok).collect())
}
