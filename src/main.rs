use languagetool_lsp::extract_lang;
use languagetool_rust::api::{check, server::ServerClient};
use lsp_server::{Connection, Message, Notification, Response};
use lsp_types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use tokio::runtime::Runtime;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpellCheckerData {
    uri: String,
    word: String,
    message: String,
    suggestions: Vec<String>,
    range: Range,
}

struct DocumentState {
    text: String,
    diagnostics: HashMap<String, SpellCheckerData>,
}

impl DocumentState {
    fn new() -> Self {
        Self {
            text: String::new(),
            diagnostics: HashMap::new(),
        }
    }
}

fn parse_server_url(url: &str) -> (String, String) {
    // Default to localhost:8010
    let default_host = "localhost".to_string();
    let default_port = "8010".to_string();

    // Handle simple hostname:port format
    if url.contains(':') && !url.contains("://") {
        let parts: Vec<&str> = url.split(':').collect();
        if parts.len() == 2 {
            return (parts[0].to_string(), parts[1].to_string());
        }
    }

    // Handle URL format like "http://localhost:8010"
    if url.contains("://") {
        let parts: Vec<&str> = url.split("://").collect();
        if parts.len() >= 2 {
            let host_port = parts[1];
            let host_parts: Vec<&str> = host_port.split(':').collect();
            let host = host_parts.first().unwrap_or(&default_host.as_str()).to_string();
            let port = if host_parts.len() > 1 {
                host_parts.get(1).unwrap_or(&default_port.as_str()).to_string()
            } else {
                default_port
            };
            return (host, port);
        }
    }

    (default_host, default_port)
}

