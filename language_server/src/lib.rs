use std::collections::HashMap;
use std::sync::Mutex;

use lexer::locations::Location;
use parser::Parser;
use tower_lsp::jsonrpc::Result;
use tower_lsp::{lsp_types::*, LspService, Server};
use tower_lsp::{Client, LanguageServer};

trait IntoPosition {
    fn into_position(&self) -> Position;
}

impl IntoPosition for Location {
    fn into_position(&self) -> Position {
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
            Ok(map) => map.get(&uri).map(|s| s.clone()),
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
        let result = Parser::new(&params.text).parse();

        let mut diagnostics = vec![];

        if let Err(err) = result {
            let range = Range {
                start: err.span.start.into_position(),
                end: err.span.end.into_position(),
            };

            diagnostics.push(Diagnostic::new_simple(
                range.clone(),
                err.inner_error.to_string(),
            ));

            if let Some(msg) = err.message {
                diagnostics.push(Diagnostic::new_simple(range, msg))
            }
        };

        self.documents.put(params.uri.clone(), params.text);

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

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let text = self
            .documents
            .get(params.text_document_position_params.text_document.uri)
            .unwrap();

        let mut content = vec![];

        let lexer = lexer::Lexer::new(&text);

        for token in lexer.into_iter() {
            let hover_position = params.text_document_position_params.position;

            if token.start.line != hover_position.line as usize {
                continue;
            }

            let hover_is_on_token = (token.start.col..token.end_location().col)
                .contains(&(hover_position.character as usize));

            if hover_is_on_token {
                content.push(MarkedString::String(format!(
                    "{} {}",
                    token.text, token.start,
                )));
                break;
            }
        }

        if !content.is_empty() {
            return Ok(Some(Hover {
                contents: HoverContents::Array(content),
                range: None,
            }));
        }

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
