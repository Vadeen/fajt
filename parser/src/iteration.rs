use crate::error::Result;
use crate::static_semantics::ExprSemantics;
use crate::{Error, Parser, ThenTry};
use fajt_ast::{
    ForBinding, ForDeclaration, ForInit, Stmt, StmtDoWhile, StmtFor, StmtForIn, StmtForOf,
    StmtVariable, StmtWhile, VariableKind,
};
use fajt_common::io::{PeekRead, ReReadWithState};
use fajt_lexer::punct;
use fajt_lexer::token::Token;
use fajt_lexer::token_matches;
use fajt_lexer::{keyword, LexerState};

impl<I> Parser<'_, I>
where
    I: PeekRead<Token, Error = fajt_lexer::error::Error>,
    I: ReReadWithState<Token, State = LexerState, Error = fajt_lexer::error::Error>,
{
    /// Parses the `DoWhileStatement` production.
    pub(super) fn parse_do_while_stmt(&mut self) -> Result<Stmt> {
        let span_start = self.position();
        self.consume_assert(&keyword!("do"))?;

        let body = self.parse_stmt()?;

        self.consume_assert(&keyword!("while"))?;
        self.consume_assert(&punct!("("))?;

        let test = self.with_context(self.context.with_in(true)).parse_expr()?;

        self.consume_assert(&punct!(")"))?;
        self.maybe_consume(&punct!(";"))?;

        let span = self.span_from(span_start);
        Ok(StmtDoWhile {
            span,
            body: Box::new(body),
            test: Box::new(test),
        }
        .into())
    }

    /// Parses the `WhileStatement` production.
    pub(super) fn parse_while_stmt(&mut self) -> Result<Stmt> {
        let span_start = self.position();
        self.consume_assert(&keyword!("while"))?;
        self.consume_assert(&punct!("("))?;

        let test = self.with_context(self.context.with_in(true)).parse_expr()?;

        self.consume_assert(&punct!(")"))?;

        let body = self.parse_stmt()?;

        let span = self.span_from(span_start);
        Ok(StmtWhile {
            span,
            test: Box::new(test),
            body: Box::new(body),
        }
        .into())
    }

    /// Parses the `ForStatement` and `ForInOfStatement` production.
    pub(super) fn parse_for_stmt(&mut self) -> Result<Stmt> {
        let span_start = self.position();
        self.consume_assert(&keyword!("for"))?;

        let asynchronous = self.context.is_await && self.maybe_consume(&keyword!("await"))?;
        self.consume_assert(&punct!("("))?;

        let start_token = self.current()?.clone();
        if let Some(stmt) = self.try_parse_for(span_start, asynchronous)? {
            return Ok(stmt);
        }

        self.reader.rewind_to(&start_token)?;

        self.parse_for_in_of(span_start, asynchronous)
    }

    /// Tries to parse the `ForStatement` production. Returns `Ok(None)` if the loop did not match
    /// the `ForStatement` production but it may be a valid `ForInOfStatement` production.
    /// Expects `for (` to already have been consumed.
    fn try_parse_for(&mut self, span_start: usize, asynchronous: bool) -> Result<Option<Stmt>> {
        let init = match self.parse_optional_for_init() {
            Ok(init) => init,
            Err(_) => return Ok(None),
        };

        if !self.maybe_consume(&punct!(";"))? {
            return Ok(None);
        }

        if asynchronous {
            let span = self.span_from(span_start);
            return Err(Error::syntax_error(
                "'for await' loops must be used with 'of'".to_owned(),
                span,
            ));
        }

        let test = (!self.current_matches(&punct!(";")))
            .then_try(|| self.with_context(self.context.with_in(true)).parse_expr())?;
        self.consume_assert(&punct!(";"))?;

        let update = (!self.current_matches(&punct!(")")))
            .then_try(|| self.with_context(self.context.with_in(true)).parse_expr())?;
        self.consume_assert(&punct!(")"))?;

        let body = self.parse_stmt()?;
        let span = self.span_from(span_start);

        Ok(Some(
            StmtFor {
                span,
                init,
                test: test.map(Box::new),
                update: update.map(Box::new),
                body: Box::new(body),
            }
            .into(),
        ))
    }

    /// Parses the `ForInOfStatement` production.
    /// Expects `for (` to already have been consumed.
    fn parse_for_in_of(&mut self, span_start: usize, asynchronous: bool) -> Result<Stmt> {
        let declaration = self.parse_for_declaration()?;

        match self.current()? {
            token_matches!(keyword!("of")) => {
                self.parse_for_of(span_start, declaration, asynchronous)
            }
            token_matches!(keyword!("in")) => {
                if asynchronous {
                    let span = self.span_from(span_start);
                    return Err(Error::syntax_error(
                        "'for await' loops must be used with 'of'".to_owned(),
                        span,
                    ));
                }

                self.parse_for_in(span_start, declaration)
            }
            _ => Err(Error::unexpected_token(self.consume()?)),
        }
    }

    fn parse_for_in(&mut self, span_start: usize, left: ForDeclaration) -> Result<Stmt> {
        self.consume_assert(&keyword!("in"))?;

        let right = self.with_context(self.context.with_in(true)).parse_expr()?;

        self.consume_assert(&punct!(")"))?;

        let body = self.parse_stmt()?;
        let span = self.span_from(span_start);
        Ok(StmtForIn {
            span,
            left,
            right: Box::new(right),
            body: Box::new(body),
        }
        .into())
    }

    fn parse_for_of(
        &mut self,
        span_start: usize,
        left: ForDeclaration,
        asynchronous: bool,
    ) -> Result<Stmt> {
        self.consume_assert(&keyword!("of"))?;

        let right = self.with_context(self.context.with_in(true)).parse_expr()?;

        self.consume_assert(&punct!(")"))?;

        let body = self.parse_stmt()?;
        let span = self.span_from(span_start);
        Ok(StmtForOf {
            span,
            left,
            right: Box::new(right),
            body: Box::new(body),
            asynchronous,
        }
        .into())
    }

    /// Parses the `ForDeclaration` and `var ForBinding` productions.
    fn parse_for_declaration(&mut self) -> Result<ForDeclaration> {
        let span_start = self.position();
        let variable_kind = self.parse_optional_variable_kind()?;

        if let Some(kind) = variable_kind {
            let binding = self.parse_binding_pattern()?;
            return Ok(ForDeclaration::Declaration(ForBinding {
                span: self.span_from(span_start),
                kind,
                binding,
            }));
        }

        match self.current()? {
            token_matches!(punct!("[")) | token_matches!(punct!("{")) => {
                let assignment_pattern = self.parse_assignment_pattern()?;
                Ok(ForDeclaration::AssignmentPattern(assignment_pattern))
            }
            _ => {
                let expr = self.parse_left_hand_side_expr()?;

                expr.early_errors_left_hand_side_expr(&self.context)?;
                Ok(ForDeclaration::Expr(Box::new(expr)))
            }
        }
    }

    /// Parses the first `Expression` in `for (Expression; Expression; Expression;')`.
    /// Returns None if it does not exists or it failed to parse.
    fn parse_optional_for_init(&mut self) -> Result<Option<ForInit>> {
        if self.current_matches(&punct!(";")) {
            return Ok(None);
        }

        Ok(Some(self.parse_for_init()?))
    }

    fn parse_for_init(&mut self) -> Result<ForInit> {
        let span_start = self.position();

        let variable_kind = self.parse_optional_variable_kind()?;
        if let Some(kind) = variable_kind {
            return self.parse_for_init_variable_declaration(span_start, kind);
        }

        Ok(ForInit::Expr(Box::new(
            self.with_context(self.context.with_in(false))
                .parse_expr()?,
        )))
    }

    fn parse_for_init_variable_declaration(
        &mut self,
        span_start: usize,
        kind: VariableKind,
    ) -> Result<ForInit> {
        let declarations = self
            .with_context(self.context.with_in(false))
            .parse_variable_declarations()?;

        let span = self.span_from(span_start);
        Ok(ForInit::Declaration(StmtVariable {
            span,
            kind,
            declarations,
        }))
    }

    fn parse_optional_variable_kind(&mut self) -> Result<Option<VariableKind>> {
        let variable_kind = match self.current()? {
            token_matches!(keyword!("var")) => Some(VariableKind::Var),
            token_matches!(keyword!("let")) if self.peek_matches_lexical_binding() => {
                Some(VariableKind::Let)
            }
            token_matches!(keyword!("const")) => Some(VariableKind::Const),
            _ => None,
        };

        if variable_kind.is_some() {
            self.consume()?;
        }

        Ok(variable_kind)
    }
}
