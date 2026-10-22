//! The thought tree, proven against the implementation it was ported from.

use std::path::PathBuf;

use super::*;

struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "agent-thought-test-{name}-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).expect("the directory");
        Self { root }
    }

    fn tree(&self) -> ThoughtTree {
        ThoughtTree::open(&self.root.join("thought.sqlite")).expect("the tree opens")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn input(title: &str, parent_id: Option<String>) -> ThoughtNodeInput {
    ThoughtNodeInput {
        id: None,
        parent_id,
        title: title.to_string(),
        kind: ThoughtNodeKind::Action,
        question: "what now?".to_string(),
        options: Vec::new(),
        notes: String::new(),
        position_x: 0.0,
        position_y: 0.0,
    }
}

/// A fresh workspace opens on the pipeline rather than a blank canvas.
#[test]
fn a_fresh_tree_lists_the_default_pipeline() {
    let scratch = Scratch::new("seed");
    let tree = scratch.tree();
    let nodes = tree.list_or_seed().expect("the tree lists");
    assert_eq!(nodes.len(), 7);
    assert_eq!(nodes.first().map(|node| node.title.as_str()), Some("Take in a turn"));
    assert!(
        nodes.iter().any(|node| node.title == "Reflect at the boundary"),
        "the pipeline names the reflection boundary"
    );
}

/// The default is added, never imposed: an operator's own tree survives.
#[test]
fn adding_the_default_pipeline_twice_does_not_duplicate_it() {
    let scratch = Scratch::new("twice");
    let tree = scratch.tree();
    let first = tree.add_default_pipeline().expect("the pipeline is added");
    assert_eq!(first.len(), 7);

    let second = tree.add_default_pipeline().expect("the pipeline is added");
    assert!(second.is_empty(), "nothing is added the second time");
    assert_eq!(tree.list().expect("the tree lists").len(), 7);
}

/// A stale tab re-parenting onto a node someone else deleted would leave a
/// branch nothing can reach from the root.
#[test]
fn a_parent_that_does_not_exist_is_refused() {
    let scratch = Scratch::new("parent");
    let tree = scratch.tree();
    let error = tree
        .save(&input("orphan", Some("no-such-node".to_string())))
        .expect_err("a missing parent is refused");
    assert!(error.to_string().contains("no longer exists"), "{error}");
    assert!(tree.list().expect("the tree lists").is_empty());
}

/// Routing is a tree, so a half-deleted branch would leave questions whose
/// parent no longer exists.
#[test]
fn deleting_a_node_removes_everything_below_it() {
    let scratch = Scratch::new("delete");
    let tree = scratch.tree();
    let root = tree.save(&input("root", None)).expect("the root is saved");
    let child = tree
        .save(&input("child", Some(root.id.clone())))
        .expect("the child is saved");
    let grandchild = tree
        .save(&input("grandchild", Some(child.id.clone())))
        .expect("the grandchild is saved");
    let sibling = tree.save(&input("sibling", None)).expect("the sibling is saved");

    tree.delete(&root.id).expect("the branch is deleted");
    let remaining: Vec<String> = tree
        .list()
        .expect("the tree lists")
        .into_iter()
        .map(|node| node.id)
        .collect();
    assert!(!remaining.contains(&root.id));
    assert!(!remaining.contains(&child.id));
    assert!(!remaining.contains(&grandchild.id));
    assert!(remaining.contains(&sibling.id), "an unrelated branch survives");
}

#[test]
fn an_untitled_node_is_named_rather_than_left_blank() {
    let scratch = Scratch::new("untitled");
    let tree = scratch.tree();
    let node = tree.save(&input("   ", None)).expect("the node is saved");
    assert_eq!(node.title, "Untitled node");
}

#[test]
fn options_are_trimmed_and_empty_ones_dropped() {
    let scratch = Scratch::new("options");
    let tree = scratch.tree();
    let mut input = input("with options", None);
    input.options = vec![
        "  first  ".to_string(),
        "".to_string(),
        "   ".to_string(),
        "second".to_string(),
    ];
    let node = tree.save(&input).expect("the node is saved");
    assert_eq!(node.options, vec!["first".to_string(), "second".to_string()]);
}

/// A tree that has nodes but no root is damaged. Adding to it would hang the
/// new branches off nothing.
#[test]
fn a_damaged_tree_gets_nothing_added() {
    let scratch = Scratch::new("damaged");
    let tree = scratch.tree();
    // A node whose parent points at a node that is not there: the tree has
    // nodes and no root.
    {
        let connection = tree.connection();
        connection
            .execute(
                "INSERT INTO thought_nodes (id, parent_id, title, kind, question, options, notes, position_x, position_y, updated_at)
                 VALUES ('orphan', 'gone', 'orphan', 'action', '', '[]', '', 0, 0, 'now')",
                [],
            )
            .expect("the node is inserted");
    }
    let added = tree.add_default_pipeline().expect("the pipeline is considered");
    assert!(added.is_empty(), "a damaged tree gets nothing added");
    assert_eq!(tree.list().expect("the tree lists").len(), 1);
}
