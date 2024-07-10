use petgraph::prelude::NodeIndex;
use tower_lsp::lsp_types::Range;

use super::symbol_table::ScopeId;

#[derive(Debug, PartialEq, Clone)]
pub struct SymbolId {
    pub file_id: Option<petgraph::prelude::NodeIndex>,
    pub symbol_table_id: ScopeId,
    pub index: usize,
}

impl SymbolId {
    pub const fn new(
        file_id: Option<petgraph::prelude::NodeIndex>, // file_id only present if symbol is defined in a different file
        symbol_table_id: ScopeId,
        index: usize,
    ) -> Self {
        Self {
            file_id,
            symbol_table_id,
            index,
        }
    }

    pub const fn get_file_id(&self) -> Option<petgraph::prelude::NodeIndex> {
        self.file_id
    }
}

#[derive(Debug, Clone)]
pub struct Usage {
    pub file_id: Option<NodeIndex>,
    pub range: Range,
}

impl Usage {
    pub const fn new_external(file_id: NodeIndex, range: Range) -> Usage {
        Usage {
            file_id: Some(file_id),
            range,
        }
    }

    pub const fn new_local(range: Range) -> Usage {
        Usage {
            file_id: None,
            range,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: String,
    pub type_symbol: Option<SymbolId>,
    pub def_range: Range,
    pub usages: Vec<Usage>,
    pub field_scope_id: Option<ScopeId>,
}

impl Symbol {
    pub const fn new(name: String, kind: String, def_position: Range) -> Symbol {
        Symbol {
            name,
            kind,
            type_symbol: None,
            def_range: def_position,
            usages: vec![],
            field_scope_id: None,
        }
    }

    pub fn add_usage(&mut self, usage: Usage) {
        self.usages.push(usage);
    }
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str(
            format!(
                "{0: <10} | {1: <15} | {2: <10} | {3: <10} | {4: <10}\n",
                self.kind,
                self.name,
                format!(
                    "l:{} c:{}",
                    self.def_range.start.line, self.def_range.start.character
                ),
                self.usages.len(),
                0 // TODO
            )
            .as_str(),
        )
    }
}
