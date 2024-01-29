use std::{collections::HashMap, fs, path::PathBuf};

use petgraph::visit::EdgeRef;
use petgraph::EdgeDirection;
use petgraph::{dot::Dot, prelude::NodeIndex, Graph};
use serde_json::Value;
use tower_lsp::lsp_types::{
    CompletionContext, CompletionItem, CompletionTriggerKind, Diagnostic, HoverContents, Location,
    Position, Range, SemanticTokensResult, TextDocumentContentChangeEvent, Url, WorkspaceEdit,
};

use crate::features::hover::get_hover_info;
use crate::metadata::{AstEditor, AstQuery, SymbolId, SymbolTableQuery, Visitable};
use crate::{file::File, settings::Settings};

pub trait FileManagement {
    fn get_file(&self, url: &Url) -> Option<&File>;
    fn get_file_mut(&mut self, url: &Url) -> Option<&mut File>;
    fn add_file(&mut self, url: Url, content: &str);
    fn update_file(&mut self, url: &Url, changes: Vec<TextDocumentContentChangeEvent>);
}

pub trait LanguageActions {
    fn get_definition_location(&self, url: &Url, symbol_position: Position) -> Option<Location>;
    fn get_semantic_tokens(&self, url: &Url) -> Option<SemanticTokensResult>;
    fn rename_symbol(
        &mut self,
        url: &Url,
        symbol_position: Position,
        new_name: String,
    ) -> Option<WorkspaceEdit>;
    fn get_completion(
        &self,
        url: &Url,
        position: Position,
        context: Option<CompletionContext>,
    ) -> Option<Vec<CompletionItem>>;
    fn get_hover_info(&self, url: &Url, position: Position) -> Option<HoverContents>;
    fn get_quick_diagnostics(&self, url: &Url) -> Vec<Diagnostic>;
    fn get_full_diagnostics(&self, url: &Url) -> Vec<Diagnostic>;
}

#[derive(Debug, Clone)]
pub enum Import {
    Local,
    Library,
}

pub struct Workspace {
    root_path: Option<PathBuf>,
    settings: Settings,
    url_node_map: HashMap<Url, NodeIndex>,
    file_graph: Graph<File, Import>,
    tree_sitter_language: tree_sitter::Language,
}

impl Workspace {
    pub fn new(tree_sitter_language: tree_sitter::Language) -> Workspace {
        Workspace {
            root_path: None,
            settings: Settings::default(),
            url_node_map: HashMap::new(),
            file_graph: Graph::new(),
            tree_sitter_language,
        }
    }

    pub fn set_root_path(&mut self, path: Option<PathBuf>) {
        self.root_path = path;
    }

    pub fn update_settings(&mut self, settings: Value) {
        self.settings = Settings::parse(settings);
        info!("Settings: {:?}", self.settings);
    }

    fn add_file(&mut self, url: Url, content: &str) -> Option<NodeIndex> {
        if self.url_node_map.contains_key(&url) {
            return None;
        }

        let file = File::new(url.clone(), content, self.tree_sitter_language);

        let import_paths = file.get_import_paths();
        debug!("Resolved import paths: {:?}", import_paths);

        let new_file_index = self.file_graph.add_node(file);
        self.url_node_map.insert(url.clone(), new_file_index);

        for path in import_paths {
            match path {
                Ok((import_type, path)) => {
                    let imported_file_url = Url::from_file_path(path.clone()).unwrap();

                    let maybe_imported_file_index = if let Some(imported_file_index) =
                        self.url_node_map.get(&imported_file_url)
                    {
                        self.file_graph
                            .add_edge(new_file_index, *imported_file_index, import_type);
                        Some(*imported_file_index)
                    } else {
                        let content = fs::read_to_string(path).unwrap();
                        let imported_file_index = self.add_file(imported_file_url, &content);
                        if let Some(i) = imported_file_index {
                            self.file_graph.add_edge(new_file_index, i, import_type);
                            Some(i)
                        } else {
                            None
                        }
                    };

                    if let Some(imported_file_index) = maybe_imported_file_index {
                        self.link_imported_symbols(&new_file_index, &imported_file_index)
                    }
                }
                Err(range) => {
                    info!("Import problem");

                    crate::features::diagnostics::ImportErrors::add_error(
                        url.clone(),
                        Diagnostic::new_simple(range, String::from("File could not be found.")),
                    );
                }
            }
        }

        debug!("File graph:\n{:?}", Dot::with_config(&self.file_graph, &[]));

        Some(new_file_index)
    }

