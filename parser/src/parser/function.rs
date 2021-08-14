use crate::ast::{
    ArrowFunctionBody, BindingElement, Body, DeclFunction, Expr, ExprArrowFunction, ExprFunction,
    ExprLiteral, FormalParameters, Ident, Literal, Stmt, StmtExpr,
};
use crate::error::Result;
use crate::parser::ContextModify;
use crate::Parser;
use fajt_common::io::PeekRead;
use fajt_lexer::keyword;
use fajt_lexer::punct;
use fajt_lexer::token::Token;
use fajt_lexer::token_matches;
use std::mem;

impl<I> Parser<'_, I>
where
    I: PeekRead<Token, Error = fajt_lexer::error::Error>,
{
    /// Parses the `ArrowFunction` goal symbol, but expects the parameters as input since that may
    /// be a non terminal before we know if it is an arrow function or parenthesized expression.
    pub(super) fn parse_arrow_function_expr(
        &mut self,
        span_start: usize,
        binding_parameter: bool,
        parameters: FormalParameters,
    ) -> Result<Expr> {
        self.consume_assert(punct!("=>"))?;

        let body = if self.current_matches(punct!("{")) {
            ArrowFunctionBody::Body(self.parse_function_body()?)
        } else {
            ArrowFunctionBody::Expr(self.parse_assignment_expr()?.into())
        };

        let span = self.span_from(span_start);
        Ok(ExprArrowFunction {
            span,
            asynchronous: false,
            binding_parameter,
            parameters,
            body,
        }
        .into())
    }

    /// Parses the async version of `ArrowFunction` goal symbol, but expects the parameters as input
    /// since that may be a non terminal before we know if it is an arrow function or parenthesized
    /// expression.
    pub(super) fn parse_async_arrow_function_expr(
        &mut self,
        span_start: usize,
        binding_parameter: bool,
        parameters: FormalParameters,
    ) -> Result<Expr> {
        self.consume_assert(punct!("=>"))?;

        let body = if self.current_matches(punct!("{")) {
            ArrowFunctionBody::Body(
                self.with_context(ContextModify::new().set_await(true))
                    .parse_function_body()?,
            )
        } else {
            ArrowFunctionBody::Expr(
                self.with_context(ContextModify::new().set_await(true))
                    .parse_assignment_expr()?
                    .into(),
            )
        };

        let span = self.span_from(span_start);
        Ok(ExprArrowFunction {
            span,
            asynchronous: true,
            binding_parameter,
            parameters,
            body,
        }
        .into())
    }

    /// Parses the `ArrowParameters` goal symbol.
    /// Returns true in second tuple element if the parameters are a binding identifier without
    /// parentheses.
    pub(crate) fn parse_arrow_identifier_argument(&mut self) -> Result<FormalParameters> {
        let span_start = self.position();
        let identifier = self.parse_identifier()?;
        let span = self.span_from(span_start);
        Ok(FormalParameters {
            span: span.clone(),
            bindings: vec![BindingElement {
                span,
                pattern: identifier.into(),
                initializer: None,
            }],
            rest: None,
        })
    }

    /// Parses the `FunctionExpression` goal symbol.
    pub(super) fn parse_function_expr(&mut self) -> Result<Expr> {
        let span_start = self.position();
        self.consume_assert(keyword!("function"))?;

        let generator = self.maybe_consume(punct!("*"))?;
        let identifier = self.parse_optional_identifier()?;
        let parameters = self.parse_formal_parameters()?;
        let body = self.parse_function_body()?;

        let span = self.span_from(span_start);
        Ok(ExprFunction {
            span,
            asynchronous: false,
            generator,
            identifier,
            parameters,
            body,
        }
        .into())
    }

    /// Parses the `AsyncFunctionExpression` goal symbol.
    pub(super) fn parse_async_function_expr(&mut self) -> Result<Expr> {
        let span_start = self.position();
        self.consume_assert(keyword!("async"))?;

        let function_token = self.consume_assert(keyword!("function"))?;
        debug_assert!(!function_token.first_on_line);

        let generator = self.maybe_consume(punct!("*"))?;
        let identifier = self.parse_optional_identifier()?;
        let parameters = self.parse_formal_parameters()?;
        let body = self.parse_function_body()?;

        let span = self.span_from(span_start);
        Ok(ExprFunction {
            span,
            asynchronous: true,
            generator,
            identifier,
            parameters,
            body,
        }
        .into())
    }

    /// Parses the `FunctionDeclaration` goal symbol.
    pub(super) fn parse_function_declaration(&mut self) -> Result<Stmt> {
        let span_start = self.position();
        self.consume_assert(keyword!("function"))?;

        let generator = self.maybe_consume(punct!("*"))?;
        let ident = self.parse_identifier()?;

        self.with_context(ContextModify::new().set_yield(false).set_await(false))
            .parse_function_implementation(span_start, ident, generator, false)
    }

    /// Parses the `AsyncFunctionDeclaration` goal symbol.
    pub(super) fn parse_async_function_declaration(&mut self) -> Result<Stmt> {
        let span_start = self.position();
        self.consume_assert(keyword!("async"))?;

        let function_token = self.consume_assert(keyword!("function"))?;
        debug_assert!(!function_token.first_on_line);

        let generator = self.maybe_consume(punct!("*"))?;
        let ident = self.parse_identifier()?;

        self.with_context(ContextModify::new().set_yield(false).set_await(true))
            .parse_function_implementation(span_start, ident, generator, true)
    }

    /// Parses the part after the identifier of a function declaration.
    ///
    /// Example:
    /// ```no_rust
    /// function fn( a, ...args ) { return 1 };
    ///            ^~~~~~~~~~~~~~~~~~~~~~~~~~~^
    ///
    /// async function fn( a, ...args ) { return 1 };
    ///                  ^~~~~~~~~~~~~~~~~~~~~~~~~~~^
    /// ```
    pub(super) fn parse_function_implementation(
        &mut self,
        span_start: usize,
        ident: Ident,
        generator: bool,
        asynchronous: bool,
    ) -> Result<Stmt> {
        let parameters = self.parse_formal_parameters()?;
        let body = self.parse_function_body()?;

        let span = self.span_from(span_start);
        Ok(DeclFunction {
            span,
            asynchronous,
            generator,
            identifier: ident,
            parameters,
            body,
        }
        .into())
    }

    /// Parses the `FormalParameters` goal symbol.
    pub(crate) fn parse_formal_parameters(&mut self) -> Result<FormalParameters> {
        let span_start = self.position();

        self.consume_assert(punct!("("))?;

        let mut parameters = Vec::new();
        let mut rest = None;
        loop {
            match self.current()? {
                token_matches!(punct!(")")) => {
                    self.consume()?;
                    break;
                }
                token_matches!(punct!("...")) => {
                    rest = Some(self.parse_binding_rest_element()?);
                    self.consume_parameter_delimiter()?;
                }
                _ => {
                    parameters.push(self.parse_binding_element()?);
                    self.consume_parameter_delimiter()?;
                }
            }
        }

        let span = self.span_from(span_start);
        Ok(FormalParameters {
            span,
            bindings: parameters,
            rest,
        })
    }

    /// Parses the `FunctionBody` or `AsyncFunctionBody` goal symbol.
    pub(super) fn parse_function_body(&mut self) -> Result<Body> {
        let span_start = self.position();
        self.consume_assert(punct!("{"))?;

        let mut directives = Vec::new();
        let mut statements = Vec::new();
        loop {
            if self.maybe_consume(punct!("}"))? {
                break;
            }

            let mut stmt = self.parse_stmt()?;
            if !statements.is_empty() {
                statements.push(stmt);
                continue;
            }

            if let Some(string) = match_string_literal(&mut stmt) {
                directives.push(string);
            } else {
                statements.push(stmt);
            }
        }

        let span = self.span_from(span_start);
        Ok(Body {
            span,
            directives,
            statements,
        })
    }
}

fn match_string_literal(stmt: &mut Stmt) -> Option<String> {
    if let Stmt::Expr(StmtExpr {
        expr:
            Expr::Literal(ExprLiteral {
                literal: Literal::String(string, _),
                ..
            }),
        ..
    }) = stmt
    {
        Some(mem::take(string))
    } else {
        None
    }
}
