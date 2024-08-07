use std::sync::{Arc, Mutex};

use tower_lsp::lsp_types::{Diagnostic, Url};

use super::parse::Parse;

use crate::project::{features::diagnostics::ImportErrors, AstQuery, SymbolTableQuery};

macro_rules! diags {
    ($($diag:expr),*) => {
        {
            let mut diags = Vec::new();

            $(
                diags.append(&mut $diag);
            )*

            diags
        }
    };
}

pub trait DiagnosticProvider {
    fn get_diagnostics(
        uri: &Url,
        ast_query: &Arc<Mutex<impl AstQuery>>,
        symbol_table_query: &Arc<Mutex<impl SymbolTableQuery>>,
    ) -> Vec<Diagnostic>;
}

pub fn get_quick(
    uri: &Url,
    ast_query: &Arc<Mutex<impl AstQuery>>,
    symbol_table_query: &Arc<Mutex<impl SymbolTableQuery>>,
) -> Vec<Diagnostic> {
    diags![
        Parse::get_diagnostics(uri, ast_query, symbol_table_query),
        ImportErrors::get_diagnostics(uri, ast_query, symbol_table_query)
    ]
}

pub fn get_full(
    uri: &Url,
    ast_query: &Arc<Mutex<impl AstQuery>>,
    symbol_table_query: &Arc<Mutex<impl SymbolTableQuery>>,
) -> Vec<Diagnostic> {
    diags![
        Parse::get_diagnostics(uri, ast_query, symbol_table_query),
        ImportErrors::get_diagnostics(uri, ast_query, symbol_table_query)
    ]
}
