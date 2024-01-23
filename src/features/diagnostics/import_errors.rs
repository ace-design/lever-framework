// Might be a bad idea

use once_cell::sync::Lazy;
use std::{
    cell::Cell,
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::metadata::{AstQuery, SymbolTableQuery};
use tower_lsp::lsp_types::{Diagnostic, Url};

use super::provider::DiagnosticProvider;

type DiagnosticBuffer = Cell<HashMap<Url, Vec<Diagnostic>>>;

static BUFFER_INSTANCE: Lazy<Mutex<DiagnosticBuffer>> =
    Lazy::new(|| Mutex::new(Cell::new(HashMap::new())));

pub struct ImportErrors {}

impl ImportErrors {
    pub fn add_error(uri: Url, diag: Diagnostic) {
        let mut lock = BUFFER_INSTANCE.lock().unwrap();
        let diags = lock.get_mut();
        let entry = diags.entry(uri).or_default();
        entry.push(diag);
    }

    pub fn clear(uri: &Url) {
        let mut lock = BUFFER_INSTANCE.lock().unwrap();
        if let Some(diags) = lock.get_mut().get_mut(uri) {
            diags.clear();
        }
    }
}

impl DiagnosticProvider for ImportErrors {
    fn get_diagnostics(
        uri: &Url,
        _ast_query: &Arc<Mutex<impl AstQuery>>,
        _symbol_table_query: &Arc<Mutex<impl SymbolTableQuery>>,
    ) -> Vec<Diagnostic> {
        let mut lock = BUFFER_INSTANCE.lock().unwrap();
        let diags = lock.get_mut().get(uri);

        if let Some(diags) = diags {
            diags.clone()
        } else {
            vec![]
        }
    }
}
