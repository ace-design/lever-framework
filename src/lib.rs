pub use lever_core;
pub use lever_gen;

pub use indextree;
pub use tree_sitter;

pub async fn start_server(setup: &lever_core::Setup) {
    lever_core::start_server(setup).await
}
