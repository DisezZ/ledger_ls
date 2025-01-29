mod ledger;

use ls::Backend;
use tower_lsp::{LspService, Server};
use trace::setup_logging;

#[tokio::main]
async fn main() {
    let _guard = setup_logging();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

mod ls {

    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    use request::PrepareRenameRequest;
    use tower_lsp::jsonrpc::Result;
    use tower_lsp::lsp_types::*;
    use tower_lsp::{Client, LanguageServer};
    use tracing::debug;

    use tree_sitter::{Language, Node, Parser, Point};

    use crate::ledger::{self, traverse, Ledger};

    pub struct Backend {
        pub client: Client,
        pub language: Language,
        pub ledger: Arc<RwLock<Ledger>>,
    }

    enum NodeKind {
        Account,
        Payee,
    }

    impl TryFrom<String> for NodeKind {
        type Error = &'static str;

        fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
            match value.as_str() {
                "account" => Ok(NodeKind::Account),
                "payee" => Ok(NodeKind::Payee),
                _ => Err("Unknown node kind"),
            }
        }
    }

    impl Backend {
        pub fn new(client: Client) -> Self {
            let language = tree_sitter_ledger::language();
            let mut parser = Parser::new();
            parser.set_language(language).unwrap();
            let ledger = ledger::Ledger::new(parser);
            Self {
                client,
                language,
                ledger: Arc::new(RwLock::new(ledger)),
            }
        }

        fn get_node_kind(&self, pos: Position) -> Option<NodeKind> {
            let ledger = self.ledger.write().unwrap();
            let mut kind = None;
            ledger.traverse_ast(&mut |node| {
                if pos.line as usize >= node.start_position().row
                    && pos.character as usize >= node.start_position().column
                    && pos.line as usize <= node.end_position().row
                    && pos.character as usize <= node.end_position().column
                {
                    if node.kind() == "account" {
                        kind = Some(NodeKind::Account);
                    } else if node.kind() == "payee" {
                        kind = Some(NodeKind::Payee);
                    }
                }
            });
            kind
        }

        fn account_completion(&self, pos: Position) -> Vec<CompletionItem> {
            let ledger = self.ledger.write().unwrap();
            let items = ledger
                .get_accounts(pos)
                .iter()
                .map(|e| CompletionItem::new_simple(e.clone(), "Account".into()))
                .collect::<Vec<CompletionItem>>();
            items
        }

        fn payee_completion(&self, pos: Position) -> Vec<CompletionItem> {
            let ledger = self.ledger.write().unwrap();
            let items = ledger
                .get_payees(pos)
                .iter()
                .map(|e| CompletionItem::new_simple(e.clone(), "Payee".into()))
                .collect::<Vec<CompletionItem>>();
            items
        }
    }

    #[tower_lsp::async_trait]
    impl LanguageServer for Backend {
        async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
            debug!("Initialize");
            Ok(InitializeResult {
                capabilities: ServerCapabilities {
                    text_document_sync: Some(TextDocumentSyncCapability::Kind(
                        TextDocumentSyncKind::FULL,
                    )),
                    completion_provider: Some(CompletionOptions {
                        trigger_characters: Some(vec![":".into(), ".".into()]),
                        ..Default::default()
                    }),
                    rename_provider: Some(OneOf::Right(RenameOptions {
                        prepare_provider: Some(true),
                        work_done_progress_options: WorkDoneProgressOptions {
                            work_done_progress: None,
                        },
                    })),
                    ..Default::default()
                },
                ..Default::default()
            })
        }

        async fn initialized(&self, params: InitializedParams) {
            self.client
                .log_message(MessageType::INFO, "server initialized!")
                .await;
        }

        async fn did_open(&self, params: DidOpenTextDocumentParams) {
            let mut ledger = self.ledger.write().unwrap();
            ledger.process_text(&params.text_document.text);
        }

        async fn did_change(&self, params: DidChangeTextDocumentParams) {
            debug!("did_change params: {:?}", params);
            let mut ledger = self.ledger.write().unwrap();
            ledger.process_text(&params.content_changes[0].text);
        }

        async fn did_close(&self, params: DidCloseTextDocumentParams) {
            debug!("Document close");
        }

        async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
            debug!(
                "cmp at cursor: ({:?}, {:?})",
                params.text_document_position.position.line,
                params.text_document_position.position.character,
            );
            let pos = params.text_document_position.position;
            match self.get_node_kind(pos) {
                Some(kind) => match kind {
                    NodeKind::Account => Ok(Some(CompletionResponse::List(CompletionList {
                        is_incomplete: false,
                        items: self.account_completion(pos),
                    }))),
                    NodeKind::Payee => Ok(Some(CompletionResponse::List(CompletionList {
                        is_incomplete: false,
                        items: self.payee_completion(pos),
                    }))),
                },
                None => Ok(None),
            }
            // if let Some(ctx) = params.context {
            //     let Position { line, character } = params.text_document_position.position;
            //
            //     if let Some(trigger_char) = ctx.trigger_character {
            //         match trigger_char.as_str() {
            //             ":" | "." | _ => {
            //                 let mut items = vec![];
            //                 items.extend(self.account_completion());
            //                 items.extend(self.payee_completion());
            //                 return Ok(Some(CompletionResponse::List(CompletionList {
            //                     is_incomplete: false,
            //                     items: items,
            //                 })));
            //             }
            //             _ => (),
            //         }
            //     }
            // }
            // Ok(None)
        }

        async fn prepare_rename(
            &self,
            params: TextDocumentPositionParams,
        ) -> Result<Option<PrepareRenameResponse>> {
            debug!(
                "prepare rename at cursor: ({:?}, {:?})",
                params.position.line, params.position.character,
            );
            let pos = params.position;
            let ledger = self.ledger.write().unwrap();
            let node = ledger
                .ast
                .as_ref()
                .unwrap()
                .root_node()
                .named_descendant_for_point_range(
                    Point::new(pos.line as usize, pos.character as usize),
                    Point::new(pos.line as usize, pos.character as usize),
                )
                .unwrap();
            match NodeKind::try_from(node.kind().to_string()).ok() {
                Some(kind) => match kind {
                    NodeKind::Account | NodeKind::Payee => {
                        Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
                            range: Range::new(
                                Position {
                                    line: node.range().start_point.row as u32,
                                    character: node.range().start_point.column as u32,
                                },
                                Position {
                                    line: node.range().end_point.row as u32,
                                    character: (node.range().end_point.column + 1) as u32,
                                },
                            ),
                            placeholder: ledger.source
                                [node.byte_range().start..node.byte_range().end]
                                .to_string(),
                        }))
                    }
                    _ => Ok(None),
                },
                None => Ok(None),
            }
        }

        async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
            debug!(
                "rename at cursor: ({:?}, {:?})",
                params.text_document_position.position.line,
                params.text_document_position.position.character,
            );
            let pos = params.text_document_position.position;
            let ledger = self.ledger.write().unwrap();
            let cur_node = ledger
                .ast
                .as_ref()
                .unwrap()
                .root_node()
                .named_descendant_for_point_range(
                    Point::new(pos.line as usize, pos.character as usize),
                    Point::new(pos.line as usize, pos.character as usize),
                )
                .unwrap();
            let mut url_text_edit: HashMap<Url, Vec<TextEdit>> = HashMap::new();
            let mut text_edit_vec: Vec<TextEdit> = vec![];
            traverse(ledger.ast.as_ref().unwrap().root_node(), &mut |node| {
                if node.kind() != cur_node.kind() {
                    return;
                }

                let text =
                    ledger.source[node.byte_range().start..node.byte_range().end].to_string();
                let cur_text = ledger.source
                    [cur_node.byte_range().start..cur_node.byte_range().end]
                    .to_string();
                if cur_text != text {
                    return;
                }

                let range = Range::new(
                    Position {
                        line: node.range().start_point.row as u32,
                        character: node.range().start_point.column as u32,
                    },
                    Position {
                        line: node.range().end_point.row as u32,
                        character: (node.range().end_point.column + 1) as u32,
                    },
                );
                text_edit_vec.push(TextEdit::new(range, params.new_name.clone()));
            });
            url_text_edit.insert(
                params.text_document_position.text_document.uri,
                // vec![TextEdit::new(
                //     Range::new(Position::new(0, 0), Position::new(0, 9)),
                //     params.new_name,
                // )],
                text_edit_vec,
            );
            Ok(Some(WorkspaceEdit::new(url_text_edit)))
        }

        async fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }
}

mod trace {
    use std::fs::OpenOptions;

    use tracing::level_filters::LevelFilter;

    pub fn setup_logging() -> tracing_appender::non_blocking::WorkerGuard {
        let file = OpenOptions::new()
            .append(true)
            .open("/home/lutfee/dev/projects/ledger_ls/logs/server.log")
            .unwrap();
        let (non_blocking, _guard) = tracing_appender::non_blocking(file);
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(LevelFilter::DEBUG)
            .with_writer(non_blocking)
            // Use a more compact, abbreviated log format
            .compact()
            // Display source code file paths
            .with_file(true)
            // Display source code line numbers
            .with_line_number(true)
            .finish();
        tracing::subscriber::set_global_default(subscriber).unwrap();
        _guard
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn show_account_completion_when_in_accout_section_in_xact() {
        // let s =
    }
}
