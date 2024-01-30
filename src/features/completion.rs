use std::sync::{Arc, Mutex};

use crate::{
    language_def::LanguageDefinition,
    metadata::{AstQuery, SymbolTableQuery, Visitable},
};
use tower_lsp::lsp_types::{
    CompletionContext, CompletionItem, CompletionItemKind, CompletionItemLabelDetails,
    CompletionTriggerKind, Position, Url,
};

fn default_list(
    position: Position,
    query: &Arc<Mutex<impl SymbolTableQuery>>,
) -> Option<Vec<CompletionItem>> {
    let mut items: Vec<CompletionItem> = Vec::new();

    for symbol in query.lock().unwrap().get_symbols_at_pos(position) {
        items.push(CompletionItem {
            label: symbol.name.clone(),
            kind: get_symbol_completion_type(symbol.name),
            ..Default::default()
        })
    }

    Some(items)
}

fn get_symbol_completion_type(symbol_kind: String) -> Option<CompletionItemKind> {
    Some(
        LanguageDefinition::get()
            .symbol_types
            .iter()
            .find(|symbol_type| symbol_type.name == symbol_kind)?
            .completion_type
            .get(),
    )
}

pub fn get_imported_list(
    uri: &Url,
    st_query: &Arc<Mutex<impl SymbolTableQuery>>,
) -> Vec<CompletionItem> {
    let query = st_query.lock().unwrap();

    query
        .get_symbols_at_root()
        .0
        .iter()
        .map(|s| CompletionItem {
            label: s.name.clone(),
            label_details: Some(CompletionItemLabelDetails {
                detail: Some(uri.path_segments().unwrap().last().unwrap().to_string()),
                description: None,
            }),
            kind: get_symbol_completion_type(s.kind.clone()),
            ..Default::default()
        })
        .collect()
}

pub fn get_list(
    position: Position,
    ast_query: &Arc<Mutex<impl AstQuery>>,
    st_query: &Arc<Mutex<impl SymbolTableQuery>>,
    context: Option<CompletionContext>,
) -> Option<Vec<CompletionItem>> {
    if let Some(context) = context {
        if context.trigger_kind == CompletionTriggerKind::TRIGGER_CHARACTER {
            let ast_query = ast_query.lock().unwrap();
            let root_visit = ast_query.visit_root();

            let new_pos = Position {
                line: position.line,
                character: position.character - 1,
            };
            let node = root_visit.get_node_at_position(new_pos)?;
            debug!("{:?}", node.get());
            if let Some(linked_symbol_id) = node.get().linked_symbol.clone() {
                let st_query = st_query.lock().unwrap();
                let type_symbol_id = st_query.get_symbol(linked_symbol_id)?.type_symbol.clone()?;
                if let Some(type_symbol) = st_query.get_symbol(type_symbol_id) {
                    let symbols = st_query.get_symbols_in_scope(type_symbol.field_scope_id?);

                    return Some(
                        symbols
                            .iter()
                            .map(|item| CompletionItem {
                                label: item.name.clone(),
                                kind: Some(CompletionItemKind::FIELD),
                                ..Default::default()
                            })
                            .collect(),
                    );
                }
            }
        }
    }

    default_list(position, st_query)
}