    fn link_imported_symbols(&mut self, file_index: &NodeIndex, imported_file_index: &NodeIndex) {
        let imported_file = self.file_graph.node_weight(*imported_file_index).unwrap();
        let (imported_symbols, scope_id) = imported_file
            .symbol_table_manager
            .lock()
            .unwrap()
            .get_symbols_at_root();
        let file = self.file_graph.node_weight_mut(*file_index).unwrap();

        let unlinked_symbols: HashMap<String, Range> = file
            .symbol_table_manager
            .lock()
            .unwrap()
            .get_unlinked_symbols()
            .into_iter()
            .collect();

        for (i, s) in imported_symbols.into_iter().enumerate() {
            if let Some(range) = unlinked_symbols.get(&s.get_name()) {
                let symbol_id = SymbolId::new(Some(*imported_file_index), scope_id, i);

                file.ast_manager
                    .lock()
                    .unwrap()
                    .link_symbol(symbol_id, *range);
            }
        }
    }

    fn clear_outgoing_edges(&mut self, file_index: &NodeIndex) {
        let outgoing_edges: Vec<_> = self
            .file_graph
            .edges_directed(*file_index, EdgeDirection::Outgoing)
            .map(|edge| edge.id())
            .collect();

        for id in outgoing_edges {
            self.file_graph.remove_edge(id);
        }
    }
}

impl FileManagement for Workspace {
    fn get_file(&self, url: &Url) -> Option<&File> {
        let index = self.url_node_map.get(url)?;
        self.file_graph.node_weight(*index)
    }

    fn get_file_mut(&mut self, url: &Url) -> Option<&mut File> {
        let index = self.url_node_map.get(url)?;
        self.file_graph.node_weight_mut(*index)
    }

    fn add_file(&mut self, url: Url, content: &str) {
        self.add_file(url, content);
    }

    fn update_file(&mut self, url: &Url, changes: Vec<TextDocumentContentChangeEvent>) {
        crate::features::diagnostics::ImportErrors::clear(url);
        let file_index = *self.url_node_map.get(url).unwrap();
        self.clear_outgoing_edges(&file_index);

        let file = self.get_file_mut(url).unwrap();

        file.update(changes);

        for path in file.get_import_paths() {
            match path {
                Ok((import_type, path)) => {
                    let imported_file_url = Url::from_file_path(path.clone()).unwrap();

                    let maybe_imported_file_index = if let Some(imported_file_index) =
                        self.url_node_map.get(&imported_file_url)
                    {
                        self.file_graph
                            .add_edge(file_index, *imported_file_index, import_type);
                        Some(*imported_file_index)
                    } else {
                        let content = fs::read_to_string(path).unwrap();
                        let imported_file_index = self.add_file(imported_file_url, &content);
                        if let Some(i) = imported_file_index {
                            self.file_graph.add_edge(file_index, i, import_type);
                            Some(i)
                        } else {
                            None
                        }
                    };

                    if let Some(imported_file_index) = maybe_imported_file_index {
                        self.link_imported_symbols(&file_index, &imported_file_index)
                    }
                }
                Err(range) => {
                    crate::features::diagnostics::ImportErrors::add_error(
                        url.clone(),
                        Diagnostic::new_simple(range, String::from("File could not be found.")),
                    );
                }
            }
        }

        debug!("File graph:\n{:?}", Dot::with_config(&self.file_graph, &[]));
    }
}

