use bumpalo::Bump;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{LanguageServer, LspService, Server};

use phppp::{analyzer, indexer, parser};

#[derive(Default)]
struct DocumentState {
    text: String,
    ast: Option<parser::Ast>,
    symbols: indexer::FileSymbols,
}

struct Backend {
    documents: Arc<Mutex<HashMap<Url, DocumentState>>>,
    bump: Mutex<Bump>, // Shared arena allocator guarded by mutex
    index: indexer::GlobalIndex,
}

impl Default for Backend {
    fn default() -> Self {
        Self {
            documents: Arc::new(Mutex::new(HashMap::new())),
            bump: Mutex::new(Bump::new()),
            index: indexer::GlobalIndex::new(),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions::default()),
                definition_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
            ..InitializeResult::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        // Optionally start file watcher here
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.handle_change(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.first() {
            self.handle_change(params.text_document.uri, change.text.clone())
                .await;
        }
    }
}

impl Backend {
    async fn handle_change(&self, uri: Url, content: String) {
        let mut docs = self.documents.lock().unwrap();
        let bump = self.bump.lock().unwrap();
        let ast = parser::parse_php(&content, &bump); // parsed with tree-sitter
        let symbols = indexer::extract_symbols(&content, &ast, &uri);

        self.index.insert(uri.clone(), symbols.clone());

        analyzer::resolve_types_parallel(&symbols);

        docs.insert(
            uri,
            DocumentState {
                text: content,
                ast: Some(ast),
                symbols,
            },
        );
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let backend = Backend::default();
    let (service, socket) = LspService::new(|_| backend);
    Server::new(stdin, stdout, socket).serve(service).await;
}
