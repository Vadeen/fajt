use crate::ast::{
    DeclExport, DeclImport, ExportDecl, ExportNamed, ExportNamespace, Ident, NamedExport,
    NamedImport, Stmt, VariableKind,
};
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
    /// Parses the `ExportDeclaration` goal symbol.
    pub(super) fn parse_export_declaration(&mut self) -> Result<Stmt> {
        let span_start = self.position();
        self.consume_assert(keyword!("export"))?;

        match self.current() {
            token_matches!(ok: punct!("{")) => self.parse_named_export(span_start),
            token_matches!(ok: punct!("*")) => self.parse_namespace_export(span_start),
            token_matches!(ok: keyword!("var")) => {
                self.parse_var_decl_export(span_start, VariableKind::Var)
            }
            token_matches!(ok: keyword!("let")) => {
                self.parse_var_decl_export(span_start, VariableKind::Let)
            }
            token_matches!(ok: keyword!("const")) => {
                self.parse_var_decl_export(span_start, VariableKind::Const)
            }
            token_matches!(ok: keyword!("class")) | token_matches!(ok: keyword!("function")) => {
                todo!("Hoistable/class declaration")
            }
            token_matches!(ok: keyword!("async")) if self.peek_matches(keyword!("function")) => {
                todo!("Hoistable/class declaration")
            }
            token_matches!(ok: keyword!("default")) => self.parse_default_export(),
            _ => err!(UnexpectedToken(self.consume()?)),
        }
    }

    fn parse_var_decl_export(
        &mut self,
        span_start: usize,
        variable_kind: VariableKind,
    ) -> Result<Stmt> {
        let decl = self.parse_variable_stmt(variable_kind)?;
        let span = self.span_from(span_start);
        Ok(DeclExport::Decl(ExportDecl {
            span,
            decl: Box::new(decl),
        })
        .into())
    }

    fn parse_default_export(&mut self) -> Result<Stmt> {
        self.consume_assert(keyword!("default"))?;
        match self.current()? {
            token_matches!(keyword!("class"))
            | token_matches!(keyword!("function"))
            | token_matches!(keyword!("async"))
                if self.peek_matches(keyword!("function")) =>
            {
                todo!("Default hoistable/class declaration")
            }
            _ => todo!("Default assignment expression"),
        }
    }

    /// Parses `export * from 'module'` and `export * as alias from 'module'`.
    fn parse_namespace_export(&mut self, span_start: usize) -> Result<Stmt> {
        self.consume_assert(punct!("*"))?;
        let alias = self
            .maybe_consume(keyword!("as"))?
            .then_try(|| self.parse_identifier())?;
        self.consume_assert(keyword!("from"))?;
        let from = self.parse_module_specifier()?;
        let span = self.span_from(span_start);
        Ok(DeclExport::Namespace(ExportNamespace { span, alias, from }).into())
    }

    /// Parses `export { name }` and `export { name as name2 } from 'other'`.
    fn parse_named_export(&mut self, span_start: usize) -> Result<Stmt> {
        let named_exports = self.parse_named_exports()?;
        let from = self
            .maybe_consume(keyword!("from"))?
            .then_try(|| self.parse_module_specifier())?;
        let span = self.span_from(span_start);
        Ok(DeclExport::Named(ExportNamed {
            span,
            named_exports,
            from,
        })
        .into())
    }

    /// Parses the `ImportDeclaration` goal symbol.
    pub(super) fn parse_import_declaration(&mut self) -> Result<Stmt> {
        let span_start = self.position();
        self.consume_assert(keyword!("import"))?;

        // `import "./module.js"`;
        if self.current_matches_string_literal() {
            let from = self.parse_module_specifier()?;
            self.consume_optional_semicolon()?;
            let span = self.span_from(span_start);
            return Ok(DeclImport {
                span,
                default_binding: None,
                namespace_binding: None,
                named_imports: None,
                from,
            }
            .into());
        }

        let default_binding = self.is_identifier().then_try(|| self.parse_identifier())?;
        let (named_imports, namespace_binding) =
            if default_binding.is_none() || self.maybe_consume(punct!(","))? {
                match self.current() {
                    token_matches!(ok: punct!("*")) => (None, Some(self.parse_namespace_import()?)),
                    token_matches!(ok: punct!("{")) => (Some(self.parse_named_imports()?), None),
                    _ => return err!(UnexpectedToken(self.consume()?)),
                }
            } else {
                (None, None)
            };

        self.consume_assert(keyword!("from"))?;
        let from = self.parse_module_specifier()?;

        self.consume_optional_semicolon()?;

        let span = self.span_from(span_start);
        Ok(DeclImport {
            span,
            default_binding,
            namespace_binding,
            named_imports,
            from,
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

    /// Parses the `NamedExports` goal symbol.
    fn parse_named_exports(&mut self) -> Result<Vec<NamedExport>> {
        self.consume_assert(punct!("{"))?;

        let mut named_exports = Vec::new();
        loop {
            if self.current_matches(punct!("}")) {
                self.consume()?;
                break;
            }

            named_exports.push(self.parse_export_specifier()?);
            self.consume_object_delimiter()?;
        }

        Ok(named_exports)
    }
    /// Parses the `ExportSpecifier` goal symbol.
    fn parse_export_specifier(&mut self) -> Result<NamedExport> {
        let span_start = self.position();
        let mut name = self.parse_identifier()?;

        // If there is an alias, we swap the name and alias identifiers, since the name should be
        // the name of the export, and the alias the local name.
        let alias_of = self
            .maybe_consume(keyword!("as"))?
            .then_try(|| self.parse_identifier())?
            .map(|alias| std::mem::replace(&mut name, alias));

        let span = self.span_from(span_start);
        Ok(NamedExport {
            span,
            name,
            alias_of,
        })
    }
}
