use std::env;
use std::sync::RwLock;

use crate::language_def::{self, LanguageDefinition};
use crate::plugin_manager::{self, OnState, PluginManager, PluginsResult};
use crate::project::workspace::{FileManagement, LanguageActions, Workspace};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

extern crate simplelog;

use simplelog::*;

use std::fs::File;

pub struct Backend {
    client: Client,
    workspace: RwLock<Workspace>,
    plugin_manager: RwLock<PluginManager>,
}

impl Backend {
    pub fn init(client: Client, ts_language: tree_sitter::Language) -> Backend {
        Backend {
            client,
            workspace: Workspace::new(ts_language).into(),
            plugin_manager: PluginManager::new().into(),
        }
    }

    pub fn publish_diagnostics(&self, uri: Url, diags: Vec<Diagnostic>) {
        let client = self.client.clone();
        tokio::spawn(async move { client.publish_diagnostics(uri, diags, None).await });
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let log_file_path = env::temp_dir().join(format!(
            "lever-{}.log",
            LanguageDefinition::get().language.name.to_lowercase()
        ));

        if let Ok(log_file) = File::create(log_file_path) {
            let result = WriteLogger::init(LevelFilter::Debug, Config::default(), log_file);

            if result.is_err() {
                self.client
                    .log_message(MessageType::ERROR, "Log file couldn't be created.")
                    .await;
            }
        }

        std::panic::set_hook(Box::new(|info| {
            error!("{info}");
        }));

        if let Some(root_uri) = params.root_uri.clone() {
            self.workspace
                .write()
                .unwrap()
                .set_root_path(root_uri.to_file_path().ok());
        }

        info!(
            "Inititalizing Language Server with options: {:?}",
            params.initialization_options
        );
        if let Some(options) = params.initialization_options {
            self.plugin_manager
                .write()
                .unwrap()
                .load_plugins(params.root_uri, options.to_string().as_str());
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            range: Some(false),
                            legend: language_def::LanguageDefinition::get_semantic_token_legend(),
                            full: Some(SemanticTokensFullOptions::Delta { delta: Some(true) }),
                            ..Default::default()
                        },
                    ),
                ),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![String::from(".")]),
                    ..Default::default()
                }),
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        will_save: Some(false),
                        will_save_wait_until: Some(false),
                        save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                    },
                )),
                definition_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(false),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        info!("Lsp initialized");
    }

    async fn shutdown(&self) -> Result<()> {
        info!("Lsp stopped");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        info!("Opening file: {}", doc.uri);

        let mut diagnostics = {
            let mut workspace = self.workspace.write().unwrap();
            workspace.add_file(doc.uri.clone(), &doc.text);

            workspace.get_full_diagnostics(&doc.uri)
        };

        let mut plugin_result: PluginsResult = self
            .plugin_manager
            .write()
            .unwrap()
            .run_plugins(&doc.uri, &OnState::Save);

        diagnostics.append(&mut plugin_result.diagnostic);

        self.publish_diagnostics(doc.uri, diagnostics);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let diagnostics = {
            let mut workspace = self.workspace.write().unwrap();
            workspace.update_file(&params.text_document.uri, params.content_changes);

            workspace.get_quick_diagnostics(&params.text_document.uri)
        };

        self.publish_diagnostics(params.text_document.uri, diagnostics);
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let mut diagnostics = {
            let workspace = self.workspace.read().unwrap();

            workspace.get_full_diagnostics(&params.text_document.uri)
        };

        let mut plugin_result: PluginsResult = self
            .plugin_manager
            .write()
            .unwrap()
            .run_plugins(&params.text_document.uri, &OnState::Save);

        diagnostics.append(&mut plugin_result.diagnostic);

        for plugin_notification in plugin_result.notification {
            self.client
                .send_notification::<plugin_manager::CustomNotification>(plugin_notification)
                .await;
        }

        self.publish_diagnostics(params.text_document.uri, diagnostics);
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;

        let maybe_location = {
            let workspace = self.workspace.read().unwrap();

            workspace.get_definition_location(&uri, params.text_document_position_params.position)
        };

        maybe_location.map_or(Ok(None), |location| {
            Ok(Some(GotoDefinitionResponse::Scalar(location)))
        })
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let maybe_hover_info = {
            let workspace = self.workspace.read().unwrap();

            workspace.get_hover_info(
                &params.text_document_position_params.text_document.uri,
                params.text_document_position_params.position,
            )
        };

        maybe_hover_info.map_or(Ok(None), |hover_info| {
            Ok(Some(Hover {
                contents: hover_info,
                range: None,
            }))
        })
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let response = {
            let workspace = self.workspace.read().unwrap();

            Ok(workspace.get_semantic_tokens(&params.text_document.uri))
        };

        response
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let completion_list = {
            let workspace = self.workspace.read().unwrap();

            workspace
                .get_completion(
                    &params.text_document_position.text_document.uri,
                    params.text_document_position.position,
                    params.context,
                )
                .unwrap_or_default()
        };

        Ok(Some(CompletionResponse::Array(completion_list)))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let response = {
            let mut workspace = self.workspace.write().unwrap();

            Ok(workspace.rename_symbol(
                &params.text_document_position.text_document.uri,
                params.text_document_position.position,
                params.new_name,
            ))
        };

        response
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        let mut workspace = self.workspace.write().unwrap();
        workspace.update_settings(params.settings);
    }
}
