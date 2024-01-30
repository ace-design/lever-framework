use core::fmt;

use tower_lsp::lsp_types::Range;

use super::ast::VisitNode;

use super::{Ast, SymbolId};

pub trait AstEditor {
    fn update(&mut self, content: &str, syntax_tree: tree_sitter::Tree);
    fn link_symbol(&mut self, symbol_id: SymbolId, range: Range);
}

pub trait AstQuery {
    fn visit_root(&self) -> VisitNode;
}

#[derive(Debug, Clone)]
pub struct AstManager {
    pub ast: Ast,
}

impl AstManager {
    pub fn new(source_code: &str, tree: tree_sitter::Tree) -> AstManager {
        let ast = Ast::new(source_code, tree).unwrap();
        AstManager { ast }
    }

    pub fn get_ast(&mut self) -> &mut Ast {
        &mut self.ast
    }
}

impl fmt::Display for AstManager {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(&self.ast.to_string())
    }
}

impl AstQuery for AstManager {
    fn visit_root(&self) -> VisitNode {
        self.ast.visit_root()
    }
}

impl AstEditor for AstManager {
    fn update(&mut self, content: &str, syntax_tree: tree_sitter::Tree) {
        *self = AstManager::new(content, syntax_tree);
    }

    fn link_symbol(&mut self, symbol_id: SymbolId, range: Range) {
        self.ast.link_symbol(symbol_id, range);
    }
}