fn main() {
    let rt = Runtime::new().unwrap();

    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        ..Default::default()
    })
    .unwrap();

    connection
        .initialize(server_capabilities)
        .expect("init failed");

    let mut documents: HashMap<String, DocumentState> = HashMap::new();

    // Check if LanguageTool server is available
    // Use the default client which will connect to localhost:8010
    if let Err(e) = rt.block_on(check_server_connection_default()) {
        eprintln!("Warning: Could not connect to LanguageTool server: {}", e);
        eprintln!("The LSP will start but spell checking will not work.");
        eprintln!("Make sure LanguageTool is running: docker run -p 8010:8010 silpheel/languagetool");
    }

    while let Ok(msg) = connection.receiver.recv() {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req).unwrap() {
                    break;
                }

                if req.method == "textDocument/codeAction" {
                    let params: CodeActionParams = serde_json::from_value(req.params).unwrap();
                    let uri = params.text_document.uri.to_string();

                    let mut code_actions = vec![];

                    if let Some(doc_state) = documents.get(&uri) {
                        let cursor_line = params.range.start.line;

                        // Check if cursor is on a misspelled word
                        let mut cursor_on_misspelled = false;
                        for (_diag_id, spell_data) in &doc_state.diagnostics {
                            let ranges_intersect = spell_data.range.start.line
                                <= params.range.end.line
                                && spell_data.range.end.line >= params.range.start.line
                                && spell_data.range.start.character <= params.range.end.character
                                && spell_data.range.end.character >= params.range.start.character;

                            if ranges_intersect {
                                cursor_on_misspelled = true;
                                break;
                            }
                        }

                        for (diag_id, spell_data) in &doc_state.diagnostics {
                            let ranges_intersect = spell_data.range.start.line
                                <= params.range.end.line
                                && spell_data.range.end.line >= params.range.start.line
                                && spell_data.range.start.character
                                    <= params.range.end.character
                                && spell_data.range.end.character
                                    >= params.range.start.character;

                            let should_show = if cursor_on_misspelled {
                                ranges_intersect
                            } else {
                                spell_data.range.start.line == cursor_line
                            };

                            if should_show {
                                for suggestion in &spell_data.suggestions {
                                    let title = if cursor_on_misspelled {
                                        format!("--> '{}'", suggestion)
                                    } else {
                                        format!("'{}' --> '{}'", spell_data.word, suggestion)
                                    };

                                    let action = CodeAction {
                                        title,
                                        kind: Some(CodeActionKind::QUICKFIX),
                                        diagnostics: None,
                                        edit: Some(WorkspaceEdit {
                                            changes: Some(
                                                vec![(
                                                    params.text_document.uri.clone(),
                                                    vec![TextEdit {
                                                        range: spell_data.range.clone(),
                                                        new_text: suggestion.clone(),
                                                    }],
                                                )]
                                                .into_iter()
                                                .collect(),
                                            ),
                                            document_changes: None,
                                            change_annotations: None,
                                        }),
                                        command: None,
                                        is_preferred: None,
                                        disabled: None,
                                        data: Some(serde_json::to_value(diag_id).unwrap()),
                                    };
                                    code_actions.push(action);
                                }
                            }
                        }
                    }

                    let result = serde_json::to_value(&code_actions).unwrap();
                    let response = Response {
                        id: req.id,
                        result: Some(result),
                        error: None,
                    };

                    connection.sender.send(Message::Response(response)).unwrap();
                }
            }

            Message::Notification(notif) => {
                if notif.method == "textDocument/didOpen"
                    || notif.method == "textDocument/didChange"
                {
                    let params = if notif.method == "textDocument/didOpen" {
                        let open: DidOpenTextDocumentParams =
                            serde_json::from_value(notif.params).unwrap();
                        DidChangeTextDocumentParams {
                            text_document: VersionedTextDocumentIdentifier {
                                uri: open.text_document.uri.clone(),
                                version: 1,
                            },
                            content_changes: vec![TextDocumentContentChangeEvent {
                                range: None,
                                range_length: None,
                                text: open.text_document.text.clone(),
                            }],
                        }
                    } else {
                        serde_json::from_value(notif.params).unwrap()
                    };

                    let uri = params.text_document.uri.to_string();
                    let text = params.content_changes[0].text.clone();

                    // Get or create document state
                    let is_new_doc = !documents.contains_key(&uri);
                    let mut doc_state = if is_new_doc {
                        DocumentState::new()
                    } else {
                        documents.remove(&uri).unwrap()
                    };

                    // Always update text
                    doc_state.text = text.clone();

                    // Always check for now - performance optimization can come later
                    let should_check = true;

                    let mut diagnostics = vec![];

                    // Extract language and clean text (remove language comments)
                    let (lang, cleaned_text) = languagetool_lsp::extract_lang_and_clean(&text);

                    // Check text with LanguageTool
                    if let Err(e) = rt.block_on(async {
                        check_text_with_languagetool(
                            &text,
                            &cleaned_text,
                            &lang,
                            &uri,
                            &mut doc_state,
                            &mut diagnostics,
                        ).await
                    }) {
                        eprintln!("Error checking text with LanguageTool: {}", e);
                    }

                    documents.insert(uri, doc_state);

                    let params = PublishDiagnosticsParams {
                        uri: params.text_document.uri,
                        diagnostics,
                        version: None,
                    };

                    connection
                        .sender
                        .send(Message::Notification(Notification {
                            method: "textDocument/publishDiagnostics".into(),
                            params: serde_json::to_value(params).unwrap(),
                        }))
                        .unwrap();
                }
            }

            _ => {}
        }
    }

    io_threads.join().unwrap();
}

async fn check_server_connection_default() -> Result<(), Box<dyn std::error::Error>> {
    let client = ServerClient::from_env_or_default();
    client.ping().await?;
    Ok(())
}

