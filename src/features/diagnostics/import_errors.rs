// Might be a bad idea

use once_cell::sync::Lazy;
use std::{
    cell::Cell,
    sync::{Arc, Mutex},
};

use crate::metadata::{AstQuery, SymbolTableQuery};
use tower_lsp::lsp_types::{Diagnostic, Url};

use super::provider::DiagnosticProvider;

static BUFFER_INSTANCE: Lazy<Mutex<Cell<Vec<Diagnostic>>>> =
    Lazy::new(|| Mutex::new(Cell::new(vec![])));

pub struct ImportErrors {}

impl ImportErrors {
    pub fn add_error(uri: Url, diag: Diagnostic) {
        let mut lock = BUFFER_INSTANCE.lock().unwrap();
        let diags = lock.get_mut();
        diags.push(diag)
    }
}

impl DiagnosticProvider for ImportErrors {
    fn get_diagnostics(
        _ast_query: &Arc<Mutex<impl AstQuery>>,
        _symbol_table_query: &Arc<Mutex<impl SymbolTableQuery>>,
    ) -> Vec<Diagnostic> {
        let mut lock = BUFFER_INSTANCE.lock().unwrap();
        let diags = lock.get_mut();

        let returned_diags = diags.clone();

        diags.clear();

        returned_diags
    }
}
