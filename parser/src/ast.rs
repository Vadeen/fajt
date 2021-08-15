pub use binding::*;
pub use class::*;
pub(crate) use cover::*;
pub use expr::*;
pub use literal::*;
pub use stmt::*;

use fajt_lexer::token::Span;
use serde::{Deserialize, Serialize};

#[macro_use]
mod macros;

mod binding;
#[macro_use]
mod expr;
mod class;
mod cover;
mod literal;
mod stmt;

#[derive(Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct StatementList<T> {
    span: Span,
    body: Vec<T>,
}

#[derive(Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub enum Program {
    Script(StatementList<Stmt>),
    Module(StatementList<Stmt>),
}

impl Program {
    pub fn from_body(body: Vec<Stmt>) -> Self {
        let span_start = body.first().map(|s| s.span().start).unwrap_or(0);
        let span_end = body.last().map(|s| s.span().end).unwrap_or(0);
        Program::Script(StatementList {
            span: Span::new(span_start, span_end),
            body,
        })
    }

    pub fn span(&self) -> &Span {
        match self {
            Program::Script(body) => &body.span,
            Program::Module(body) => &body.span,
        }
    }
}

ast_struct! {
    pub struct Ident {
        pub span: Span,
        pub name: String,
    }
}

impl Ident {
    pub fn new<N, S>(name: N, span: S) -> Self
    where
        N: Into<String>,
        S: Into<Span>,
    {
        Ident {
            name: name.into(),
            span: span.into(),
        }
    }
}
