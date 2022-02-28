use crate::error::Result;
use crate::{Error, Parser};
use fajt_ast::{
    ArrayElement, AssignmentOperator, Expr, ExprLiteral, FormalParameters, LitArray, LitObject,
    Literal, PropertyDefinition, Spanned,
};
use fajt_common::io::{PeekRead, ReReadWithState};
use fajt_lexer::token::Token;
use fajt_lexer::LexerState;

// TODO start thinking where to really place these.
impl<I> Parser<'_, I>
where
    I: PeekRead<Token, Error = fajt_lexer::error::Error>,
    I: ReReadWithState<Token, State = LexerState, Error = fajt_lexer::error::Error>,
{
    /// Early error on invalid update expression argument.
    pub(super) fn validate_update_expression_argument(&self, argument: &Expr) -> Result<()> {
        if !self.is_assignment_target_type_simple(argument)? {
            return Err(Error::syntax_error(
                "Invalid update expression argument".to_owned(),
                argument.span().clone(),
            ));
        }

        Ok(())
    }

    /// Early error for invalid delete argument.
    pub(super) fn validate_delete_argument(&self, argument: &Expr) -> Result<()> {
        if !self.context.is_strict {
            return Ok(());
        }

        match argument {
            Expr::IdentRef(ident) => {
                return Err(Error::syntax_error(
                    "Delete of an unqualified identifier in strict mode".to_owned(),
                    ident.span.clone(),
                ));
            }
            Expr::Parenthesized(parenthesized) => {
                return self.validate_delete_argument(parenthesized.expression.as_ref());
            }
            _ => {}
        }

        Ok(())
    }

    /// Early error on invalid left hand side expression.
    pub(super) fn validate_left_side_expr(
        &self,
        expr: &Expr,
        operator: &AssignmentOperator,
    ) -> Result<()> {
        if operator == &AssignmentOperator::Assign {
            match expr {
                Expr::Literal(ExprLiteral {
                    literal: Literal::Array(array),
                    ..
                }) => {
                    return self.validate_array_literal_cover_assignment(array);
                }
                Expr::Literal(ExprLiteral {
                    literal: Literal::Object(object),
                    ..
                }) => {
                    return self.validate_object_literal_cover_assignment(object);
                }
                _ => {}
            }
        }

        if !self.is_assignment_target_type_simple(expr)? {
            return Err(Error::syntax_error(
                "Invalid left-hand side assignment".to_owned(),
                expr.span().clone(),
            ));
        }

        Ok(())
    }

    /// Early error if `ArrayLiteral` do not cover `ArrayAssignment`.
    fn validate_array_literal_cover_assignment(&self, array: &LitArray) -> Result<()> {
        let mut elements = array.elements.iter().peekable();

        while let Some(element) = elements.next() {
            if elements.peek().is_none() {
                break;
            }

            if let ArrayElement::Spread(spread) = element {
                return Err(Error::syntax_error(
                    "Rest element must be last element".to_owned(),
                    spread.span().clone(),
                ));
            }
        }

        Ok(())
    }

    /// Early error if `ObjectLiteral` do not cover `ObjectAssignment`.
    fn validate_object_literal_cover_assignment(&self, object: &LitObject) -> Result<()> {
        let mut props = object.props.iter().peekable();

        while let Some(prop) = props.next() {
            if let PropertyDefinition::Method(method) = prop {
                return Err(Error::syntax_error(
                    "Invalid destructuring assignment target".to_owned(),
                    method.span.clone(),
                ));
            }

            if props.peek().is_some() {
                if let PropertyDefinition::Spread(spread) = prop {
                    return Err(Error::syntax_error(
                        "Rest element must be last element".to_owned(),
                        spread.span().clone(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Returns true if `AssignmentTargetType` for `expr` is simple.
    fn is_assignment_target_type_simple(&self, expr: &Expr) -> Result<bool> {
        Ok(match expr {
            Expr::IdentRef(ident) => {
                if self.context.is_strict && (ident.name == "arguments" || ident.name == "eval") {
                    return Err(Error::syntax_error(
                        "Unexpected `eval` or `arguments` in strict mode".to_owned(),
                        expr.span().clone(),
                    ));
                } else {
                    true
                }
            }
            Expr::Member(_) => true,
            _ => false,
        })
    }

    pub(super) fn validate_unique_formal_parameters(
        &self,
        params: &FormalParameters,
    ) -> Result<()> {
        let mut bound_names = self.get_bound_names(params);
        bound_names.sort_unstable();

        let first_duplicate = get_first_duplicate(&bound_names);

        if let Some(duplicate) = first_duplicate {
            return Err(Error::syntax_error(
                format!(
                    "Found duplicate parameter '{}', duplicates not allowed here",
                    duplicate
                ),
                params.span.clone(),
            ));
        }

        Ok(())
    }

    pub(super) fn validate_property_set_parameters(&self, params: &FormalParameters) -> Result<()> {
        if params.rest.is_some() {
            return Err(Error::syntax_error(
                "Setter function parameter must not be a rest parameter".to_owned(),
                params.span.clone(),
            ));
        }

        if params.bindings.len() != 1 {
            return Err(Error::syntax_error(
                "Setter must have exactly one parameter".to_owned(),
                params.span.clone(),
            ));
        }

        Ok(())
    }

    fn get_bound_names<'a>(&self, params: &'a FormalParameters) -> Vec<&'a str> {
        let mut names: Vec<&str> = params
            .bindings
            .iter()
            .flat_map(|binding| binding.pattern.get_bound_names())
            .collect();

        if let Some(rest) = &params.rest {
            names.append(&mut rest.as_ref().get_bound_names());
        }

        names
    }
}

/// Assumes the `list` is sorted.
fn get_first_duplicate<'a>(list: &[&'a str]) -> Option<&'a str> {
    let mut iter = list.iter().peekable();
    while let Some(item) = iter.next() {
        if let Some(peek) = iter.peek() {
            if item == *peek {
                return Some(item);
            }
        }
    }

    None
}
