use phppp::server::Backend;
use tower_lsp::LanguageServer;
use tower_lsp::lsp_types::{
    CompletionParams, CompletionResponse, DidOpenTextDocumentParams, GotoDefinitionParams,
    GotoDefinitionResponse, HoverParams, Position, TextDocumentIdentifier, TextDocumentItem,
    TextDocumentPositionParams, Url,
};

#[tokio::test]
async fn goto_definition_basic() {
    let backend = Backend::default();
    let uri = Url::parse("file:///test.php").unwrap();
    let text = "<?php function foo() {}\nfoo();";
    let open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "php".into(),
            version: 1,
            text: text.into(),
        },
    };
    backend.did_open(open).await;

    let params = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 1,
                character: 1,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };
    let resp = backend.goto_definition(params).await.unwrap();
    match resp.unwrap() {
        GotoDefinitionResponse::Scalar(loc) => {
            assert_eq!(loc.range.start.line, 0);
        }
        _ => panic!("unexpected response"),
    }
}

#[tokio::test]
async fn completion_returns_items() {
    let backend = Backend::default();
    let uri = Url::parse("file:///test.php").unwrap();
    let text = "<?php function foo() {}";
    let open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "php".into(),
            version: 1,
            text: text.into(),
        },
    };
    backend.did_open(open).await;

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 0,
                character: 20,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };
    let resp = backend.completion(params).await.unwrap().unwrap();
    let items = match resp {
        CompletionResponse::Array(items) => items,
        _ => panic!("unexpected"),
    };
    assert!(items.iter().any(|i| i.label == "foo"));
}

#[tokio::test]
async fn hover_shows_symbol() {
    let backend = Backend::default();
    let uri = Url::parse("file:///test.php").unwrap();
    let text = "<?php function foo() {}\nfoo();";
    backend
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "php".into(),
                version: 1,
                text: text.into(),
            },
        })
        .await;

    let params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 1,
                character: 1,
            },
        },
        work_done_progress_params: Default::default(),
    };
    let resp = backend.hover(params).await.unwrap().unwrap();
    let contents = match resp.contents {
        tower_lsp::lsp_types::HoverContents::Scalar(s) => s,
        _ => panic!("unexpected"),
    };
    match contents {
        tower_lsp::lsp_types::MarkedString::String(s) => {
            assert!(s.contains("foo"));
        }
        _ => panic!("unexpected"),
    }
}
