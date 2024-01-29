use core::fmt;
use std::fmt::Debug;

use super::symbol_table::{ScopeId, SymbolId, SymbolTable};
use super::{Ast, Symbol};

use indextree::NodeId;
use tower_lsp::lsp_types::{Position, Range};

use crate::metadata::symbol_table::SymbolTableActions;

pub trait SymbolTableEditor {
    fn update(&mut self, ast: &mut Ast);
}

pub trait SymbolTableQuery {
    fn get_symbols_at_pos(&self, position: Position) -> Vec<Symbol>;
    fn get_symbols_at_root(&self) -> (Vec<Symbol>, NodeId);
    fn get_symbols_in_scope(&self, scope_id: ScopeId) -> Vec<Symbol>;
    fn get_symbol_at_pos(&self, name: String, position: Position) -> Option<&Symbol>;
    fn get_all_symbols(&self) -> Vec<Symbol>;
    fn get_symbol(&self, symbol_id: SymbolId) -> Option<&Symbol>;
    fn get_unlinked_symbols(&self) -> Vec<(String, Range)>;
}

#[derive(Debug, Clone)]
pub struct SymbolTableManager {
    symbol_table: SymbolTable,
}

impl SymbolTableManager {
    pub fn new(ast: &mut Ast) -> SymbolTableManager {
        let symbol_table = SymbolTable::new(ast);
        SymbolTableManager { symbol_table }
    }
}

impl fmt::Display for SymbolTableManager {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(&self.symbol_table.to_string())
    }
}

impl SymbolTableQuery for SymbolTableManager {
    fn get_symbols_at_pos(&self, position: Position) -> Vec<Symbol> {
        self.symbol_table.get_symbols_in_scope_at_pos(position)
    }

    fn get_symbols_at_root(&self) -> (Vec<Symbol>, NodeId) {
        (
            self.symbol_table.get_symbols_at_root(),
            self.symbol_table.root_id.unwrap(),
        )
    }

    fn get_symbols_in_scope(&self, scope_id: ScopeId) -> Vec<Symbol> {
        self.symbol_table.get_symbols_in_scope(scope_id)
    }

    fn get_symbol_at_pos(&self, name: String, position: Position) -> Option<&Symbol> {
        self.symbol_table.get_symbol_at_pos(name, position)
    }

    fn get_all_symbols(&self) -> Vec<Symbol> {
        self.symbol_table.get_all_symbols()
    }

    fn get_symbol(&self, symbol_id: SymbolId) -> Option<&Symbol> {
        self.symbol_table.get_symbol(symbol_id)
    }

    fn get_unlinked_symbols(&self) -> Vec<(String, Range)> {
        self.symbol_table.get_unlinked_symbols()
    }
}

impl SymbolTableEditor for SymbolTableManager {
    fn update(&mut self, ast: &mut Ast) {
        *self = SymbolTableManager::new(ast)
    }
}
