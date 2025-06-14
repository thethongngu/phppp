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
        Self {
            client,
            documents: Arc::new(Mutex::new(HashMap::new())),
            bump: Mutex::new(Bump::new()),
            index: indexer::GlobalIndex::new(),
        }
    }

    async fn log(&self, message: impl Into<String>) {
        let _ = self
            .client
            .log_message(MessageType::LOG, message.into())
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        self.log("initialize called").await;
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
        self.log("shutdown called").await;
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.log(format!("opened {}", params.text_document.uri))
            .await;
        self.handle_change(params.text_document.uri.clone(), params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.first() {
            self.log("document changed").await;
            self.handle_change(params.text_document.uri.clone(), change.text.clone())
                .await;
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        self.log("goto_definition request").await;
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
                    return Ok(Some(GotoDefinitionResponse::Scalar(resolved.location)));
                }
            }
        }
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        self.log("completion request").await;
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
        self.log(format!("completion returned {} items", items.len()))
            .await;
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        self.log("hover request").await;
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
                    return Ok(Some(Hover {
                        contents,
                        range: Some(resolved.location.range),
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
            self.log("Restart requested").await;
            std::process::exit(0);
        }
        Ok(None)
    }
}

impl Backend {
    async fn handle_change(&self, uri: Url, content: String) {
        self.log(format!("indexing {}", uri)).await;

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

        self.log("document indexed").await;
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
