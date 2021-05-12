pub mod expression;
pub mod statement;

pub use expression::*;
pub use statement::*;

use crate::error::Error;
use crate::error::ErrorKind::SyntaxError;
use fajt_lexer::token::{Keyword, Span, Token, TokenValue};
use std::convert::TryFrom;

#[derive(Debug, PartialOrd, PartialEq)]
pub struct Body<T> {
    span: Span,
    body: Vec<T>,
}

#[derive(Debug, PartialOrd, PartialEq)]
pub enum Program {
    Script(Body<Statement>),
    Module(Body<ModuleItem>),
}

impl Program {
    pub fn from_body(body: Vec<Statement>) -> Self {
        Program::Script(Body {
            span: (0, 0).into(),
            body,
        })
    }
}

#[derive(Debug, PartialOrd, PartialEq)]
pub enum ModuleItem {
    ImportDeclaration(/* TODO */),
    ExportDeclaration(/* TODO */),
    Statement(Statement),
}

#[derive(Debug, PartialOrd, PartialEq)]
pub struct Ident {
    pub span: Span,
    pub name: String,
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

impl TryFrom<Token> for Ident {
    type Error = Error;

    fn try_from(token: Token) -> Result<Self, Self::Error> {
        match token.value {
            TokenValue::Identifier(name) => Ok(Ident {
                span: token.span.clone(),
                name,
            }),
            // Await can be used as a keyword in the parser context.
            TokenValue::Keyword(Keyword::Await) => Ok(Ident {
                name: "await".to_owned(),
                span: token.span,
            }),
            // Yield can be used as a keyword in the parser context.
            TokenValue::Keyword(Keyword::Yield) => Ok(Ident {
                name: "yield".to_owned(),
                span: token.span,
            }),
            _ => Err(Error::of(SyntaxError(
                format!("Tried to use '{:?}' as an identifier.", token.value),
                token.span,
            ))),
        }
    }
}
