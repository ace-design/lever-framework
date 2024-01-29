use tower_lsp::lsp_types::{HoverContents, MarkedString};

use crate::metadata::Symbol;

pub fn get_hover_info(symbol: &Symbol, type_symbol: &Symbol) -> Option<HoverContents> {
    Some(HoverContents::Scalar(MarkedString::String(format!(
        "{}: {}",
        symbol.name, type_symbol.name
    ))))
}
