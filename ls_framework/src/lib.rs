mod language_server;
#[macro_use]
extern crate lazy_static;

mod features;
mod file;
mod language_def;
mod metadata;
mod plugin_manager;
mod settings;
mod utils;
mod workspace;

#[macro_use]
extern crate log;

pub async fn start_server(language_def: &str) {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    language_def::LanguageDefinition::load(language_def);

    let (service, socket) = tower_lsp::LspService::new(language_server::Backend::init);
    tower_lsp::Server::new(stdin, stdout, socket)
        .serve(service)
        .await;
}
