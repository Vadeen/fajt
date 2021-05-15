mod lib;

use fajt_lexer::token::Span;
use fajt_parser::ast::*;

#[test]
fn empty_statement() {
    parser_test!(
        input: ";",
        output: [
            EmptyStatement {
                span: Span::new(0, 1)
            }.into()
        ]
    );
}

#[test]
fn empty_block_statement() {
    parser_test!(
        input: "{ }",
        output: [
            BlockStatement {
                span: Span::new(0, 3),
                statements: vec![]
            }.into()
        ]
    );
}
