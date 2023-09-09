use std::collections::HashMap;
use std::sync::Mutex;
mod completions;
mod position;

use crate::lexer;
use crate::lexer::locations::{GetSpan, Location};
use crate::parser::ast::{self, Program};
use crate::parser::{self, Parser};
use completions::*;
use tower_lsp::jsonrpc::Result;
use tower_lsp::{lsp_types::*, LspService, Server};
use tower_lsp::{Client, LanguageServer};

use self::position::ContainsPosition;

trait IntoPosition {
    fn into_position(self) -> Position;
}

impl IntoPosition for Location {
    fn into_position(self) -> Position {
        Position {
            line: self.line as u32,
            character: self.col as u32,
        }
    }
}

impl IntoPosition for lexer::locations::Position {
    fn into_position(self) -> Position {
        Position {
            line: self.line as u32,
            character: self.col as u32,
        }
    }
}

#[derive(Debug)]
struct Backend {
    pub client: Client,
    pub documents: TextDocuments,
}

#[derive(Debug)]
struct TextDocuments {
    pub inner: Mutex<HashMap<Url, String>>,
}

impl TextDocuments {
    fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    fn get(&self, uri: Url) -> Option<String> {
        match self.inner.lock() {
            Ok(map) => map.get(&uri).cloned(),
            Err(_) => None,
        }
    }

    fn put(&self, url: Url, text: String) {
        if let Ok(mut map) = self.inner.lock() {
            map.insert(url, text);
        }
    }
}

struct ChangedDocumentItem {
    pub uri: Url,

    pub version: i32,

    pub text: String,
}

impl Backend {
    async fn on_change(&self, params: ChangedDocumentItem) {
        let program = Parser::new(&params.text).parse();

        let mut diagnostics = vec![];

        for err in program.errors().iter() {
            let range = Range {
                start: match &err.inner_error {
                    parser::error::ParseError::ExpectedToken { found, .. }
                    | parser::error::ParseError::ExpectedEitherOfTokens { found, .. } => {
                        found.start.into_position()
                    }
                },
                end: match &err.inner_error {
                    parser::error::ParseError::ExpectedToken { found, .. }
                    | parser::error::ParseError::ExpectedEitherOfTokens { found, .. } => {
                        found.span().end.into_position()
                    }
                },
            };

            diagnostics.push(Diagnostic::new_simple(range, err.inner_error.to_string()));

            if let Some(msg) = err.message.clone() {
                diagnostics.push(Diagnostic::new_simple(range, msg))
            }
        }

        self.documents.put(params.uri.clone(), params.text);

        diagnostics.reverse();

        self.client
            .publish_diagnostics(params.uri, diagnostics, Some(params.version))
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    ..CompletionOptions::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn hover(&self, _params: HoverParams) -> Result<Option<Hover>> {
        Ok(None)
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_change(ChangedDocumentItem {
            uri: params.text_document.uri,
            version: params.text_document.version,
            text: params.text_document.text,
        })
        .await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let position = params.text_document_position.position;
        let Some(text) = self
            .documents
            .get(params.text_document_position.text_document.uri)
        else {
            return Ok(None);
        };

        let methods = http_method_completions();

        let builtin_functions = builtin_functions_completions();

        let get_variables = |program: &Program| {
            return program
                .variables_before(Location {
                    line: position.line as usize,
                    col: position.character as usize,
                })
                .iter()
                .filter_map(|v| match v {
                    ast::MaybeNode::Node(t) => Some(t.text),
                    _ => None,
                })
                .map(|var| CompletionItem {
                    label: var.to_string(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    insert_text: Some(var.to_string()),
                    ..CompletionItem::default()
                })
                .collect();
        };

        let program = parser::Parser::new(&text).parse();

        let variables = get_variables(&program);

        let completions_store = CompletionsStore {
            functions: builtin_functions,
            methods,
            header_body: header_body_keyword_completions(),
            attributes: attributes_completions(),
            variables,
        };

        let Some(current_item) = program
            .items
            .into_iter()
            .find(|i| i.span().contains(&position))
        else {
            return Ok(Some(CompletionResponse::Array(completions_store.methods)));
        };

        return Ok(current_item
            .completions(&position, &completions_store)
            .map(CompletionResponse::Array));
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.on_change(ChangedDocumentItem {
            uri: params.text_document.uri,
            version: params.text_document.version,
            text: params.content_changes[0].text.clone(),
        })
        .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

pub fn start() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let stdin = tokio::io::stdin();
            let stdout = tokio::io::stdout();

            let (service, socket) = LspService::new(|client| Backend {
                client,
                documents: TextDocuments::new(),
            });
            Server::new(stdin, stdout, socket).serve(service).await;
        });
}
