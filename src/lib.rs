#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::cast_possible_truncation, clippy::wildcard_imports)]
mod language_def;
mod language_server;
mod lsp_mappings;
mod plugin_manager;
mod project;
mod settings;
mod utils;

#[macro_use]
extern crate log;

pub async fn start_server(language_def: &str, ts_language: tree_sitter::Language) {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    language_def::LanguageDefinition::load(language_def);

    let (service, socket) =
        tower_lsp::LspService::new(|client| language_server::Backend::init(client, ts_language));
    tower_lsp::Server::new(stdin, stdout, socket)
        .serve(service)
        .await;
}
