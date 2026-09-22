pub mod expr;
pub mod stmt;

pub use expr::{BinaryOp, Expr, UnaryOp};
pub use stmt::{Attribute, ExternFnDecl, ExternParam, ImportSymbol, InterfaceMethod, Stmt};

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}
