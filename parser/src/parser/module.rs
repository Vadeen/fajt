use crate::ast::{DeclImport, Ident, NamedImport, Stmt};
use crate::error::ErrorKind::UnexpectedToken;
use crate::error::{Result, ThenTry};
use crate::Parser;
use fajt_common::io::PeekRead;
use fajt_lexer::keyword;
use fajt_lexer::punct;
use fajt_lexer::token::Token;
use fajt_lexer::token_matches;

impl<I> Parser<'_, I>
where
    I: PeekRead<Token, Error = fajt_lexer::error::Error>,
{
    /// Parses the `ImportDeclaration` goal symbol.
    pub(super) fn parse_import_declaration(&mut self) -> Result<Stmt> {
        let span_start = self.position();
        self.consume_assert(keyword!("import"))?;

        // `import "./module.js"`;
        if self.current_matches_string_literal() {
            let source = self.parse_module_specifier()?;
            let span = self.span_from(span_start);
            return Ok(DeclImport {
                span,
                default_binding: None,
                namespace_binding: None,
                named_imports: None,
                source,
            }
            .into());
        }

        let default_binding = self.is_identifier().then_try(|| self.parse_identifier())?;
        let (named_imports, namespace_binding) = if default_binding.is_none()
            || self.maybe_consume(punct!(","))?
        {
            match self.current() {
                token_matches!(ok: punct!("*")) => (None, Some(self.parse_namespace_import()?)),
                token_matches!(ok: punct!("{")) => (Some(self.parse_named_imports()?), None),
                _ => return err!(UnexpectedToken(self.consume()?)),
            }
        } else {
            (None, None)
        };

        self.consume_assert(keyword!("from"))?;
        let source = self.parse_module_specifier()?;

        let span = self.span_from(span_start);
        Ok(DeclImport {
            span,
            default_binding,
            namespace_binding,
            named_imports,
            source,
        }
        .into())
    }

    /// Parses the `ModuleSpecifier` goal symbol.
    fn parse_module_specifier(&mut self) -> Result<String> {
        let (module_name, _) = self
            .parse_literal()?
            .unwrap_literal()
            .literal
            .unwrap_string();
        Ok(module_name)
    }

    /// Parses the `NameSpaceImport` goal symbol.
    fn parse_namespace_import(&mut self) -> Result<Ident> {
        self.consume_assert(punct!("*"))?;
        self.consume_assert(keyword!("as"))?;
        self.parse_identifier()
    }

    /// Parses the `NamedImports` goal symbol.
    fn parse_named_imports(&mut self) -> Result<Vec<NamedImport>> {
        self.consume_assert(punct!("{"))?;

        let mut named_imports = Vec::new();
        loop {
            if self.current_matches(punct!("}")) {
                self.consume()?;
                break;
            }

            named_imports.push(self.parse_import_specifier()?);
            self.consume_object_delimiter()?;
        }

        Ok(named_imports)
    }

    /// Parses the `ImportSpecifier` goal symbol.
    fn parse_import_specifier(&mut self) -> Result<NamedImport> {
        let span_start = self.position();
        let name = self.parse_identifier()?;
        let alias = self
            .maybe_consume(keyword!("as"))?
            .then_try(|| self.parse_identifier())?;
        let span = self.span_from(span_start);
        Ok(NamedImport { span, name, alias })
    }
}
