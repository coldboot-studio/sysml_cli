//! Language Server Protocol server (US-206, Batch M).
//!
//! Implements the minimum LSP surface usable from a VS Code / Neovim
//! / Helix client:
//!
//! - `initialize` / `initialized` handshake
//! - `textDocument/didOpen` — store the buffer, validate, publish diagnostics
//! - `textDocument/didChange` (full-text sync) — update buffer, re-validate
//! - `textDocument/didClose` — drop buffer state, clear diagnostics
//! - `textDocument/hover` — show the rule catalog entry for the
//!   diagnostic under the cursor
//! - `shutdown` / `exit`
//!
//! The validation pipeline is the same one the CLI uses: tokens →
//! AST → project index → library → diagnostics. The project index is
//! built per-validation from the currently open documents, so
//! cross-file references resolve as documents are opened.
//!
//! Synchronous; uses `lsp_server` and `lsp_types` directly to keep
//! the dependency surface minimal and avoid pulling in a tokio
//! runtime.

use std::collections::HashMap;
use std::path::PathBuf;

use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    PublishDiagnostics,
};
use lsp_types::request::{HoverRequest, Request as _, Shutdown};
use lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticSeverity, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, Hover, HoverContents, HoverParams,
    HoverProviderCapability, InitializeParams, MarkupContent, MarkupKind, OneOf,
    Position as LspPosition, PublishDiagnosticsParams, Range as LspRange, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
};
use serde_json::Value;

use crate::config::Config;
use crate::diag::{Diagnostic, Severity};
use crate::library::LibraryLoader;
use crate::project::ProjectIndex;
use crate::rules;
use crate::validate;

/// Entry point for `sysml-validate lsp`. Owns the LSP loop until the
/// client sends `shutdown` + `exit`.
pub fn run() -> Result<(), String> {
    let (connection, io_threads) = Connection::stdio();
    let server_capabilities = serde_json::to_value(server_capabilities())
        .map_err(|error| format!("serialize capabilities: {error}"))?;
    let init_params = connection
        .initialize(server_capabilities)
        .map_err(|error| format!("initialize: {error}"))?;
    let _: InitializeParams = serde_json::from_value(init_params)
        .map_err(|error| format!("parse initialize params: {error}"))?;

    let library = LibraryLoader::embedded();
    let mut server = Server::new(connection, library);
    server.run_loop()?;

    io_threads.join().map_err(|error| format!("join io: {error}"))?;
    Ok(())
}

fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(false)),
        ..Default::default()
    }
}

struct OpenDocument {
    text: String,
    version: i32,
}

struct Server {
    connection: Connection,
    library: LibraryLoader,
    documents: HashMap<Uri, OpenDocument>,
}

impl Server {
    fn new(connection: Connection, library: LibraryLoader) -> Self {
        Self {
            connection,
            library,
            documents: HashMap::new(),
        }
    }

    fn run_loop(&mut self) -> Result<(), String> {
        while let Ok(message) = self.connection.receiver.recv() {
            match message {
                Message::Request(request) => {
                    if self
                        .connection
                        .handle_shutdown(&request)
                        .map_err(|error| format!("shutdown check: {error}"))?
                    {
                        return Ok(());
                    }
                    self.handle_request(request);
                }
                Message::Notification(notification) => self.handle_notification(notification),
                Message::Response(_) => {
                    // We don't issue server-to-client requests today, so
                    // a response would mean the client is confused.
                    // Ignore.
                }
            }
        }
        Ok(())
    }

    fn handle_request(&mut self, request: Request) {
        let id = request.id.clone();
        let result = match request.method.as_str() {
            HoverRequest::METHOD => self.handle_hover(&request),
            Shutdown::METHOD => Ok(Value::Null),
            _ => {
                self.send_response(Response {
                    id,
                    result: None,
                    error: Some(lsp_server::ResponseError {
                        code: lsp_server::ErrorCode::MethodNotFound as i32,
                        message: format!("method '{}' not implemented", request.method),
                        data: None,
                    }),
                });
                return;
            }
        };
        match result {
            Ok(value) => self.send_response(Response {
                id,
                result: Some(value),
                error: None,
            }),
            Err(error) => self.send_response(Response {
                id,
                result: None,
                error: Some(lsp_server::ResponseError {
                    code: lsp_server::ErrorCode::InternalError as i32,
                    message: error,
                    data: None,
                }),
            }),
        }
    }

