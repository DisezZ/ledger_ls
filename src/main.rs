mod ledger;

use ls::Backend;
use tower_lsp::{LspService, Server};
use trace::setup_logging;

#[tokio::main]
async fn main() {
    setup_logging();
    tesssitter_parser::parse();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

mod ls {
    use std::collections::HashSet;

    use tower_lsp::jsonrpc::Result;
    use tower_lsp::{lsp_types::*, LspService, Server};
    use tower_lsp::{Client, LanguageServer};
    use tracing::debug;
    use tree_sitter::Language;

    use crate::ledger::{self, Ledger};

    pub struct Backend {
        pub client: Client,
        pub language: Language,
        pub ledger: Ledger,
    }

    impl Backend {
        pub fn new(client: Client) -> Self {
            let language = tree_sitter_ledger::language();
            let ledger = ledger::Ledger::new();
            Self {
                client,
                language,
                ledger,
            }
        }
    }

    #[tower_lsp::async_trait]
    impl LanguageServer for Backend {
        async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
            _ = self.ledger.accounts;
            Ok(InitializeResult {
                capabilities: ServerCapabilities {
                    completion_provider: Some(CompletionOptions {
                        trigger_characters: Some(vec![".".into()]),
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
            debug!("server initialized!");
        }

        async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
            if let Some(ctx) = params.context {
                if let Some(trigger_char) = ctx.trigger_character {
                    match trigger_char.as_str() {
                        "." => {
                            debug!("completion list");
                            let v = vec![1];
                            let mut items = self
                                .ledger
                                .accounts
                                .iter()
                                .map(|e| CompletionItem::new_simple("Q".into(), "Q1".into()))
                                .collect::<Vec<CompletionItem>>();
                            items.extend::<Vec<CompletionItem>>(
                                vec![CompletionItem::new_simple(
                                    "test completion label".into(),
                                    "test completion detail".into(),
                                )]
                                .into(),
                            );
                            // items.extend();
                            return Ok(Some(CompletionResponse::List(CompletionList {
                                is_incomplete: false,
                                items: items,
                            })));
                        }
                        _ => (),
                    }
                }
            }
            Ok(None)
        }

        async fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }
}

mod trace {
    use std::fs::OpenOptions;

    use tracing::level_filters::LevelFilter;

    pub fn setup_logging() {
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
        println!("{:?}", tree);
        // let mut cursor = tree.walk();
        // tree.root_node()
        //     .children(&mut cursor)
        //     .for_each(|e| println!("{:?}", e));
        traverse_child(tree.walk());
    }

    fn traverse_child(cursor: TreeCursor) {
        // cursor.clone_into
    }
}
