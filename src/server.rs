use bumpalo::Bump;
use notify::RecommendedWatcher;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::{
    analyzer, composer, config, fs, indexer, laravel::LaravelPlugin, parser, plugin::PluginManager,
    resolver,
};

#[derive(Default, Clone)]
pub struct DocumentState {
    pub text: String,
    pub ast: Option<parser::Ast>,
    pub symbols: indexer::FileSymbols,
}

pub struct Backend {
    client: Client,
    documents: Arc<Mutex<HashMap<Url, DocumentState>>>,
    bump: Mutex<Bump>,
    index: indexer::GlobalIndex,
    watcher: Mutex<Option<RecommendedWatcher>>,
    config: config::Config,
    autoload: HashMap<String, String>,
    plugins: PluginManager,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        crate::logging::init(client.clone());
        crate::metrics::init();
        tracing::info!("running phppp version {}", env!("CARGO_PKG_VERSION"));
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let config = config::load_config(&cwd).unwrap_or_default();
        let autoload = composer::load_autoload_paths(&cwd).unwrap_or_default();
        let mut plugins = PluginManager::new();
        if config.enable_laravel {
            plugins.add(LaravelPlugin);
        }
        Self {
            client,
            documents: Arc::new(Mutex::new(HashMap::new())),
            bump: Mutex::new(Bump::new()),
            index: indexer::new_index(),
            watcher: Mutex::new(None),
            config,
            autoload,
            plugins,
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        tracing::debug!("initialize called");
        if let Some(root) = params.root_uri.and_then(|u| u.to_file_path().ok()) {
            if let Err(e) = indexer::scan_workspace(&root, &self.index) {
                tracing::error!("workspace scan failed: {}", e);
                crate::metrics::inc_error("initialize");
            }
            let idx = self.index.clone();
            if let Ok(w) = fs::watch(&root, move |res| {
                if let Ok(ev) = res {
                    for p in ev.paths {
                        let _ = indexer::index_file(&p, &idx);
                    }
                }
            }) {
                *self.watcher.lock().unwrap() = Some(w);
            }
            self.plugins.register_all(&self.index);
        }
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions::default()),
                definition_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["phppp.restart".into()],
                    work_done_progress_options: Default::default(),
                }),
                ..ServerCapabilities::default()
            },
            ..InitializeResult::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        tracing::debug!("shutdown called");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        tracing::debug!("opened {}", params.text_document.uri);
        self.handle_change(params.text_document.uri.clone(), params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.first() {
            tracing::debug!("document changed");
            self.handle_change(params.text_document.uri.clone(), change.text.clone())
                .await;
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let _timer = crate::metrics::Timer::new("goto_definition");
        tracing::debug!("goto_definition request");
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        if let Some(doc) = self.get_document(&uri) {
            if let Some(name) = self.symbol_at_position(&doc, position) {
                if let Some(resolved) = resolver::resolve_symbol(
                    &name,
                    &uri,
                    position,
                    &doc.text,
                    doc.ast.as_ref().unwrap(),
                    &doc.symbols,
                    &self.index,
                ) {
                    tracing::debug!(
                        "goto_definition found: {} at {:?}",
                        resolved.name,
                        resolved.location
                    );
                    tracing::debug!("goto_definition: returning definition");
                    return Ok(Some(GotoDefinitionResponse::Scalar(resolved.location)));
                } else {
                    tracing::debug!("goto_definition: symbol '{}' not resolved", name);
                }
            } else {
                tracing::debug!(
                    "goto_definition: no symbol found at position {:?}",
                    position
                );
            }
        } else {
            tracing::debug!("goto_definition: document not found for uri {}", uri);
        }
        tracing::debug!("goto_definition: returning None");
        tracing::debug!("goto_definition completed");
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let _timer = crate::metrics::Timer::new("completion");
        tracing::debug!("completion request");
        let uri = params.text_document_position.text_document.uri;
        let mut items = Vec::new();
        if let Some(doc) = self.get_document(&uri) {
            for sym in doc.symbols.values() {
                items.push(CompletionItem {
                    label: sym.name.clone(),
                    kind: Some(map_completion_kind(&sym.kind)),
                    ..CompletionItem::default()
                });
            }
        }
        for entry in self.index.iter() {
            for sym in entry.value().values() {
                items.push(CompletionItem {
                    label: sym.name.clone(),
                    kind: Some(map_completion_kind(&sym.kind)),
                    ..CompletionItem::default()
                });
            }
        }
        tracing::debug!("completion returned {} items", items.len());
        tracing::debug!("completion completed");
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let _timer = crate::metrics::Timer::new("hover");
        tracing::debug!("hover request");
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        if let Some(doc) = self.get_document(&uri) {
            if let Some(name) = self.symbol_at_position(&doc, position) {
                if let Some(resolved) = resolver::resolve_symbol(
                    &name,
                    &uri,
                    position,
                    &doc.text,
                    doc.ast.as_ref().unwrap(),
                    &doc.symbols,
                    &self.index,
                ) {
                    let contents = HoverContents::Scalar(MarkedString::String(format!(
                        "{} {:?}",
                        resolved.name, resolved.kind
                    )));
                    tracing::debug!("hover: returning information for {}", resolved.name);
                    return Ok(Some(Hover {
                        contents,
                        range: Some(resolved.location.range),
                    }));
                }
            }
        }
        tracing::debug!("hover: returning None");
        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        if let Some(doc) = self.get_document(&uri) {
            if let Some(name) = self.symbol_at_position(&doc, pos) {
                let mut out = Vec::new();
                let docs = self.documents.lock().unwrap();
                for (u, d) in docs.iter() {
                    for (i, line) in d.text.lines().enumerate() {
                        for m in line.match_indices(&name) {
                            out.push(Location {
                                uri: u.clone(),
                                range: Range {
                                    start: Position {
                                        line: i as u32,
                                        character: m.0 as u32,
                                    },
                                    end: Position {
                                        line: i as u32,
                                        character: m.0 as u32 + name.len() as u32,
                                    },
                                },
                            });
                        }
                    }
                }
                if !out.is_empty() {
                    return Ok(Some(out));
                }
            }
        }
        Ok(None)
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let new_name = params.new_name;
        if let Some(doc) = self.get_document(&uri) {
            if let Some(name) = self.symbol_at_position(&doc, pos) {
                let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
                let docs = self.documents.lock().unwrap();
                for (u, d) in docs.iter() {
                    for (i, line) in d.text.lines().enumerate() {
                        for m in line.match_indices(&name) {
                            changes.entry(u.clone()).or_default().push(TextEdit {
                                range: Range {
                                    start: Position {
                                        line: i as u32,
                                        character: m.0 as u32,
                                    },
                                    end: Position {
                                        line: i as u32,
                                        character: m.0 as u32 + name.len() as u32,
                                    },
                                },
                                new_text: new_name.clone(),
                            });
                        }
                    }
                }
                if !changes.is_empty() {
                    return Ok(Some(WorkspaceEdit {
                        changes: Some(changes),
                        ..WorkspaceEdit::default()
                    }));
                }
            }
        }
        Ok(None)
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> Result<Option<serde_json::Value>> {
        if params.command == "phppp.restart" {
            tracing::debug!("Restart requested");
            std::process::exit(0);
        }
        Ok(None)
    }
}

impl Backend {
    async fn handle_change(&self, uri: Url, content: String) {
        let _timer = crate::metrics::Timer::new("handle_change");
        tracing::debug!("indexing {}", uri);

        let ast = {
            let bump = self.bump.lock().unwrap();
            parser::parse_php(&content, &bump)
        };
        let symbols = indexer::extract_symbols(&content, &ast, &uri);
        self.index.insert(uri.clone(), symbols.clone());
        analyzer::resolve_types_parallel(&symbols);

        {
            let mut docs = self.documents.lock().unwrap();
            docs.insert(
                uri,
                DocumentState {
                    text: content,
                    ast: Some(ast),
                    symbols,
                },
            );
        }

        tracing::debug!("document indexed");
    }

    fn get_document(&self, uri: &Url) -> Option<DocumentState> {
        self.documents.lock().unwrap().get(uri).cloned()
    }

    fn symbol_at_position(&self, doc: &DocumentState, pos: Position) -> Option<String> {
        let ast = doc.ast.as_ref()?;
        let root = ast.0.root_node();
        let point = tree_sitter::Point {
            row: pos.line as usize,
            column: pos.character as usize,
        };
        let mut node = root.descendant_for_point_range(point, point)?;
        while node.kind() != "name"
            && node.kind() != "qualified_name"
            && node.kind() != "variable_name"
        {
            if let Some(parent) = node.parent() {
                node = parent;
            } else {
                break;
            }
        }
        if node.kind() == "name"
            || node.kind() == "qualified_name"
            || node.kind() == "variable_name"
        {
            return node
                .utf8_text(doc.text.as_bytes())
                .ok()
                .map(|s| s.to_string());
        }
        None
    }
}

fn map_completion_kind(kind: &indexer::SymbolKind) -> CompletionItemKind {
    match kind {
        indexer::SymbolKind::Function => CompletionItemKind::FUNCTION,
        indexer::SymbolKind::Class => CompletionItemKind::CLASS,
        indexer::SymbolKind::Constant => CompletionItemKind::CONSTANT,
        indexer::SymbolKind::Variable => CompletionItemKind::VARIABLE,
    }
}

pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