    fn handle_notification(&mut self, notification: Notification) {
        match notification.method.as_str() {
            DidOpenTextDocument::METHOD => {
                if let Ok(params) =
                    serde_json::from_value::<DidOpenTextDocumentParams>(notification.params)
                {
                    self.documents.insert(
                        params.text_document.uri.clone(),
                        OpenDocument {
                            text: params.text_document.text,
                            version: params.text_document.version,
                        },
                    );
                    self.publish_for(&params.text_document.uri);
                }
            }
            DidChangeTextDocument::METHOD => {
                if let Ok(params) =
                    serde_json::from_value::<DidChangeTextDocumentParams>(notification.params)
                {
                    // Full-text sync: each content change replaces the
                    // entire buffer. (We declared FULL sync above.)
                    if let Some(change) = params.content_changes.into_iter().last() {
                        self.documents.insert(
                            params.text_document.uri.clone(),
                            OpenDocument {
                                text: change.text,
                                version: params.text_document.version,
                            },
                        );
                        self.publish_for(&params.text_document.uri);
                    }
                }
            }
            DidCloseTextDocument::METHOD => {
                if let Ok(params) =
                    serde_json::from_value::<DidCloseTextDocumentParams>(notification.params)
                {
                    self.documents.remove(&params.text_document.uri);
                    // Clear diagnostics on close so stale findings
                    // don't haunt the editor's gutter.
                    self.send_notification(
                        PublishDiagnostics::METHOD,
                        PublishDiagnosticsParams {
                            uri: params.text_document.uri,
                            diagnostics: Vec::new(),
                            version: None,
                        },
                    );
                }
            }
            _ => {
                // Ignore unhandled notifications (e.g., $/setTrace).
            }
        }
    }

    fn handle_hover(&self, request: &Request) -> Result<Value, String> {
        let params: HoverParams = serde_json::from_value(request.params.clone())
            .map_err(|error| format!("parse hover params: {error}"))?;
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let Some(document) = self.documents.get(&uri) else {
            return Ok(Value::Null);
        };
        // Re-validate the buffer and find a diagnostic whose range
        // covers the cursor position. Show its full description.
        let path = uri_to_path(&uri);
        let project = self.build_project_index();
        let result = validate::validate_text(
            &path,
            &document.text,
            false,
            &Config::default(),
            &self.library,
            &project,
        );
        for diagnostic in &result.diagnostics {
            if let Some(diag_pos) = &diagnostic.position {
                // Tree-sitter and LSP both use 0-based; our internal
                // Position is 1-based. Normalize before comparing.
                let line0 = (diag_pos.line as u32).saturating_sub(1);
                if line0 == position.line {
                    let rule = rules::lookup(diagnostic.code);
                    let title = rule
                        .map(|rule| format!("**{}** — {}", diagnostic.code, rule.short_description))
                        .unwrap_or_else(|| diagnostic.code.to_string());
                    let body = rule
                        .map(|rule| rule.full_description.to_string())
                        .unwrap_or_default();
                    let markdown = format!("{title}\n\n{body}\n\n*{}*", diagnostic.message);
                    let hover = Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: markdown,
                        }),
                        range: None,
                    };
                    return serde_json::to_value(hover)
                        .map_err(|error| format!("serialize hover: {error}"));
                }
            }
        }
        Ok(Value::Null)
    }

    fn publish_for(&self, uri: &Uri) {
        let path = uri_to_path(uri);
        let Some(document) = self.documents.get(uri) else {
            return;
        };
        let project = self.build_project_index();
        let result = validate::validate_text(
            &path,
            &document.text,
            false,
            &Config::default(),
            &self.library,
            &project,
        );
        let diagnostics: Vec<LspDiagnostic> = result
            .diagnostics
            .iter()
            .filter(|d| !d.is_suppressed())
            .map(diagnostic_to_lsp)
            .collect();
        self.send_notification(
            PublishDiagnostics::METHOD,
            PublishDiagnosticsParams {
                uri: uri.clone(),
                diagnostics,
                version: Some(document.version),
            },
        );
    }

    /// Build a fresh project index covering every currently open
    /// document. Cheap because tokenization + tree-sitter parse on
    /// already-in-memory text is fast.
    fn build_project_index(&self) -> ProjectIndex {
        // Materialize open buffers to temp files? No — just rebuild
        // from the on-disk file paths we DO know about. For files
        // whose latest text is in-buffer only (unsaved changes),
        // diagnostics will be slightly stale until save; that is an
        // acceptable tradeoff for v1. Future work: pass open-buffer
        // overrides into ProjectIndex::build.
        let paths: Vec<PathBuf> = self
            .documents
            .keys()
            .map(uri_to_path)
            .filter(|p| p.exists())
            .collect();
        ProjectIndex::build(&paths)
    }

    fn send_response(&self, response: Response) {
        let _ = self.connection.sender.send(Message::Response(response));
    }

    fn send_notification<P: serde::Serialize>(&self, method: &str, params: P) {
        let Ok(params_value) = serde_json::to_value(params) else {
            return;
        };
        let _ = self
            .connection
            .sender
            .send(Message::Notification(Notification {
                method: method.to_string(),
                params: params_value,
            }));
    }
}

