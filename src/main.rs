mod ledger;

use ls::Backend;
use tower_lsp::{LspService, Server};
use trace::setup_logging;

#[tokio::main]
async fn main() {
    let _guard = setup_logging();
    // tesssitter_parser::parse();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

mod ls {
    use std::collections::HashSet;
    use std::sync::{Arc, RwLock};

    use tower_lsp::jsonrpc::Result;
    use tower_lsp::{lsp_types::*, LspService, Server};
    use tower_lsp::{Client, LanguageServer};
    use tracing::debug;
    use tracing::field::debug;
    use tree_sitter::{Language, Tree};

    use crate::ledger::{self, traverse, Ledger};

    pub struct Backend {
        pub client: Client,
        pub language: Language,
        pub ledger: Arc<RwLock<Ledger>>,
    }

    enum CompletionKind {
        Account,
        Payee,
    }

    impl Backend {
        pub fn new(client: Client) -> Self {
            let language = tree_sitter_ledger::language();
            let ledger = ledger::Ledger::new();
            Self {
                client,
                language,
                ledger: Arc::new(RwLock::new(ledger)),
            }
        }

        fn get_completion_type(&self, pos: Position) -> Option<CompletionKind> {
            let ledger = self.ledger.write().unwrap();
            let mut kind = None;
            ledger.traverse_ast(&mut |node| {
                if pos.line as usize >= node.start_position().row
                    && pos.character as usize >= node.start_position().column
                    && pos.line as usize <= node.end_position().row
                    && pos.character as usize <= node.end_position().column
                {
                    if node.kind() == "account" {
                        kind = Some(CompletionKind::Account);
                    } else if node.kind() == "payee" {
                        kind = Some(CompletionKind::Payee);
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
            debug!("Document open");
        }

        async fn did_change(&self, params: DidChangeTextDocumentParams) {
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
            match self.get_completion_type(pos) {
                Some(kind) => match kind {
                    CompletionKind::Account => Ok(Some(CompletionResponse::List(CompletionList {
                        is_incomplete: false,
                        items: self.account_completion(pos),
                    }))),
                    CompletionKind::Payee => Ok(Some(CompletionResponse::List(CompletionList {
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

        async fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }
}

mod trace {
    use std::fs::OpenOptions;

    use tracing::{debug, level_filters::LevelFilter};

    pub fn setup_logging() -> tracing_appender::non_blocking::WorkerGuard {
        let mut file = OpenOptions::new()
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

mod pest_parser {
    use pest::Parser;
    use pest_derive::Parser;

    #[derive(Parser)]
    #[grammar = "ledger.pest"]
    struct LedgerParser;

    pub fn parse() {
        let successful_parse =
            LedgerParser::parse(Rule::transaction, "123/12/1 Payee\nExpense:Shopping 44");
        println!("{:?}", successful_parse);
    }
}

mod tesssitter_parser {
    use std::fs::{File, OpenOptions};

    use tracing::debug;
    use tree_sitter::{InputEdit, Language, Parser, Point, TreeCursor};

    pub fn parse() {
        let mut parser = Parser::new();
        let language = tree_sitter_ledger::language();
        parser.set_language(language).unwrap();
        let tree = parser
            .parse(include_str!("../testdata/wallet.ledger"), None)
            .unwrap();

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .open("./tree.graph.dot")
            .unwrap();
        tree.print_dot_graph(&file);
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn show_account_completion_when_in_accout_section_in_xact() {
        // let s =
    }
}
