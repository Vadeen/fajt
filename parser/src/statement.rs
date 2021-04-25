use crate::ast::{
    BindingIdentifier, BindingPattern, BindingProperty, Stmt, VariableDeclaration, VariableStmt,
    VariableType,
};
use crate::Parser;
use fajt_lexer::token::TokenValue;
use std::convert::TryInto;

impl Parser<'_> {
    pub(super) fn parse_variable_statement(&mut self, variable_type: VariableType) -> Stmt {
        Stmt::VariableStmt(VariableStmt {
            variable_type,
            declarations: vec![self.parse_variable_declaration()],
        })
    }

    fn parse_variable_declaration(&mut self) -> VariableDeclaration {
        let token = self.reader.next();

        let identifier = match &token.value {
            TokenValue::Identifier(_) => BindingPattern::Ident(BindingIdentifier::Ident(
                token.try_into().expect("Expected identifier"),
            )),
            punct!("{") => self.parse_object_property_binding(),
            punct!("[") => unimplemented!("Array binding"),
            _ => unimplemented!(),
        };

        VariableDeclaration { identifier }
    }

    fn parse_object_property_binding(&mut self) -> BindingPattern {
        let mut bindings = Vec::new();

        loop {
            let token = self.reader.next();

            match token.value {
                punct!("}") => break,
                TokenValue::Identifier(_) => bindings.push(BindingProperty::Single(
                    BindingIdentifier::Ident(token.try_into().unwrap()),
                )),
                _ => unimplemented!(),
            }
        }

        BindingPattern::Object(bindings)
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::VariableType::Var;
    use crate::ast::{
        BindingIdentifier, BindingPattern, BindingProperty, EmptyStmt, Ident, Program, Stmt,
        VariableDeclaration, VariableStmt,
    };
    use crate::{Parser, Reader};
    use fajt_lexer::Lexer;

    #[test]
    fn parse_empty_statement() {
        parser_test!(
            input: ";",
            output: [Stmt::Empty(EmptyStmt::new((0, 1).into()))]
        );
    }

    #[test]
    fn parse_var_statement() {
        parser_test!(
            input: "var foo = 1;",
            output: [
                Stmt::VariableStmt(VariableStmt {
                    variable_type: Var,
                    declarations: vec![
                        VariableDeclaration {
                            identifier: BindingPattern::Ident(BindingIdentifier::Ident(Ident {
                                span: (4, 7).into(),
                                name: "foo".to_string()
                            })),
                        }
                    ]
                })
            ]
        );
    }

    #[test]
    fn parse_var_stmt_empty_obj_binding() {
        parser_test!(
            input: "var {} = 1;",
            output: [
                Stmt::VariableStmt(VariableStmt {
                    variable_type: Var,
                    declarations: vec![
                        VariableDeclaration {
                            identifier: BindingPattern::Object(vec![]),
                        }
                    ]
                })
            ]
        );
    }

    #[test]
    fn parse_var_stmt_empty_single_binding() {
        parser_test!(
            input: "var { a } = b;",
            output: [
                Stmt::VariableStmt(VariableStmt {
                    variable_type: Var,
                    declarations: vec![
                        VariableDeclaration {
                            identifier: BindingPattern::Object(vec![BindingProperty::Single(
                                BindingIdentifier::Ident(Ident {
                                    name: "a".to_string(),
                                    span: (6, 7).into()
                                })
                            )]),
                        }
                    ]
                })
            ]
        );
    }
}
