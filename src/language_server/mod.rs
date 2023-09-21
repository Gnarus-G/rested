use std::collections::HashMap;
use std::sync::Mutex;
mod completions;
mod position;
mod runner;
mod warnings;

use crate::config::env_file_path;
use crate::interpreter::environment::Environment;
use crate::interpreter::{self, Interpreter};
use crate::lexer;
use crate::lexer::locations::{GetSpan, Location};
use crate::parser::ast::Program;
use crate::parser::ast_visit::VisitWith;
use crate::parser::{self};
use completions::*;
use tower_lsp::jsonrpc::Result;
use tower_lsp::{lsp_types::*, LspService, Server};
use tower_lsp::{Client, LanguageServer};
use tracing::{debug, error};

use self::position::ContainsPosition;
use self::runner::NoopRunner;

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
        let Ok(env_file) = env_file_path() else {
            self.client
                .log_message(MessageType::ERROR, "failed to load configs")
                .await;

            return self
                .client
                .publish_diagnostics(params.uri, vec![], Some(params.version))
                .await;
        };

        let Ok(env) = Environment::new(env_file) else {
            self.client
                .log_message(MessageType::ERROR, "failed to initialize the environment")
                .await;

            return self
                .client
                .publish_diagnostics(params.uri, vec![], Some(params.version))
                .await;
        };

        // Handle warnings...

        let program = parser::Parser::new(&params.text).parse();

        let mut w = warnings::EnvVarsNotInAllNamespaces::new(&env);

        for item in program.items {
            item.visit_with(&mut w)
        }

        let mut diagnostics = w.warnings;

        // Done handling warnings

        let Err(interp_errors) = Interpreter::new(&params.text, env, NoopRunner).run(None) else {
            self.documents.put(params.uri.clone(), params.text);

            return self
                .client
                .publish_diagnostics(params.uri, diagnostics, Some(params.version))
                .await;
        };

        match interp_errors {
            interpreter::error::InterpreterError::ParseErrors(p) => {
                for err in p.errors.iter() {
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
                        diagnostics.push(Diagnostic::new_simple(range, msg.to_string()))
                    }
                }
            }
            interpreter::error::InterpreterError::Error(err) => {
                let range = Range {
                    start: err.span.start.into_position(),
                    end: err.span.end.into_position(),
                };

                diagnostics.push(Diagnostic::new_simple(range, err.inner_error.to_string()));

                if let Some(msg) = err.message.clone() {
                    diagnostics.push(Diagnostic::new_simple(range, msg.to_string()))
                }
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

        debug!("cursor position -> {:?}", position);

        let Some(text) = self
            .documents
            .get(params.text_document_position.text_document.uri.clone())
        else {
            error!(
                "failed to get the text by uri: {}",
                params.text_document_position.text_document.uri
            );

            debug!("{:?}", self.documents);

            return Ok(None);
        };

        let get_variables = |program: &Program| {
            return program
                .variables_before(Location {
                    line: position.line as usize,
                    col: position.character as usize,
                })
                .iter()
                .map(|var| CompletionItem {
                    label: var.text.to_string(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    insert_text: Some(var.text.to_string()),
                    ..CompletionItem::default()
                })
                .collect();
        };

        let program = parser::Parser::new(&text).parse();

        let variables = get_variables(&program);

        let env_args = env_args_completions().unwrap_or(vec![]);

        let completions_store = CompletionsStore {
            variables,
            env_args,
        };

        let Some(current_item) = program.items.iter().find(|i| i.span().contains(&position)) else {
            debug!("cursor is apparently not on any items");
            debug!("{:?}", program);
            return Ok(Some(CompletionResponse::Array(item_keywords())));
        };

        debug!("cursor on item -> {:?}", current_item);

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

pub fn start(level: tracing::Level) {
    let subscriber = tracing_subscriber::fmt()
        .pretty()
        .with_max_level(level)
        .with_ansi(false)
        .with_writer(std::io::stderr)
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(run());
    })
}

#[tracing::instrument]
async fn run() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        documents: TextDocuments::new(),
    });

    Server::new(stdin, stdout, socket).serve(service).await;
}
