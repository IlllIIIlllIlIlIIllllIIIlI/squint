//! LSP server binary for squint.
//!
//! Build with: `cargo build --features lsp --bin squint-lsp`
//!
//! Implements the Language Server Protocol over stdio. Supports:
//! - Full document sync (open, change, close)
//! - Diagnostics (violations) published on every document change
//!
//! Severity levels from the rule config are reflected in diagnostic severity:
//! `Severity::Error` → `DiagnosticSeverity::ERROR`
//! `Severity::Warning` → `DiagnosticSeverity::WARNING`

use squint::{
    build_rules,
    config::Config,
    linter::lint_source,
    rules::{Rule, Severity},
};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,
    rules: Vec<Box<dyn Rule>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "squint-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.lint_and_publish(params.text_document.uri, &params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.lint_and_publish(params.text_document.uri, &change.text)
                .await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        // Clear diagnostics when the file is closed.
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }
}

impl Backend {
    async fn lint_and_publish(&self, uri: Url, source: &str) {
        let rule_refs: Vec<&dyn Rule> = self.rules.iter().map(|r| r.as_ref()).collect();
        let violations = lint_source(source, &rule_refs);

        let diagnostics: Vec<Diagnostic> = violations
            .iter()
            .map(|v| {
                let line = (v.line as u32).saturating_sub(1);
                let col = (v.col as u32).saturating_sub(1);
                let sev = match v.severity {
                    Severity::Error => DiagnosticSeverity::ERROR,
                    Severity::Warning => DiagnosticSeverity::WARNING,
                };
                Diagnostic {
                    range: Range {
                        start: Position::new(line, col),
                        end: Position::new(line, u32::MAX),
                    },
                    severity: Some(sev),
                    code: Some(NumberOrString::String(v.rule_id.to_string())),
                    source: Some("sql-linter".to_string()),
                    message: v.message.clone(),
                    ..Default::default()
                }
            })
            .collect();

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

#[tokio::main]
async fn main() {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let cfg = Config::load(&cwd);
    let rules = build_rules(&cfg);

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client, rules });
    Server::new(stdin, stdout, socket).serve(service).await;
}
