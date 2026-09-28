pub(super) use super::resolve_imports;
pub(super) use super::CfgContext;
pub(super) use super::Parser;
pub(super) use crate::ast::*;
pub(super) use crate::lexer::Lexer;

pub(super) fn parse_code(code: &str) -> Result<crate::ast::Program, String> {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.parse()
}

mod cfg;
mod expr;
mod imports;
mod stmt;
