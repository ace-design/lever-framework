use super::ast::{Ast, Visitable};

use crate::language_def;
use crate::project::metadata::NodeKind;

use indextree::{Arena, NodeId};
use std::fmt;
use tower_lsp::lsp_types::{Position, Range};

use super::{symbol::Usage, Node, Symbol, SymbolId};

pub type ScopeId = NodeId;

#[derive(Debug, Default, Clone)]
pub struct SymbolTable {
    arena: Arena<ScopeSymbolTable>,
    pub root_id: Option<ScopeId>,
    undefined_list: Vec<(String, Range)>,
}

pub trait Actions {
    fn get_symbol(&self, id: SymbolId) -> Option<&Symbol>;
    fn get_symbol_mut(&mut self, id: SymbolId) -> Option<&mut Symbol>;
    fn get_all_symbols(&self) -> Vec<Symbol>;
    fn get_symbols_in_scope_at_pos(&self, position: Position) -> Vec<Symbol>;
    fn get_symbols_at_root(&self) -> Vec<Symbol>;
    fn get_symbols_in_scope(&self, scope_id: ScopeId) -> Vec<Symbol>;
    fn get_top_level_symbols(&self) -> Vec<Symbol>;
    fn get_symbol_at_pos(&self, name: String, position: Position) -> Option<&Symbol>;
    fn rename_symbol(&mut self, id: usize, new_name: String);
    fn get_unlinked_symbols(&self) -> Vec<(String, Range)>;
}

impl Actions for SymbolTable {
    fn get_symbol(&self, id: SymbolId) -> Option<&Symbol> {
        // Assumes file id is correct
        let scope_table = self.arena.get(id.symbol_table_id)?.get();
        scope_table.symbols.get(id.index)
    }

    fn get_symbol_mut(&mut self, id: SymbolId) -> Option<&mut Symbol> {
        let scope_table = self.arena.get_mut(id.symbol_table_id)?.get_mut();
        scope_table.symbols.get_mut(id.index)
    }

    fn get_all_symbols(&self) -> Vec<Symbol> {
        let mut symbols: Vec<Symbol> = Vec::new();

        for child_id in self.root_id.unwrap().descendants(&self.arena) {
            symbols.append(&mut self.arena.get(child_id).unwrap().get().symbols.clone());
        }

        symbols
    }

    fn get_symbols_in_scope_at_pos(&self, position: Position) -> Vec<Symbol> {
        let mut current_scope_id = self.root_id.unwrap();
        let mut symbols: Vec<Symbol>;
        symbols = self
            .arena
            .get(current_scope_id)
            .unwrap()
            .get()
            .symbols
            .clone();

        let mut subscope_exists = true;
        while subscope_exists {
            subscope_exists = false;

            for child_id in current_scope_id.children(&self.arena) {
                let scope = self.arena.get(child_id).unwrap().get();
                if scope.range.start < position && position < scope.range.end {
                    current_scope_id = child_id;
                    subscope_exists = true;

                    let mut scope_symbols = scope.symbols.clone();
                    scope_symbols.retain(|s| s.def_range.end < position);
                    symbols.append(&mut scope_symbols);
                    break;
                }
            }
        }

        symbols
    }

    fn get_symbols_at_root(&self) -> Vec<Symbol> {
        if let Some(root_id) = self.root_id {
            self.arena.get(root_id).unwrap().get().symbols.clone()
        } else {
            vec![]
        }
    }

    fn get_symbols_in_scope(&self, scope_id: ScopeId) -> Vec<Symbol> {
        self.arena.get(scope_id).unwrap().get().symbols.clone()
    }

    fn get_top_level_symbols(&self) -> Vec<Symbol> {
        self.arena
            .get(self.root_id.unwrap())
            .unwrap()
            .get()
            .symbols
            .clone()
    }

    fn get_symbol_at_pos(&self, name: String, position: Position) -> Option<&Symbol> {
        let scope_id = self.get_scope_id(position)?;

        for pre_id in scope_id.predecessors(&self.arena) {
            let scope = self.arena.get(pre_id)?.get();

            if let Some(symbol) = scope.symbols.iter().find(|s| s.name == name) {
                return Some(symbol);
            }
        }

        None
    }

    fn rename_symbol(&mut self, id: usize, new_name: String) {
        for scope in self.arena.iter_mut() {
            if let Some(symbol) = scope.get_mut().symbols.get_mut(id) {
                symbol.name = new_name;
                break;
            }
        }
    }

    fn get_unlinked_symbols(&self) -> Vec<(String, Range)> {
        self.undefined_list.clone()
    }
}

impl SymbolTable {
    pub fn new(ast: &mut Ast) -> SymbolTable {
        let mut table = SymbolTable::default();

        table.root_id = Some(table.parse_scope(ast.visit_root().get_id(), ast.get_arena()));
        table.parse_usages(ast.get_arena());
        table.parse_types(ast.visit_root().get_id(), ast.get_arena());
        table.parse_member_usages(ast.visit_root().get_id(), ast.get_arena());

        table
    }

