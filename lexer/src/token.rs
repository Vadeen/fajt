use macros::FromString;

#[macro_export]
macro_rules! token_matches {
    ($token:expr, $value:pat) => {
        matches!($token, crate::token::Token { value: $value, .. });
    };
    ($value:pat) => {
        crate::token::Token { value: $value, .. }
    };
    (@ident) => {
        token_matches!(crate::token::TokenValue::Identifier(_))
    };
    ($token:expr, opt: $value:pat) => {
        matches!($token, Some(crate::token::Token { value: $value, .. }));
    };
    (opt: $value:pat) => {
        Some(crate::token::Token { value: $value, .. })
    };
}

/// When working with tokens, do not use this enum directly. Instead use the macro
/// generated by the`from_string_macro`.
///
/// Example:
/// ```
/// # #[macro_use]
/// # extern crate fajt_lexer;
/// # use fajt_lexer::token;
/// # use fajt_lexer::token::{TokenValue, Keyword};
/// # fn main() {
/// assert_eq!(keyword!("const"), TokenValue::Keyword(Keyword::Const))
/// # }
/// ```
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, FromString, Clone)]
#[from_string_macro("keyword")]
#[from_string_macro_rules(
    ($variant:ident) => {
        crate::token::TokenValue::Keyword(crate::token::Keyword::$variant)
    };
)]
pub enum Keyword {
    Await,
    Break,
    Case,
    Catch,
    Class,
    Const,
    Continue,
    Debugger,
    Default,
    Delete,
    Do,
    Else,
    Enum,
    Export,
    Extends,
    False,
    Finally,
    For,
    Function,
    If,
    Implements,
    Import,
    In,
    Instanceof,
    Interface,
    Let,
    New,
    Null,
    Of,
    Package,
    Private,
    Protected,
    Public,
    Return,
    Static,
    Super,
    Switch,
    This,
    Throw,
    True,
    Try,
    Type,
    Var,
    Void,
    While,
    With,
    Yield,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
pub enum Base {
    Binary,
    Decimal,
    Hex,
    Octal,
}

#[derive(Debug, PartialOrd, PartialEq, Clone)]
pub enum Number {
    Integer(i64, Base),
    Decimal(f64),
}

/// When working with tokens, do not use this enum directly. Instead use the macro
/// generated by the`from_string_macro`.
///
/// Example:
/// ```
/// # #[macro_use]
/// # extern crate fajt_lexer;
/// # use fajt_lexer::token;
/// # use fajt_lexer::token::{TokenValue, Punct};
/// # fn main() {
/// assert_eq!(punct!("["), TokenValue::Punct(Punct::BraceOpen))
/// # }
/// ```
#[derive(Debug, PartialOrd, PartialEq, FromString, Clone)]
#[from_string_macro("punct")]
#[from_string_macro_rules(
    ($variant:ident) => {
        crate::token::TokenValue::Punct(crate::token::Punct::$variant)
    };
)]
pub enum Punct {
    #[from_string("(")]
    ParantOpen,
    #[from_string(")")]
    ParantClose,
    #[from_string("[")]
    BraceOpen,
    #[from_string("]")]
    BraceClose,
    #[from_string("{")]
    BracketOpen,
    #[from_string("}")]
    BracketClose,
    #[from_string(".")]
    Dot,
    #[from_string("...")]
    TripleDot,
    #[from_string(";")]
    SemiColon,
    #[from_string(",")]
    Comma,
    #[from_string("<")]
    LessThan,
    #[from_string("<<")]
    DoubleLessThan,
    #[from_string(">")]
    GreaterThan,
    #[from_string(">>")]
    DoubleGreaterThan,
    #[from_string(">>>")]
    TripleGreaterThan,
    #[from_string("=")]
    Equal,
    #[from_string("==")]
    DoubleEqual,
    #[from_string("<=")]
    LessEqual,
    #[from_string("<<=")]
    DoubleLessEqual,
    #[from_string(">=")]
    GreaterEqual,
    #[from_string(">>=")]
    DoubleGreaterEqual,
    #[from_string(">>>=")]
    TripleGreaterEqual,
    #[from_string("=>")]
    EqualGreater,
    #[from_string("!=")]
    NotEqual,
    #[from_string("+=")]
    PlusEqual,
    #[from_string("-=")]
    MinusEqual,
    #[from_string("*=")]
    StarEqual,
    #[from_string("**=")]
    DoubleStarEqual,
    #[from_string("/=")]
    SlashEqual,
    #[from_string("%=")]
    PercentEqual,
    #[from_string("|=")]
    PipeEqual,
    #[from_string("^=")]
    CaretEqual,
    #[from_string("&=")]
    AmpersandEqual,
    #[from_string("===")]
    TripleEqual,
    #[from_string("!==")]
    ExclamationDoubleEqual,
    #[from_string("+")]
    Plus,
    #[from_string("++")]
    DoublePlus,
    #[from_string("-")]
    Minus,
    #[from_string("--")]
    DoubleMinus,
    #[from_string("*")]
    Star,
    #[from_string("**")]
    DoubleStar,
    #[from_string("/")]
    Slash,
    #[from_string("%")]
    Percent,
    #[from_string("&")]
    Ampersand,
    #[from_string("&&")]
    DoubleAmpersand,
    #[from_string("|")]
    Pipe,
    #[from_string("||")]
    DoublePipe,
    #[from_string("^")]
    Caret,
    #[from_string("!")]
    Exclamation,
    #[from_string("~")]
    Tilde,
    #[from_string("?")]
    QuestionMark,
    #[from_string("??")]
    DoubleQuestionMark,
    #[from_string(":")]
    Colon,
}

#[macro_export]
macro_rules! literal(
    (integer, $value:expr) => {
        literal!(number, crate::token::Base::Decimal, $value)
    };
    (hex, $value:expr) => {
        literal!(number, crate::token::Base::Hex, $value)
    };
    (octal, $value:expr) => {
        literal!(number, crate::token::Base::Octal, $value)
    };
    (binary, $value:expr) => {
        literal!(number, crate::token::Base::Binary, $value)
    };
    (number, $type:expr, $value:expr) => {
        crate::token::TokenValue::Literal(
            crate::token::Literal::Number(
                crate::token::Number::Integer(
                    $value, $type
                )
            )
        )
    };
);

#[derive(Debug, PartialOrd, PartialEq, Clone)]
pub enum Literal {
    Number(Number),
}

#[derive(Debug, PartialOrd, PartialEq, Clone)]
pub enum TokenValue {
    Keyword(Keyword),
    Identifier(String),
    Punct(Punct),
    Literal(Literal),
}

#[derive(Debug, PartialOrd, PartialEq, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl From<(usize, usize)> for Span {
    fn from((start, end): (usize, usize)) -> Self {
        Span { start, end }
    }
}

#[derive(Debug, PartialOrd, PartialEq, Clone)]
pub struct Token {
    pub value: TokenValue,
    pub span: Span,
}

impl Token {
    pub fn new<S: Into<Span>>(value: TokenValue, span: S) -> Self {
        Token {
            value,
            span: span.into(),
        }
    }
}