fn uri_to_path(uri: &Uri) -> PathBuf {
    // lsp_types 0.97 wraps fluent_uri::Uri, which doesn't have the
    // legacy to_file_path helper. Decode manually: a file URI looks
    // like `file:///C:/path/foo.sysml` (Windows) or `file:///home/...`
    // (POSIX). Drop the scheme + authority, percent-decode, normalize
    // the leading `/` on Windows drive paths.
    let raw = uri.as_str();
    let stripped = raw
        .strip_prefix("file://")
        .unwrap_or(raw)
        .trim_start_matches('/');
    let decoded = percent_decode(stripped);
    // On Windows the path is `C:/foo`; PathBuf accepts forward slashes.
    // On POSIX we need to prefix the leading `/` we just stripped, so
    // the path is rooted again.
    if decoded.chars().nth(1) == Some(':') {
        PathBuf::from(decoded)
    } else {
        PathBuf::from(format!("/{decoded}"))
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| input.to_string())
}

fn diagnostic_to_lsp(diagnostic: &Diagnostic) -> LspDiagnostic {
    let (line, column) = diagnostic
        .position
        .as_ref()
        .map(|p| (p.line as u32, p.column as u32))
        .unwrap_or((1, 1));
    // Our positions are 1-based; LSP is 0-based.
    let start = LspPosition {
        line: line.saturating_sub(1),
        character: column.saturating_sub(1),
    };
    // Single-character range; sufficient for diagnostic gutter
    // markers. A future revision could carry a true range from the
    // AST when available.
    let end = LspPosition {
        line: start.line,
        character: start.character + 1,
    };
    LspDiagnostic {
        range: LspRange { start, end },
        severity: Some(severity_to_lsp(diagnostic.severity)),
        code: Some(lsp_types::NumberOrString::String(diagnostic.code.to_string())),
        code_description: None,
        source: Some("sysml-validate".to_string()),
        message: diagnostic.message.clone(),
        related_information: None,
        tags: None,
        data: None,
    }
}

fn severity_to_lsp(severity: Severity) -> DiagnosticSeverity {
    match severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Info => DiagnosticSeverity::INFORMATION,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::Position;
    use std::path::Path;

    #[test]
    fn severity_maps_to_lsp() {
        assert_eq!(severity_to_lsp(Severity::Error), DiagnosticSeverity::ERROR);
        assert_eq!(
            severity_to_lsp(Severity::Warning),
            DiagnosticSeverity::WARNING
        );
        assert_eq!(
            severity_to_lsp(Severity::Info),
            DiagnosticSeverity::INFORMATION
        );
    }

    #[test]
    fn diagnostic_converts_zero_based_position() {
        let diagnostic = Diagnostic::new(
            Severity::Error,
            "SYSML041",
            "Duplicate member name.",
            Path::new("/x"),
            Some(Position { line: 5, column: 7 }),
        );
        let lsp = diagnostic_to_lsp(&diagnostic);
        // 1-based 5:7 → 0-based 4:6
        assert_eq!(lsp.range.start.line, 4);
        assert_eq!(lsp.range.start.character, 6);
        assert_eq!(lsp.range.end.character, 7);
        assert_eq!(lsp.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(
            lsp.code,
            Some(lsp_types::NumberOrString::String("SYSML041".into()))
        );
        assert_eq!(lsp.source.as_deref(), Some("sysml-validate"));
    }

    #[test]
    fn diagnostic_without_position_defaults_to_first_column() {
        let diagnostic = Diagnostic::new(
            Severity::Warning,
            "SYSML040",
            "Missing position.",
            Path::new("/x"),
            None,
        );
        let lsp = diagnostic_to_lsp(&diagnostic);
        assert_eq!(lsp.range.start.line, 0);
        assert_eq!(lsp.range.start.character, 0);
    }
}
