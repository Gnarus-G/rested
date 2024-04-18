use std::collections::HashMap;
use std::sync::Mutex;
mod completions;
mod position;
mod warnings;

use crate::config::get_env_from_dir_path_or_from_home_dir;
use crate::interpreter::environment::Environment;
use crate::interpreter::{self, ir};
use crate::lexer;
use crate::lexer::locations::{GetSpan, Location};
use crate::parser::ast_visit::{self, VisitWith};
use crate::parser::{self, ast};
use anyhow::Context;
use completions::*;
use tower_lsp::jsonrpc::Result;
use tower_lsp::{lsp_types::*, LspService, Server};
use tower_lsp::{Client, LanguageServer};
use tracing::{debug, error, warn};

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

    pub version: Option<i32>,

    pub text: String,
}

impl Backend {
    async fn workspace_uris(&self) -> Result<Option<Vec<Url>>> {
        let paths = self
            .client
            .workspace_folders()
            .await?
            .map(|folders| folders.into_iter().map(|f| f.uri).collect::<Vec<_>>());

        Ok(paths)
    }

    async fn get_env(&self) -> anyhow::Result<Environment> {
        let workspace_uris = match self.workspace_uris().await {
            Ok(workspace_uris) => workspace_uris,
            _ => {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        "didn't define the root_dir for rstdls",
                    )
                    .await;
                None
            }
        };

        let env = get_env_from_dir_path_or_from_home_dir(
            workspace_uris
                .and_then(|uris| uris.first().and_then(|uri| uri.to_file_path().ok()))
                .as_deref(),
        )?;

