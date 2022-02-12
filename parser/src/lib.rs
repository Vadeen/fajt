extern crate fajt_lexer;
extern crate serde;

#[macro_use]
pub mod error;
mod binary_expr;
mod binding;
mod class;
mod cover;
mod expr;
mod function;
mod iteration;
mod literal;
mod member_access;
mod method;
mod module;
mod static_semantics;
mod stmt;
mod variable;

use crate::error::{Error, Result};
use crate::static_semantics::StaticSemantics;
use fajt_ast::{
    Expr, Ident, LitString, Literal, Program, PropertyName, SourceType, Span, Stmt, StmtList,
};
use fajt_common::io::{PeekRead, PeekReader, ReReadWithState};
use fajt_lexer::token::{KeywordContext, Token, TokenValue};
use fajt_lexer::{punct, Lexer};
use fajt_lexer::{token_matches, LexerState};
use std::cell::Cell;
use std::rc::Rc;

/// Similar trait to bool.then, but handles closures returning `Result`.
pub trait ThenTry {
    fn then_try<T, F>(self, f: F) -> Result<Option<T>>
    where
        F: FnOnce() -> Result<T>;
}

impl ThenTry for bool {
    fn then_try<T, F>(self, f: F) -> Result<Option<T>>
    where
        F: FnOnce() -> Result<T>,
    {
        if self {
            f().map(Some)
        } else {
            Ok(None)
        }
    }
}

pub fn parse_program(input: &str) -> Result<Program> {
    parse::<Program>(input, SourceType::Unknown)
}

pub fn parse<T>(input: &str, source_type: SourceType) -> Result<T>
where
    T: Parse,
{
    let lexer = Lexer::new(input).unwrap();
    let mut reader = fajt_common::io::PeekReader::new(lexer).unwrap();
    Parser::parse::<T>(&mut reader, source_type)
}

#[derive(Clone, Default)]
pub struct Context {
    is_await: bool,
    is_yield: bool,
    is_in: bool,
    is_strict: bool,
    is_default: bool,
}

macro_rules! modifier {
    ($fn_name:ident:$field_name:ident) => {
        pub fn $fn_name(&self, $field_name: bool) -> Self {
            Context {
                $field_name,
                ..(*self)
            }
        }
    };
}

impl Context {
    modifier!(with_await: is_await);
    modifier!(with_yield: is_yield);
    modifier!(with_in: is_in);
    modifier!(with_strict: is_strict);
    modifier!(with_default: is_default);

    fn keyword_context(&self) -> KeywordContext {
        let mut keyword_context = KeywordContext::empty();
        if self.is_await {
            keyword_context |= KeywordContext::AWAIT;
        }

        if self.is_yield {
            keyword_context |= KeywordContext::YIELD;
        }

        if self.is_strict {
            keyword_context |= KeywordContext::STRICT;
        }

        keyword_context
    }
}

pub trait Parse: Sized {
    fn parse<I>(parser: &mut Parser<I>) -> Result<Self>
    where
        I: PeekRead<Token, Error = fajt_lexer::error::Error>,
        I: ReReadWithState<Token, State = LexerState, Error = fajt_lexer::error::Error>;
}

impl Parse for Expr {
    fn parse<I>(parser: &mut Parser<I>) -> Result<Self>
    where
        I: PeekRead<Token, Error = fajt_lexer::error::Error>,
        I: ReReadWithState<Token, State = LexerState, Error = fajt_lexer::error::Error>,
    {
        parser
            .with_context(Context::default().with_in(true))
            .parse_expr()
    }
}

impl Parse for Stmt {
    fn parse<I>(parser: &mut Parser<I>) -> Result<Self>
    where
        I: PeekRead<Token, Error = fajt_lexer::error::Error>,
        I: ReReadWithState<Token, State = LexerState, Error = fajt_lexer::error::Error>,
    {
        parser.parse_stmt()
    }
}

impl Parse for Program {
    fn parse<I>(parser: &mut Parser<I>) -> Result<Self>
    where
        I: PeekRead<Token, Error = fajt_lexer::error::Error>,
        I: ReReadWithState<Token, State = LexerState, Error = fajt_lexer::error::Error>,
    {
        let span_start = parser.position();

        let directives = parser.parse_directive_prologue()?;
        let strict_mode = directives.iter().any(|s| s.value == "use strict");

        let body = if strict_mode {
            parser
                .with_context(parser.context.with_strict(true))
                .parse_all_stmts()?
        } else {
            parser.parse_all_stmts()?
        };

        let span = parser.span_from(span_start);
        let stmt_list = StmtList {
            span,
            directives,
            body,
        };

        Ok(Program::new(parser.source_type.get(), stmt_list))
    }
}

pub struct Parser<'a, I>
where
    I: PeekRead<Token, Error = fajt_lexer::error::Error>,
{
    context: Context,
    semantics: StaticSemantics,
    reader: &'a mut PeekReader<Token, I>,
    source_type: Rc<Cell<SourceType>>,
}

