use std::collections::HashSet;

use tower_lsp::lsp_types::CompletionParams;
use tree_sitter::Node;

#[derive(Default)]
pub struct Ledger {
    accounts: HashSet<String>,
    pub payees: HashSet<String>,
}

impl Ledger {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn process_text(&mut self, s: &String) {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(tree_sitter_ledger::language());
        let tree = parser.parse(&s, None).unwrap();
        traverse(tree.root_node(), &mut |node: Node| {
            println!(
                "Node: {}, Text: {:?}",
                node.kind(),
                // &s[node.start_byte()..node.end_byte()],
                s.get(node.start_byte()..node.end_byte())
            );
            if node.kind() == "account" {
                self.accounts
                    .insert(s[node.start_byte()..node.end_byte()].into());
            } else if node.kind() == "payee" {
                self.payees
                    .insert(s[node.start_byte()..node.end_byte()].into());
            }
        });
    }

    pub fn get_accounts(&self) -> Vec<String> {
        self.accounts.clone().into_iter().collect()
    }

    pub fn get_payees(&self) -> Vec<String> {
        self.payees.clone().into_iter().collect()
    }
}

fn traverse(node: Node, f: &mut impl FnMut(Node)) {
    f(node);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        traverse(child, f);
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashSet, io::Cursor};

    use tree_sitter::{Node, TreeCursor};

    use crate::ledger::Ledger;

    #[test]
    fn get_all_accounts() {
        // arrange
        let s = "2025-01-01 Test Payerr\n\tExpenses:Dinner\t$12.00\n\tAssets:Wallet\n".to_string();
        let mut ledger = Ledger::new();

        // act
        ledger.process_text(&s);
        let a: HashSet<String> = HashSet::from_iter(ledger.get_accounts());

        // assert
        assert_eq!(
            a,
            HashSet::from_iter::<Vec<String>>(vec![
                "Expenses:Dinner".to_string(),
                "Assets:Wallet".to_string()
            ])
        );
    }

    #[test]
    fn get_all_payees() {
        // arrange
        let s = "2025-01-01 Test Payerr\n\tExpenses:Dinner\t$12.00\n\tAssets:Wallet\n".to_string();
        let mut ledger = Ledger::new();

        // act
        ledger.process_text(&s);

        // assert
        assert_eq!(ledger.get_payees(), vec!["Test Payerr".to_string()]);
    }

    // #[test]
    // fn create_ledger() {
    //     let s = "2025-01-01 Test Payerr\n\tExpenses:Dinner\t$12.00\n\tAssets:Wallet\n";
    //     let mut parser = tree_sitter::Parser::new();
    //     parser.set_language(tree_sitter_ledger::language());
    //     let tree = parser.parse(s, None).unwrap();
    //     let mut ledger = Ledger::new();
    //     let mut cursor = tree.walk();
    //     cursor.goto_first_child();
    //     cursor.goto_next_sibling();
    //     // ledger.accounts.insert(cursor.node().kind().into());
    //     traverse(tree.root_node(), &mut |node: Node| {
    //         println!(
    //             "Node: {}, Text: {:?}",
    //             node.kind(),
    //             &s[node.start_byte()..node.end_byte()]
    //         );
    //         if node.kind() == "account" {
    //             ledger
    //                 .accounts
    //                 .insert(s[node.start_byte()..node.end_byte()].into());
    //         }
    //     });
    //
    //     assert_eq!(
    //         ledger.accounts,
    //         HashSet::from_iter::<Vec<String>>(
    //             vec!["Expenses:Dinner".into(), "Assets:Wallet".into()].into()
    //         )
    //     );
    // }
}
