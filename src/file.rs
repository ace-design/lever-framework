use std::env;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tower_lsp::lsp_types::{
    self, CompletionContext, CompletionItem, Diagnostic, Location, Position, SemanticTokensResult,
    TextDocumentContentChangeEvent, Url, WorkspaceEdit,
};
use tree_sitter::{InputEdit, Parser, Tree};

use crate::features::{completion, diagnostics, goto, rename, semantic_tokens};
use crate::language_def::{Import, LanguageDefinition};
use crate::metadata::{
    AstEditor, AstManager, AstQuery, SymbolId, SymbolTableEditor, SymbolTableManager, Visitable,
};
use crate::{utils, workspace};

pub struct File {
    pub uri: Url,
    pub source_code: String,
    pub tree: Tree,
    pub symbol_table_manager: Arc<Mutex<SymbolTableManager>>,
    pub ast_manager: Arc<Mutex<AstManager>>,
    parser: tree_sitter::Parser,
}

// Mainly used for debugging File graph
impl Debug for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("File")
            .field("name", &self.uri.path_segments().unwrap().last().unwrap())
            .finish()
    }
}

impl File {
    pub fn new(uri: Url, source_code: &str, tree_sitter_language: tree_sitter::Language) -> File {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_language).unwrap();

        let tree = parser.parse(source_code, None).unwrap();

        let ast_manager = Arc::new(Mutex::new(AstManager::new(source_code, tree.clone())));

        let symbol_table_manager = {
            let mut ast_manager = ast_manager.lock().unwrap();
            Arc::new(Mutex::new(SymbolTableManager::new(ast_manager.get_ast())))
        };

        debug!("\nAST:\n{}", ast_manager.lock().unwrap());
        debug!("\nSymbol Table:\n{}", symbol_table_manager.lock().unwrap());

        File {
            uri,
            source_code: source_code.to_string(),
            tree,
            symbol_table_manager,
            ast_manager,
            parser,
        }
    }

    pub fn update(&mut self, changes: Vec<TextDocumentContentChangeEvent>) {
        for change in changes {
            let mut old_tree: Option<&Tree> = None;
            let text: String;

            if let Some(range) = change.range {
                let start_byte = utils::pos_to_byte(range.start, &self.source_code);
                let old_end_byte = utils::pos_to_byte(range.end, &self.source_code);

                let start_position = utils::pos_to_point(range.start);

                let edit = InputEdit {
                    start_byte,
                    old_end_byte: utils::pos_to_byte(range.end, &self.source_code),
                    new_end_byte: start_byte + change.text.len(),
                    start_position,
                    old_end_position: utils::pos_to_point(range.end),
                    new_end_position: utils::calculate_end_point(start_position, &change.text),
                };

                self.source_code
                    .replace_range(start_byte..old_end_byte, &change.text);

                text = self.source_code.clone();
                let tree = &mut self.tree;
                tree.edit(&edit);
                old_tree = Some(tree);
            } else {
                // If change.range is None, change.text represents the whole file
                text = change.text.clone();
            }

            self.tree = self.parser.parse(text, old_tree).unwrap();
        }

        let mut ast_manager = self.ast_manager.lock().unwrap();
        let mut st_manager = self.symbol_table_manager.lock().unwrap();

        ast_manager.update(&self.source_code, self.tree.to_owned());
        st_manager.update(ast_manager.get_ast());

        debug!("\nAST:\n{}", ast_manager);
        debug!("\nSymbol Table:\n{}", st_manager);
    }

    pub fn get_import_paths(&self) -> Vec<Result<(workspace::Import, PathBuf), lsp_types::Range>> {
        let ast = self.ast_manager.lock().unwrap();
        let visit = ast.visit_root();
        let nodes = visit.get_descendants();

        nodes
            .iter()
            .filter_map(|node| match node.get().import {
                Import::Local => {
                    let mut file_name = node.get().content.clone();

                    // TODO: Remove this
                    file_name.remove(0);
                    file_name.remove(file_name.len() - 1);

                    let mut curr_path = self.uri.to_file_path().unwrap();
                    curr_path.pop(); // Get dir

                    curr_path.push(file_name.clone());

                    if curr_path.exists() {
                        Some(Ok((workspace::Import::Local, curr_path)))
                    } else {
                        Some(Err(node.get().range))
                    }
                }
                Import::Library => {
                    let lib_paths = &LanguageDefinition::get().language.library_paths;
                    let file_name = &node.get().content;

                    if let Some(path) = lib_paths.env_variables.iter().find_map(|var| {
                        if let Ok(existing_var) = env::var(var) {
                            if let Ok(mut path) = existing_var.parse::<PathBuf>() {
                                path.push(file_name);
                                if path.exists() {
                                    return Some(path);
                                }
                            }
                        }
                        None
                    }) {
                        return Some(Ok((workspace::Import::Library, path)));
                    }

                    if cfg!(target_os = "windows") {
                        if let Some(path) = utils::find_lib(&lib_paths.windows, file_name) {
                            return Some(Ok((workspace::Import::Library, path)));
                        }
                    } else if cfg!(target_os = "macos") {
                        if let Some(path) = utils::find_lib(&lib_paths.macos, file_name) {
                            return Some(Ok((workspace::Import::Library, path)));
                        }
                    } else if cfg!(target_os = "linux") {
                        if let Some(path) = utils::find_lib(&lib_paths.linux, file_name) {
                            return Some(Ok((workspace::Import::Library, path)));
                        }
                    } else {
                        error!("Unsupported platform for imports, all file imports will fail.");
                    }

                    Some(Err(node.get().range))
                }
                Import::None => None,
            })
            .collect()
    }

    pub fn get_quick_diagnostics(&self) -> Vec<Diagnostic> {
        diagnostics::get_quick_diagnostics(&self.uri, &self.ast_manager, &self.symbol_table_manager)
    }

    pub fn get_full_diagnostics(&self) -> Vec<Diagnostic> {
        diagnostics::get_full_diagnostics(&self.uri, &self.ast_manager, &self.symbol_table_manager)
    }

    pub fn get_completion_list(
        &self,
        position: Position,
        context: Option<CompletionContext>,
    ) -> Option<Vec<CompletionItem>> {
        completion::get_list(
            position,
            &self.ast_manager,
            &self.symbol_table_manager,
            context,
        )
    }

    pub fn get_import_completion_list(&self) -> Vec<CompletionItem> {
        completion::get_imported_list(&self.uri, &self.symbol_table_manager)
    }

    pub fn get_symbol_id_at_pos(&self, position: Position) -> Option<SymbolId> {
        let ast_query = self.ast_manager.lock().unwrap();
        let root_visit = ast_query.visit_root();
        let node = root_visit.get_node_at_position(position)?;

        node.get().linked_symbol.clone()
    }

    pub fn get_semantic_tokens(&self) -> SemanticTokensResult {
        semantic_tokens::get_tokens(
            &self.ast_manager,
            &self.symbol_table_manager,
            &self.tree,
            &self.source_code,
        )
    }

    pub fn get_definition_location(&self, position: Position) -> Option<Location> {
        let range =
            goto::get_definition_range(&self.ast_manager, &self.symbol_table_manager, position)?;
        Some(Location::new(self.uri.clone(), range))
    }

    pub fn rename_symbol(&self, position: Position, new_name: String) -> Option<WorkspaceEdit> {
        rename::rename(
            &self.ast_manager,
            &self.symbol_table_manager,
            self.uri.clone(),
            new_name,
            position,
        )
    }
}
