use bumpalo::Bump;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::{analyzer, indexer, parser, resolver};

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
}

impl Backend {
    pub fn new(client: Client) -> Self {
        crate::logging::init(client.clone());
        log::info!("running phppp version {}", env!("CARGO_PKG_VERSION"));
        Self {
            client,
            documents: Arc::new(Mutex::new(HashMap::new())),
            bump: Mutex::new(Bump::new()),
            index: indexer::GlobalIndex::new(),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        log::debug!("initialize called");
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
        log::debug!("shutdown called");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        log::debug!("opened {}", params.text_document.uri);
        self.handle_change(params.text_document.uri.clone(), params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.first() {
            log::debug!("document changed");
            self.handle_change(params.text_document.uri.clone(), change.text.clone())
                .await;
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        log::debug!("goto_definition request");
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
                    log::debug!(
                        "goto_definition found: {} at {:?}",
                        resolved.name,
                        resolved.location
                    );
                    log::debug!("goto_definition: returning definition");
                    return Ok(Some(GotoDefinitionResponse::Scalar(resolved.location)));
                } else {
                    log::debug!("goto_definition: symbol '{}' not resolved", name);
                }
            } else {
                log::debug!(
                    "goto_definition: no symbol found at position {:?}",
                    position
                );
            }
        } else {
            log::debug!("goto_definition: document not found for uri {}", uri);
        }
        log::debug!("goto_definition: returning None");
        log::debug!("goto_definition completed");
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        log::debug!("completion request");
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
        log::debug!("completion returned {} items", items.len());
        log::debug!("completion completed");
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        log::debug!("hover request");
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
                    log::debug!("hover: returning information for {}", resolved.name);
                    return Ok(Some(Hover {
                        contents,
                        range: Some(resolved.location.range),
                    }));
                }
            }
        }
        log::debug!("hover: returning None");
        Ok(None)
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> Result<Option<serde_json::Value>> {
        if params.command == "phppp.restart" {
            log::debug!("Restart requested");
            std::process::exit(0);
        }
        Ok(None)
    }
}

impl Backend {
    async fn handle_change(&self, uri: Url, content: String) {
        log::debug!("indexing {}", uri);

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

        log::debug!("document indexed");
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
