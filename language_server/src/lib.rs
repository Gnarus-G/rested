use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use lexer::locations::{Location, Span};
use lexer::{Lexer, Token, TokenKind};
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
    pub variables: Mutex<HashSet<(String, (usize, usize))>>,
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

        match result {
            Ok(ast) => {
                let vars = ast.items.into_iter().filter_map(|item| match item {
                    parser::ast::Item::Let { identifier, .. } => Some((
                        identifier.name.to_owned(),
                        (identifier.span.start.line, identifier.span.start.col),
                    )),
                    _ => None,
                });

                if let Ok(mut v) = self.variables.lock() {
                    v.extend(vars)
                }
            }
            Err(err) => {
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
            }
        };

        self.documents.put(params.uri.clone(), params.text);

        self.client
            .publish_diagnostics(params.uri, diagnostics, Some(params.version))
            .await;
    }
}

trait ContainsPosition {
    fn contains(&self, position: &Position) -> bool;
}

impl ContainsPosition for Span {
    fn contains(&self, position: &Position) -> bool {
        if self.start.line == self.end.line {
            return (self.start.col..=self.end.col).contains(&(position.character as usize));
        }
        (self.start.line..=self.end.line).contains(&(position.line as usize))
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

        let methods = ["get", "post", "put", "patch", "delete"]
            .map(|keyword| CompletionItem {
                label: format!("{}", keyword),
                kind: Some(CompletionItemKind::KEYWORD),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                    new_text: format!("{}", keyword),
                    range: Range::new(
                        position,
                        Position {
                            line: position.line,
                            character: keyword.len() as u32,
                        },
                    ),
                })),
                ..CompletionItem::default()
            })
            .to_vec();

        let functions = ["env", "read", "escape_new_lines"]
            .map(|keyword| CompletionItem {
                label: format!("{}(..)", keyword),
                kind: Some(CompletionItemKind::FUNCTION),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                    new_text: format!("{}(", keyword),
                    range: Range::new(
                        position,
                        Position {
                            line: position.line,
                            character: keyword.len() as u32,
                        },
                    ),
                })),
                ..CompletionItem::default()
            })
            .to_vec();

        let variables = self
            .variables
            .lock()
            .unwrap()
            .iter()
            .filter(|(.., (line, col))| {
                if *line == position.line as usize {
                    return *col < position.character as usize;
                }
                return *line < position.line as usize;
            })
            .map(|(var, ..)| CompletionItem {
                label: var.to_owned(),
                kind: Some(CompletionItemKind::VARIABLE),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                    new_text: var.to_owned(),
                    range: Range::new(
                        position,
                        Position {
                            line: position.line,
                            character: var.len() as u32,
                        },
                    ),
                })),
                ..CompletionItem::default()
            })
            .collect();

        let tokens: Vec<_> = Lexer::new(&text).into_iter().collect();

        use TokenKind::*;

        let Some(last_token) = tokens.last() else {
            return Ok(Some(CompletionResponse::Array(
                [methods, functions].concat(),
            )));
        };

        let on_last_token = position.line == last_token.start.line as u32
            && position.character > last_token.start.col as u32;

        let (idx, token) = if on_last_token {
            (tokens.len() - 1, last_token)
        } else {
            let found_token = tokens
                .iter()
                .enumerate()
                .find(|(_, token)| {
                    if token.start.line == position.line as usize {
                        return token.start.col > position.character as usize;
                    }
                    return token.start.line > position.line as usize;
                })
                .map(|(idx, ..)| tokens.get(idx - 1).map(|t| (idx - 1, t)))
                .flatten();

            match found_token {
                Some(a) => a,
                None => return Ok(None),
            }
        };

        match token.kind {
            Set => {
                return Ok(Some(CompletionResponse::Array(vec![CompletionItem {
                    label: "BASE_URL".to_owned(),
                    kind: Some(CompletionItemKind::CONSTANT),
                    ..CompletionItem::default()
                }])))
            }
            Assign => {
                return Ok(Some(CompletionResponse::Array(
                    [functions, variables].concat(),
                )));
            }
            StringLiteral => {
                if let Some(Token { kind: Header, .. }) = tokens.get(idx - 1) {
                    return Ok(Some(CompletionResponse::Array(
                        [functions, variables].concat(),
                    )));
                }
            }
            _ => {}
        }

        return Ok(Some(CompletionResponse::Array(
            [methods, functions].concat(),
        )));
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
                variables: Mutex::new(HashSet::new()),
            });
            Server::new(stdin, stdout, socket).serve(service).await;
        });
}
