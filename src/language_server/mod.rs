use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Mutex;
mod completions;
mod hover;
mod position;
mod warnings;

use crate::config::get_env_from_dir_path_or_from_home_dir;
use crate::interpreter::environment::Environment;
use crate::interpreter::{self, runner};
use crate::lexer;
use crate::lexer::locations::{GetSpan, Location};
use crate::parser::ast_visit::VisitWith;
use crate::parser::{self, ast};
use anyhow::{anyhow, Context};
use completions::*;
use tower_lsp::jsonrpc::Result;
use tower_lsp::{lsp_types::*, LspService, Server};
use tower_lsp::{Client, LanguageServer};
use tracing::{debug, error, info, warn};

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

    fn get(&self, uri: &Url) -> Option<String> {
        match self.inner.lock() {
            Ok(map) => map.get(uri).cloned(),
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

    async fn log_error(&self, err: impl Into<Box<dyn std::error::Error>>) {
        self.client
            .log_message(MessageType::ERROR, format!("{:#}", err.into()))
            .await;
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
                document_formatting_provider: Some(OneOf::Left(true)),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: None,
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["run".to_string()],
                    ..Default::default()
                }),
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

        let Some(text) = self.documents.get(&uri) else {
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
            Ok(program) => Some(program),
            Err(err) => {
                self.client
                    .log_message(MessageType::ERROR, format!("{err:#}"))
                    .await;
                None
            }
        };

        let mut hover = hover::HoverDocsResolver::new(program, current_position, env);

        current_item.visit_with(&mut hover);

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: hover.docs.unwrap_or_default(),
            }),
            range: None,
        }))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let position = params.text_document_position.position;

        debug!("cursor position -> {:?}", position);

        let Some(text) = self
            .documents
            .get(&params.text_document_position.text_document.uri)
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

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let Some(text) = self.documents.get(&uri) else {
            warn!(
                "formatting request for an unknown document, by uri: {}",
                uri
            );
            return Ok(None);
        };

        let program = ast::Program::from(&text);
        let formatted_text = match program.to_formatted_string() {
            Ok(formatted_text) => formatted_text,
            Err(err) => {
                error!("failed to format the source text");
                error!("{err:#}");
                return Ok(None);
            }
        };

        let start = Position::new(0, 0);
        let Some(end) = program.items.last().map(|item| {
            let pos = item.span().end;
            Position {
                line: (pos.line as u32) + 1,
                character: (pos.col as u32),
            }
        }) else {
            info!("document has no items to format: {uri}");
            return Ok(None);
        };

        Ok(Some(vec![TextEdit {
            range: Range::new(start, end),
            new_text: formatted_text,
        }]))
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let env = match self.get_env().await {
            Ok(env) => env,
            Err(err) => {
                self.client
                    .log_message(MessageType::ERROR, format!("{err:#}"))
                    .await;
                return Ok(None);
            }
        };

        let uri = params.text_document.uri;
        let Some(text) = self.documents.get(&uri) else {
            warn!("codeLens request for an unknown document, by uri: {}", uri);
            return Ok(None);
        };

        let program = parser::Parser::new(&text).parse();

        let program = match program.interpret(&env) {
            Ok(p) => p,
            Err(err) => {
                self.log_error(anyhow!("{err:#}")).await;
                return Ok(None);
            }
        };

        let codelenses = program
            .items
            .iter()
            .map(|item| {
                let range = Range {
                    start: item.span.start.into_position(),
                    end: item.span.end.into_position(),
                };
                let arg = runner::request_id::RequestId::from(item);

                CodeLens {
                    range,
                    command: Some(Command {
                        title: "Run".to_string(),
                        command: "run".to_string(),
                        arguments: Some(vec![
                            serde_json::Value::String(uri.to_string()),
                            serde_json::Value::String(arg.as_string()),
                        ]),
                    }),
                    data: None,
                }
            })
            .collect();

        Ok(Some(codelenses))
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> Result<Option<serde_json::Value>> {
        match params.command.as_ref() {
            "run" => {
                let args = params
                    .arguments
                    .into_iter()
                    .map(|arg| {
                        arg.as_str()
                            .expect("we should have passed args from the code_lens method")
                            .to_string()
                    })
                    .collect::<Vec<_>>();

                let [path, request_id] = args.as_slice() else {
                    self.log_error(anyhow!(
                        "incorrect number of arguments for 'run' command: {:?}",
                        args
                    ))
                    .await;
                    return Ok(None);
                };

                let uri = Url::from_str(path).expect("failed to read path argument as a Url");
                let path = uri.path();

                let request_id = runner::request_id::RequestId::from_str(request_id)
                    .expect("found invalid request id passed to 'run' command")
                    .url_or_name;

                let Ok(code) = interpreter::read_program_text(Some(path.into())) else {
                    self.log_error(anyhow!("failed to read file from path: {}", path))
                        .await;
                    return Ok(None);
                };

                let env = match self.get_env().await {
                    Ok(env) => env,
                    Err(err) => {
                        self.client
                            .log_message(MessageType::ERROR, format!("{err:#}"))
                            .await;
                        return Ok(None);
                    }
                };

                let Ok(program) = interpreter::interpret_program(&code, env) else {
                    self.log_error(anyhow!("failed to interpret program")).await;
                    return Ok(None);
                };

                info!("running request, id: {}", request_id);

                let response = program
                    .run_ureq(Some(&[request_id]))
                    .iter()
                    .map(|(id, res)| {
                        let mut text = String::new();
                        text.push('`');
                        text.push_str(&id.as_string());
                        text.push('`');
                        text.push('\n');

                        let res = match res {
                            runner::RunResponse::Success(s) => {
                                text.push_str("```json\n");
                                s
                            }
                            runner::RunResponse::Failure(s) => {
                                text.push_str("```sh\n");
                                s
                            }
                        };

                        text.push_str(res);
                        text.push_str("\n```");
                        return text;
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n");

                assert_ne!(
                    response.len(),
                    0,
                    "there must be response(s) to the request"
                );

                return Ok(Some(serde_json::Value::Array(
                    response
                        .lines()
                        .map(|line| serde_json::Value::String(line.to_string()))
                        .collect(),
                )));
            }
            _ => Ok(None),
        }
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