    fn get_scope_id(&self, position: Position) -> Option<ScopeId> {
        self._get_scope_id(position, self.root_id?)
    }

    fn _get_scope_id(&self, position: Position, current: ScopeId) -> Option<ScopeId> {
        for child_scope_id in current.children(&self.arena) {
            let child_scope = self.arena.get(child_scope_id).unwrap().get();

            if position >= child_scope.range.start && position <= child_scope.range.end {
                if let Some(scope_id) = self._get_scope_id(position, child_scope_id) {
                    if scope_id.to_string() == self.root_id?.to_string() {
                        return Some(child_scope_id);
                    }
                    return Some(scope_id);
                }
                return Some(child_scope_id);
            }
        }
        self.root_id
    }

    fn parse_scope(&mut self, node_id: NodeId, ast_arena: &mut Arena<Node>) -> ScopeId {
        let table = ScopeSymbolTable::new(ast_arena.get(node_id).unwrap().get().range);
        let current_table_node_id = self.arena.new_node(table);

        let mut queue: Vec<NodeId> = node_id.children(ast_arena).collect();

        while let Some(node_id) = queue.pop() {
            let symbol_index = if let crate::language_def::Symbol::Init {
                kind,
                name_node,
                type_node: _,
            } = &ast_arena.get(node_id).unwrap().get().symbol
            {
                debug!(
                    "{:?} {}",
                    ast_arena.get(node_id).unwrap().get().kind,
                    node_id.children(ast_arena).count()
                );
                let name_node_id = node_id
                    .children(ast_arena)
                    .find(|id| {
                        debug!("{:?} {}", ast_arena.get(*id).unwrap().get().kind, name_node);
                        ast_arena.get(*id).unwrap().get().kind == NodeKind::Node(name_node.clone())
                    })
                    .unwrap();

                let name_node = ast_arena.get(name_node_id).unwrap().get();

                let symbol = Symbol::new(name_node.content.clone(), kind.clone(), name_node.range);

                let symbols = &mut self
                    .arena
                    .get_mut(current_table_node_id)
                    .unwrap()
                    .get_mut()
                    .symbols;
                symbols.push(symbol);

                let index = symbols.len() - 1;
                ast_arena
                    .get_mut(name_node_id)
                    .unwrap()
                    .get_mut()
                    .link(current_table_node_id, index);

                Some(index)
            } else {
                None
            };

            if ast_arena.get(node_id).unwrap().get().kind.is_scope_node() {
                let subtable = self.parse_scope(node_id, ast_arena);

                if let Some(i) = symbol_index {
                    self.arena
                        .get_mut(current_table_node_id)
                        .unwrap()
                        .get_mut()
                        .symbols[i]
                        .field_scope_id = Some(subtable);
                }

                current_table_node_id.append(subtable, &mut self.arena);
            } else {
                queue.append(&mut node_id.children(ast_arena).collect());
            }
        }

        current_table_node_id
    }

    fn parse_usages(&mut self, arena: &mut Arena<Node>) {
        for node in arena
            .iter_mut()
            .filter(|node| matches!(node.get().symbol, language_def::Symbol::Usage))
        {
            let node = node.get_mut();
            let symbol_name = &node.content;

            let scope_id = self.get_scope_id(node.range.start).unwrap();
            let scope_ids: Vec<NodeId> = scope_id.predecessors(&self.arena).collect();

            let mut found = false;
            for id in scope_ids {
                if let Some(index) = self
                    .arena
                    .get(id)
                    .unwrap()
                    .get()
                    .symbols
                    .iter()
                    .position(|s| &s.name == symbol_name)
                {
                    let symbol = &mut self.arena.get_mut(id).unwrap().get_mut().symbols[index];
                    node.link(id, index);
                    found = true;
                    symbol.usages.push(Usage::new_local(node.range));
                    break;
                }
            }

            if !found {
                self.undefined_list.push((node.content.clone(), node.range));
            }
        }
    }

    fn parse_types(&mut self, root_id: NodeId, ast_arena: &mut Arena<Node>) {
        for node_id in root_id.descendants(ast_arena) {
            if let language_def::Symbol::Init {
                kind,
                name_node,
                type_node: Some(type_node_query),
            } = ast_arena.get(node_id).unwrap().get().symbol.clone()
            {
                if let Some(type_node_id) = node_id.children(ast_arena).find(|id| {
                    ast_arena.get(*id).unwrap().get().kind
                        == NodeKind::Node(type_node_query.clone())
                }) {
                    if let Some(symbol_id) = ast_arena
                        .get(type_node_id)
                        .unwrap()
                        .get()
                        .linked_symbol
                        .clone()
                    {
                        let name_node_id = node_id
                            .children(ast_arena)
                            .find(|id| {
                                ast_arena.get(*id).unwrap().get().kind
                                    == NodeKind::Node(name_node.clone())
                            })
                            .unwrap();

                        if let Some(name_symbol_id) = ast_arena
                            .get(name_node_id)
                            .unwrap()
                            .get()
                            .linked_symbol
                            .clone()
                        {
                            self.get_symbol_mut(name_symbol_id).unwrap().type_symbol =
                                Some(symbol_id);
                        }
                    }
                } else {
                    error!("Failed to parse type of symbol {kind}. This is caused by a problem within the Lever rules file.");
                }
            }
        }
    }