impl<'a, I> Parser<'a, I>
where
    I: PeekRead<Token, Error = fajt_lexer::error::Error>,
    I: ReReadWithState<Token, State = LexerState, Error = fajt_lexer::error::Error>,
{
    pub fn new(reader: &'a mut PeekReader<Token, I>, source_type: SourceType) -> Result<Self> {
        Ok(Parser {
            context: Context::default(),
            semantics: StaticSemantics::with_context(Context::default()),
            reader,
            source_type: Rc::new(Cell::new(source_type)),
        })
    }

    pub fn parse<T>(reader: &'a mut PeekReader<Token, I>, source_type: SourceType) -> Result<T>
    where
        T: Parse,
    {
        let mut parser = Parser::new(reader, source_type)?;
        T::parse(&mut parser)
    }

    fn source_type(&self) -> SourceType {
        self.source_type.get()
    }

    fn set_source_type(&mut self, source_type: SourceType) {
        self.source_type.replace(source_type);
    }

    fn current(&self) -> Result<&Token> {
        Ok(self.reader.current()?)
    }

    fn consume(&mut self) -> Result<Token> {
        Ok(self.reader.consume()?)
    }

    fn peek(&self) -> Option<&Token> {
        self.reader.peek()
    }

    fn is_end(&self) -> bool {
        self.reader.is_end()
    }

    fn position(&self) -> usize {
        self.current()
            .map(|t| t.span.start)
            .unwrap_or_else(|_| self.reader.position())
    }

    fn span_from(&self, start: usize) -> Span {
        Span::new(start, self.reader.position())
    }

    pub fn with_context(&mut self, context: Context) -> Parser<'_, I> {
        Parser {
            context: context.clone(),
            semantics: StaticSemantics::with_context(context),
            reader: self.reader,
            source_type: self.source_type.clone(),
        }
    }

    fn current_matches(&self, value: &TokenValue) -> bool {
        if let Ok(token) = self.current() {
            &token.value == value
        } else {
            false
        }
    }

    fn current_matches_string_literal(&self) -> bool {
        matches!(
            self.current(),
            Ok(Token {
                value: TokenValue::Literal(Literal::String(_)),
                ..
            })
        )
    }

    fn peek_matches(&self, value: &TokenValue) -> bool {
        if let Some(token) = self.peek() {
            &token.value == value
        } else {
            false
        }
    }

    fn consume_assert(&mut self, expected: &'static TokenValue) -> Result<Token> {
        let token = self.consume()?;
        if &token.value != expected {
            return Err(Error::expected_other_token(token, expected));
        }
        Ok(token)
    }

    fn maybe_consume(&mut self, value: &TokenValue) -> Result<bool> {
        if self.current_matches(value) {
            self.consume()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn followed_by_new_lined(&self) -> bool {
        self.peek().map_or(false, |t| t.first_on_line)
    }

    fn peek_is_identifier(&self) -> bool {
        is_identifier(self.peek(), self.context.keyword_context())
    }

    fn is_identifier(&self) -> bool {
        is_identifier(self.current().ok(), self.context.keyword_context())
    }

    fn parse_identifier(&mut self) -> Result<Ident> {
        let token = self.consume()?;
        Ok(match token.value {
            TokenValue::Identifier(s) => Ident::new(s, token.span),
            TokenValue::Keyword(keyword) => {
                if keyword.is_allowed_as_identifier(self.context.keyword_context()) {
                    Ident::new(keyword.to_string(), token.span)
                } else {
                    return Err(Error::forbidden_identifier(keyword.to_string(), token.span));
                }
            }
            _ => return Err(Error::expected_ident(token)),
        })
    }

    fn parse_optional_identifier(&mut self) -> Result<Option<Ident>> {
        Ok(if self.is_identifier() {
            Some(self.parse_identifier()?)
        } else {
            None
        })
    }

    /// Parses the `PropertyName` production.
    fn parse_property_name(&mut self) -> Result<PropertyName> {
        match self.current()? {
            token_matches!(@literal) => {
                let token = self.consume()?;
                match token.value {
                    TokenValue::Literal(Literal::String(string)) => {
                        Ok(PropertyName::String(string))
                    }
                    TokenValue::Literal(Literal::Number(number)) => {
                        Ok(PropertyName::Number(number))
                    }
                    _ => Err(Error::unexpected_token(token)),
                }
            }
            token_matches!(punct!("[")) => {
                self.consume()?;
                let expr = self.parse_assignment_expr()?;
                self.consume_assert(&punct!("]"))?;
                Ok(PropertyName::Computed(expr.into()))
            }
            _ if self.is_identifier() => Ok(PropertyName::Ident(self.parse_identifier()?)),
            _ => Err(Error::unexpected_token(self.consume()?)),
        }
    }

    fn parse_directive_prologue(&mut self) -> Result<Vec<LitString>> {
        let mut directives = Vec::new();

        loop {
            if self.current_matches_string_literal() {
                let stmt = self.parse_stmt()?;
                let string = stmt
                    .unwrap_expr_stmt()
                    .expr
                    .unwrap_literal()
                    .literal
                    .unwrap_string();
                directives.push(string);
            } else {
                break;
            }
        }

        Ok(directives)
    }

    fn consume_array_delimiter(&mut self) -> Result<()> {
        self.consume_list_delimiter(&punct!("]"))
    }

    fn consume_object_delimiter(&mut self) -> Result<()> {
        self.consume_list_delimiter(&punct!("}"))
    }

    fn consume_parameter_delimiter(&mut self) -> Result<()> {
        self.consume_list_delimiter(&punct!(")"))
    }

    fn consume_list_delimiter(&mut self, list_end: &TokenValue) -> Result<()> {
        if !self.maybe_consume(&punct!(","))? && !self.current_matches(list_end) {
            let token = self.consume()?;
            return Err(Error::expected_other_token(token, &punct!(",")));
        }

        Ok(())
    }
}

fn is_identifier(token: Option<&Token>, keyword_context: KeywordContext) -> bool {
    match token {
        Some(Token {
            value: TokenValue::Identifier(_),
            ..
        }) => true,
        Some(Token {
            value: TokenValue::Keyword(keyword),
            ..
        }) => keyword.is_allowed_as_identifier(keyword_context),
        _ => false,
    }
}
