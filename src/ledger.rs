use std::collections::HashSet;

use tower_lsp::lsp_types::CompletionParams;

pub struct Ledger {
    pub accounts: HashSet<String>,
    pub payees: HashSet<String>,
}

impl Ledger {
    pub fn new() -> Self {
        let mut accounts = HashSet::new();
        Self {
            accounts,
            payees: HashSet::new(),
        }
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashSet, io::Cursor};

    use tree_sitter::{Node, TreeCursor};

    use crate::ledger::Ledger;

    #[test]
    fn create_ledger() {
        let s = "2025-01-01 Test Payerr\n\tExpenses:Dinner\t$12.00\n\tAssets:Wallet\n";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(tree_sitter_ledger::language());
        let tree = parser.parse(s, None).unwrap();
        let mut ledger = Ledger::new();
        let mut cursor = tree.walk();
        cursor.goto_first_child();
        cursor.goto_next_sibling();
        // ledger.accounts.insert(cursor.node().kind().into());
        traverse(tree.root_node(), &mut |node: Node| {
            println!(
                "Node: {}, Text: {:?}",
                node.kind(),
                &s[node.start_byte()..node.end_byte()]
            );
            if node.kind() == "account" {
                ledger
                    .accounts
                    .insert(s[node.start_byte()..node.end_byte()].into());
            }
        });

        assert_eq!(
            ledger.accounts,
            HashSet::from_iter::<Vec<String>>(
                vec!["Expenses:Dinner".into(), "Assets:Wallet".into()].into()
            )
        );
    }

    fn traverse(node: Node, f: &mut impl FnMut(Node)) {
        f(node);
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            traverse(child, f);
        }
    }
}
