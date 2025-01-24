use std::collections::HashSet;

use tower_lsp::lsp_types::{CompletionParams, Position};
use tracing::debug;
use tree_sitter::{Node, Point, Tree};

pub struct Ledger {
    ast: Tree,
    source: String,
    // accounts: HashSet<String>,
    // pub payees: HashSet<String>,
}

impl Ledger {
    pub fn new() -> Self {
        let s = "";
        let mut parser = tree_sitter::Parser::new();
        _ = parser.set_language(tree_sitter_ledger::language());
        let tree = parser.parse(&s, None).unwrap();
        Self {
            ast: tree,
            source: s.to_string(),
        }
    }

    pub fn process_text(&mut self, s: &String) {
        let mut parser = tree_sitter::Parser::new();
        _ = parser.set_language(tree_sitter_ledger::language());
        self.ast = parser.parse(&s, None).unwrap();
        self.source = s.clone()

        // traverse(tree.root_node(), &mut |node: Node| {
        //     println!(
        //         "Node: {}, Text: {:?}",
        //         node.kind(),
        //         // &s[node.start_byte()..node.end_byte()],
        //         s.get(node.start_byte()..node.end_byte())
        //     );
        //     if node.kind() == "account" {
        //         self.accounts
        //             .insert(s[node.start_byte()..node.end_byte()].into());
        //     } else if node.kind() == "payee" {
        //         self.payees
        //             .insert(s[node.start_byte()..node.end_byte()].into());
        //     }
        // });
    }

    pub fn get_accounts(&self, pos: Position) -> Vec<String> {
        let mut accounts: HashSet<String> = HashSet::new();
        debug!("get_accounts: pre {:?}", accounts);
        traverse(self.ast.root_node(), &mut |node: Node| {
            if node.kind() == "account"
                && !in_range(pos, node.start_position(), node.end_position())
            {
                debug!(
                    "get_accounts: in {:?} ({:?}, {:?})",
                    self.source[node.start_byte()..node.end_byte()].to_string(),
                    node.start_position(),
                    node.end_position()
                );
                accounts.insert(self.source[node.start_byte()..node.end_byte()].into());
            }
        });
        debug!("get_accounts: post {:?}", accounts);
        accounts.into_iter().collect()
    }

    pub fn get_payees(&self, pos: Position) -> Vec<String> {
        let mut payees: HashSet<String> = HashSet::new();
        traverse(self.ast.root_node(), &mut |node: Node| {
            if node.kind() == "payee" && !in_range(pos, node.start_position(), node.end_position())
            {
                payees.insert(self.source[node.start_byte()..node.end_byte()].into());
            }
        });
        payees.into_iter().collect()
    }

    pub fn traverse_ast(&self, f: &mut impl FnMut(Node)) {
        traverse(self.ast.root_node(), f);
    }
}

pub fn traverse(node: Node, f: &mut impl FnMut(Node)) {
    f(node);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        traverse(child, f);
    }
}

pub fn in_range(pos: Position, start: Point, end: Point) -> bool {
    pos.line as usize >= start.row
        && pos.character as usize >= start.column
        && pos.line as usize <= end.row
        && pos.character as usize <= end.column
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
        let a: HashSet<String> = HashSet::from_iter(ledger.get_accounts(Default::default()));

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
        let a: HashSet<String> = HashSet::from_iter(ledger.get_payees(Default::default()));

        // assert
        assert_eq!(
            a,
            HashSet::from_iter::<Vec<String>>(vec!["Test Payerr".to_string(),])
        );
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
