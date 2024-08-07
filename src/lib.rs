pub use lever_core::{Ast, HighlightType, Import, Node, NodeKind, Setup, Symbol, Translator};
pub use lever_gen::{rules_translator, start_server};
pub use {indextree, tokio, tree_sitter};

pub use crate::indextree::NodeId;

pub async fn start_server(setup: &lever_core::Setup) {
    lever_core::start_server(setup).await
}