        return Ok(env);
    }

    async fn on_change(&self, params: ChangedDocumentItem) {
        let Ok(env) = self.get_env().await else {
            self.client
                .log_message(MessageType::ERROR, "failed to initialize the environment")
                .await;

            return self
                .client
                .publish_diagnostics(params.uri, vec![], params.version)
                .await;
        };

        // Handle warnings...

        let program = parser::Parser::new(&params.text).parse();

        let mut w = warnings::EnvVarsNotInAllNamespaces::new(&env);

        for item in program.items.iter() {
            item.visit_with(&mut w)
        }

        let mut diagnostics = w.warnings;

        // Done handling warnings

        let Err(interp_errors) = program.interpret(&env) else {
            self.documents.put(params.uri.clone(), params.text);

            return self
                .client
                .publish_diagnostics(params.uri, diagnostics, params.version)
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
            interpreter::error::InterpreterError::EvalErrors(errors) => {
                for err in errors.iter() {
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
        }

        self.documents.put(params.uri.clone(), params.text);

        diagnostics.reverse();

        self.client
            .publish_diagnostics(params.uri, diagnostics, params.version)
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

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let current_position = params.text_document_position_params.position;

        debug!("cursor position -> {:?}", current_position);

        let Some(text) = self.documents.get(uri.clone()) else {
            error!("failed to get the text by uri: {}", uri);

            debug!("{:?}", self.documents);

            return Ok(None);
        };

        let program = parser::Parser::new(&text).parse();

        let env = match self.get_env().await {
            Ok(env) => env,
            Err(err) => {
                self.client
                    .log_message(MessageType::ERROR, format!("{err:#}"))
                    .await;
                return Ok(None);
            }
        };

        struct OnHoverFinder<'source> {
            program: ir::Program<'source>,
            position: Position,
            docs: Option<String>,
            is_in_env_call: bool,
            env: Environment,
        }

        impl<'source> ast_visit::Visitor<'source> for OnHoverFinder<'source> {
            fn visit_call_expr(&mut self, expr: &ast::CallExpr<'source>) {
                if let ast::result::ParsedNode::Ok(ident) = &expr.identifier {
                    if ident.text == "env" {
                        self.is_in_env_call = true
                    }
                };

                if expr.identifier.span().contains(&self.position) {
                    if let ast::result::ParsedNode::Ok(ident) = &expr.identifier {
                        let docs = match ident.text {
                            "env" => [
                                "Read env file to grab values.",
                                "Read `.env.rd.json` from the current workspace if there is one,",
                                "otherwise read that in the home directory.",
                                "```typescript",
                                "(builtin) env(variable: string): string",
                                "```",
                            ]
                            .join("\n"),
                            "json" => [
                                "Convert any value to a json string.",
                                "```typescript",
                                "(builtin) json(value: any): string",
                                "```",
                            ]
                            .join("\n"),
                            "read" => [
                                "Read file contents into a string and returns that string.",
                                "```typescript",
                                "(builtin) read(filename: string): string",
                                "```",
                            ]
                            .join("\n"),
                            "escape_new_lines" => [
                                "Escape the '\\n' characters in a string.",
                                "```typescript",
                                "(builtin) escape_new_lines(value: string): string",
                                "```",
                            ]
                            .join("\n"),
                            _ => "".to_string(),
                        };

                        self.docs = Some(docs);
                        return;
                    };
                }

                expr.visit_children_with(self);
            }

            fn visit_string(&mut self, stringlit: &ast::StringLiteral<'source>) {
                if stringlit.span.contains(&self.position) && self.is_in_env_call {
                    let var = &stringlit.value.to_string();
                    match self.env.get_variable_value(var).cloned() {
                        Some(value) => {
                            self.docs = Some(value);
                            return;
                        }
                        None => warn!("didn't get a value for the variable {var}"),
                    }
                }
            }

            fn visit_endpoint(&mut self, endpoint: &ast::Endpoint<'source>) {
                if endpoint.span().contains(&self.position) {
                    let item_at_position = self
                        .program
                        .items
                        .iter()
                        .find(|i| i.span.contains(&self.position));

                    match item_at_position {
                        Some(item) => {
                            self.docs = Some(item.request.url.clone());
                            return;
                        }
                        None => {
                            warn!("didn't find a evaluated request item for endpoint on cursor")
                        }
                    };
                }
                endpoint.visit_children_with(self);
            }
        }

        let Some(current_item) = program
            .items
            .iter()
            .find(|i| i.span().contains(&current_position))
        else {
            debug!("cursor is apparently not on any items");
            debug!("{:?}", program);
            return Ok(None);
        };

        let program = match program.interpret(&env) {
            Ok(program) => program,
            Err(err) => {
                self.client
                    .log_message(MessageType::ERROR, format!("{err:#}"))
                    .await;
                return Ok(None);
            }
        };

        let mut finder = OnHoverFinder {
            program,
            position: current_position,
            docs: None,
            is_in_env_call: false,
            env,
        };

        current_item.visit_with(&mut finder);

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: finder.docs.unwrap_or_default(),
            }),
            range: None,
        }))
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

        let program = parser::Parser::new(&text).parse();

        let env = match self.get_env().await {
            Ok(env) => env,
            Err(err) => {
                self.client
                    .log_message(MessageType::ERROR, format!("{err:#}"))
                    .await;
                return Ok(None);
            }
        };

        let mut completions_collector = CompletionsCollector::new(&program, position, env);

        let Some(current_item) = program.items.iter().find(|i| i.span().contains(&position)) else {
            debug!("cursor is apparently not on any items");
            debug!("{:?}", program);
            return Ok(Some(CompletionResponse::Array(item_keywords())));
        };

        debug!("cursor on item -> {:?}", current_item);

        current_item.visit_with(&mut completions_collector);

        debug!("done collecting completions");

        return Ok(completions_collector.into_response());
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_change(ChangedDocumentItem {
            uri: params.text_document.uri,
            version: Some(params.text_document.version),
            text: params.text_document.text,
        })
        .await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let text = match std::fs::read_to_string(params.text_document.uri.path())
            .context("failed to read file after save")
        {
            Ok(text) => text,
            Err(err) => {
                self.client
                    .log_message(MessageType::WARNING, format!("{err:#}"))
                    .await;
                return;
            }
        };

        self.on_change(ChangedDocumentItem {
            uri: params.text_document.uri,
            version: None,
            text,
        })
        .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.on_change(ChangedDocumentItem {
            uri: params.text_document.uri,
            version: Some(params.text_document.version),
            text: params.content_changes[0].text.clone(),
        })
        .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents
            .inner
            .lock()
            .expect("failed to get lock for text documents")
            .remove(&params.text_document.uri);
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