impl LanguageActions for Workspace {
    fn get_definition_location(&self, url: &Url, symbol_position: Position) -> Option<Location> {
        let file = self.get_file(url)?;

        let ast_query = file.ast_manager.lock().unwrap();
        let root_visit = ast_query.visit_root();
        let node = root_visit.get_node_at_position(symbol_position)?;

        let symbol_id = node.get().linked_symbol.clone()?;

        Some(if let Some(file_index) = symbol_id.get_file_id() {
            let other_file = self.file_graph.node_weight(file_index).unwrap();
            let range = other_file
                .symbol_table_manager
                .lock()
                .unwrap()
                .get_symbol(symbol_id)?
                .get_definition_range();

            Location {
                uri: other_file.uri.clone(),
                range,
            }
        } else {
            let range = file
                .symbol_table_manager
                .lock()
                .unwrap()
                .get_symbol(symbol_id)?
                .get_definition_range();

            Location {
                uri: url.clone(),
                range,
            }
        })
    }

    fn rename_symbol(
        &mut self,
        url: &Url,
        symbol_position: Position,
        new_name: String,
    ) -> Option<WorkspaceEdit> {
        let file = self.get_file_mut(url).unwrap();

        file.rename_symbol(symbol_position, new_name)
    }

    fn get_semantic_tokens(&self, url: &Url) -> Option<SemanticTokensResult> {
        let file = self.get_file(url)?;

        Some(file.get_semantic_tokens())
    }

    fn get_completion(
        &self,
        url: &Url,
        position: Position,
        context: Option<CompletionContext>,
    ) -> Option<Vec<CompletionItem>> {
        let file_index = *self.url_node_map.get(url).unwrap();
        let file = self.get_file(url)?;

        if context.is_none()
            || context.clone().unwrap().trigger_kind == CompletionTriggerKind::INVOKED
        {
            if let Some(mut items) = file.get_completion_list(position, context) {
                for edge in self
                    .file_graph
                    .edges_directed(file_index, EdgeDirection::Outgoing)
                {
                    let imported_file = self.file_graph.node_weight(edge.target()).unwrap();
                    debug!("{}", imported_file.uri.to_string());
                    items.append(&mut imported_file.get_import_completion_list());
                }
                Some(items)
            } else {
                None
            }
        } else {
            file.get_completion_list(position, context)
        }
    }

    fn get_hover_info(&self, url: &Url, position: Position) -> Option<HoverContents> {
        let file = self.get_file(url)?;

        if let Some(symbol_id) = file.get_symbol_id_at_pos(position) {
            let symbol = {
                let st = if let Some(file_id) = symbol_id.get_file_id() {
                    self.file_graph
                        .node_weight(file_id)
                        .unwrap()
                        .symbol_table_manager
                        .lock()
                        .unwrap()
                } else {
                    file.symbol_table_manager.lock().unwrap()
                };

                st.get_symbol(symbol_id)?.clone()
            };

            let st = if let Some(file_id) = symbol.get_type_symbol()?.get_file_id() {
                self.file_graph
                    .node_weight(file_id)
                    .unwrap()
                    .symbol_table_manager
                    .lock()
                    .unwrap()
            } else {
                file.symbol_table_manager.lock().unwrap()
            };

            let type_symbol = st.get_symbol(symbol.get_type_symbol()?)?;

            get_hover_info(&symbol, type_symbol)
        } else {
            None
        }
    }

    fn get_quick_diagnostics(&self, url: &Url) -> Vec<Diagnostic> {
        let maybe_file = self.get_file(url);

        if let Some(file) = maybe_file {
            file.get_quick_diagnostics()
        } else {
            vec![]
        }
    }

    fn get_full_diagnostics(&self, url: &Url) -> Vec<Diagnostic> {
        let maybe_file = self.get_file(url);

        if let Some(file) = maybe_file {
            file.get_full_diagnostics()
        } else {
            vec![]
        }
    }
}