    fn parse_member_usages(&mut self, root_id: NodeId, arena: &mut Arena<Node>) {
        let ids: Vec<NodeId> = root_id
            .descendants(arena)
            .filter(|id| {
                matches!(
                    arena.get(*id).unwrap().get().symbol,
                    language_def::Symbol::MemberUsage
                )
            })
            .collect();

        for id in ids {
            if let Some(previous_sibling_id) = arena.get(id).unwrap().previous_sibling() {
                let previous_sibling = arena.get(previous_sibling_id).unwrap().get();
                match previous_sibling.symbol {
                    language_def::Symbol::Usage => {
                        if let Some(symbol_id) = previous_sibling.linked_symbol.clone() {
                            let parent_symbol = self.get_symbol(symbol_id).unwrap();

                            if let Some(parent_type_symbol_id) = parent_symbol.type_symbol.clone() {
                                let parent_type_symbol =
                                    self.get_symbol(parent_type_symbol_id).unwrap();

                                if let Some(field_scope_id) = parent_type_symbol.field_scope_id {
                                    let scope_table =
                                        self.arena.get_mut(field_scope_id).unwrap().get_mut();

                                    if let Some(member_symbol_index) =
                                        scope_table.symbols.iter().position(|s| {
                                            s.name == arena.get(id).unwrap().get().content
                                        })
                                    {
                                        arena
                                            .get_mut(id)
                                            .unwrap()
                                            .get_mut()
                                            .link(field_scope_id, member_symbol_index);
                                        scope_table
                                            .symbols
                                            .get_mut(member_symbol_index)
                                            .unwrap()
                                            .usages
                                            .push(Usage::new_local(
                                                arena.get(id).unwrap().get().range,
                                            ));
                                    }
                                }
                            }
                        }
                    }
                    language_def::Symbol::Expression => {
                        if let Some(previous_symbol_id) =
                            previous_sibling_id.children(arena).find(|id| {
                                matches!(
                                    arena.get(*id).unwrap().get().symbol,
                                    language_def::Symbol::MemberUsage
                                )
                            })
                        {
                            if let Some(previous_sibling) = arena.get(previous_symbol_id) {
                                if let Some(symbol_id) =
                                    previous_sibling.get().linked_symbol.clone()
                                {
                                    let parent_symbol = self.get_symbol(symbol_id).unwrap();

                                    if let Some(parent_type_symbol_id) =
                                        parent_symbol.type_symbol.clone()
                                    {
                                        let parent_type_symbol =
                                            self.get_symbol(parent_type_symbol_id).unwrap();

                                        if let Some(field_scope_id) =
                                            parent_type_symbol.field_scope_id
                                        {
                                            let scope_table = self
                                                .arena
                                                .get_mut(field_scope_id)
                                                .unwrap()
                                                .get_mut();

                                            if let Some(member_symbol_index) =
                                                scope_table.symbols.iter().position(|s| {
                                                    s.name == arena.get(id).unwrap().get().content
                                                })
                                            {
                                                arena
                                                    .get_mut(id)
                                                    .unwrap()
                                                    .get_mut()
                                                    .link(field_scope_id, member_symbol_index);
                                                scope_table
                                                    .symbols
                                                    .get_mut(member_symbol_index)
                                                    .unwrap()
                                                    .usages
                                                    .push(Usage::new_local(
                                                        arena.get(id).unwrap().get().range,
                                                    ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

impl fmt::Display for SymbolTable {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut output = String::new();

        let mut sorted = self.arena.iter().collect::<Vec<_>>();
        sorted.sort_by(|a, b| a.get().range.start.cmp(&b.get().range.start));

        for node in sorted {
            output.push_str(format!("{}\n", node.get()).as_str());
        }

        fmt.write_str(&output)
    }
}

#[derive(Debug, Default, Clone)]
struct ScopeSymbolTable {
    range: Range,
    symbols: Vec<Symbol>,
}

impl ScopeSymbolTable {
    fn new(range: Range) -> ScopeSymbolTable {
        ScopeSymbolTable {
            range,
            ..Default::default()
        }
    }
}

impl fmt::Display for ScopeSymbolTable {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut output = String::from("\n");

        output.push_str(
            format!(
                "{0: <10} | {1: <15} | {2: <10} | {3: <10} | {4: <10}\n",
                "symbol", "name", "position", "usages", "fields"
            )
            .as_str(),
        );

        output.push_str("-".repeat(62).as_str());
        output.push('\n');

        for s in &self.symbols {
            output.push_str(&s.to_string());
        }

        fmt.write_str(&output)
    }
}
