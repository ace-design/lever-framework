#![warn(clippy::all)]
#![allow(clippy::cast_possible_truncation, clippy::wildcard_imports)]
mod language_def;
mod language_server;
mod lsp_mappings;
mod plugin_manager;
mod project;
mod settings;
mod setup;
mod utils;

#[macro_use]
extern crate log;

pub use language_def::*;
pub use lsp_mappings::*;
pub use project::{Ast, Node, NodeKind, Translator};
pub use setup::*;

pub async fn start_server(setup: &Setup) {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    language_def::LanguageDefinition::load(&setup.language_def);

    let (service, socket) = tower_lsp::LspService::new(|client| {
        language_server::Backend::init(client, setup.treesitter_language)
    });
    tower_lsp::Server::new(stdin, stdout, socket)
        .serve(service)
        .await;
}