async fn check_text_with_languagetool(
    original_text: &str,
    cleaned_text: &str,
    lang: &str,
    uri: &str,
    doc_state: &mut DocumentState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = ServerClient::from_env_or_default();

    let req = check::Request::default()
        .with_text(cleaned_text)
        .with_language(lang.to_string());

    match client.check(&req).await {
        Ok(response) => {
            for lt_match in response.matches {
                let context = &lt_match.context;
                let offset = context.offset as usize;
                let length = context.length as usize;

                // Map offset from cleaned text to original text
                let original_offset = map_cleaned_offset_to_original(cleaned_text, original_text, offset);
                let original_end_offset = map_cleaned_offset_to_original(cleaned_text, original_text, offset + length);

                // Convert character offset to line/character position in original text
                let (line, character) = char_offset_to_line_char(original_text, original_offset);
                let (end_line, end_character) = char_offset_to_line_char(original_text, original_end_offset);

                let word = extract_word_at_offset(original_text, original_offset, original_end_offset - original_offset);
                let message = lt_match.message.clone();
                let suggestions = extract_suggestions(&lt_match);

                let diag_id = format!("{}:{}:{}", uri, line, character);

                let spell_data = SpellCheckerData {
                    uri: uri.to_string(),
                    word: word.clone(),
                    message: message.clone(),
                    suggestions: suggestions.clone(),
                    range: Range {
                        start: Position {
                            line: line as u32,
                            character: character as u32,
                        },
                        end: Position {
                            line: end_line as u32,
                            character: end_character as u32,
                        },
                    },
                };

                doc_state.diagnostics.insert(diag_id.clone(), spell_data);

                let diagnostic_message = if suggestions.is_empty() {
                    message
                } else {
                    format!("{}: {}", message, suggestions.join(", "))
                };

                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: line as u32,
                            character: character as u32,
                        },
                        end: Position {
                            line: end_line as u32,
                            character: end_character as u32,
                        },
                    },
                    severity: Some(DiagnosticSeverity::HINT),
                    message: diagnostic_message,
                    data: Some(serde_json::to_value(diag_id).unwrap()),
                    ..Default::default()
                });
            }
        }
        Err(e) => {
            eprintln!("LanguageTool check error: {}", e);
        }
    }

    Ok(())
}

fn map_cleaned_offset_to_original(cleaned_text: &str, original_text: &str, cleaned_offset: usize) -> usize {
    // Find the position in the original text that corresponds to the cleaned offset
    // This handles the case where language comments were removed

    let mut original_chars = original_text.chars().peekable();
    let mut cleaned_chars = cleaned_text.chars().peekable();
    let mut original_pos = 0;
    let mut cleaned_pos = 0;

    while cleaned_pos < cleaned_offset {
        match (original_chars.peek(), cleaned_chars.peek()) {
            (Some(&o_char), Some(&c_char)) if o_char == c_char => {
                original_chars.next();
                cleaned_chars.next();
                original_pos += 1;
                cleaned_pos += 1;
            }
            (Some(&o_char), _) => {
                // Character in original but not in cleaned (part of removed comment)
                original_chars.next();
                original_pos += 1;
            }
            (None, Some(_)) => {
                // Shouldn't happen: cleaned text longer than original
                cleaned_chars.next();
                cleaned_pos += 1;
            }
            (None, None) => break,
        }
    }

    original_pos
}

fn char_offset_to_line_char(text: &str, offset: usize) -> (usize, usize) {
    let mut line_number = 0;
    let mut char_in_line = 0;

    for (char_idx, char) in text.chars().enumerate() {
        if char_idx == offset {
            return (line_number, char_in_line);
        }

        if char == '\n' {
            line_number += 1;
            char_in_line = 0;
        } else if char != '\r' {
            // Don't count \r as a character (part of \r\n)
            char_in_line += 1;
        }
    }

    // If we didn't find the exact offset, return the last position
    (line_number, char_in_line)
}

fn extract_word_at_offset(text: &str, start: usize, length: usize) -> String {
    let end = (start + length).min(text.chars().count());

    // Extract the substring using character offsets
    let chars: Vec<char> = text.chars().collect();
    let result: String = chars[start..end].iter().collect();
    result
}

fn extract_suggestions(lt_match: &languagetool_rust::api::check::Match) -> Vec<String> {
    if lt_match.replacements.is_empty() {
        Vec::new()
    } else {
        lt_match.replacements.iter().map(|r| r.value.clone()).collect()
    }
}
