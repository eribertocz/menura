//! Property test of E0H-1.3: printing a tree and parsing the text gives the same tree.

// `cfg(test)` marks the helpers as test code, where `expect` is allowed (clippy.toml).
#[cfg(test)]
mod support;

use semantic_compiler_expressions::{parse, print};
use support::generator::{Generator, expression};

#[test]
fn e0h_1_3_printing_and_reparsing_gives_the_same_tree() {
    for seed in 1..=2000_u64 {
        let mut generator = Generator::new(seed);
        let depth = u32::try_from(seed % 5).unwrap();
        let tree = expression(&mut generator, depth);
        let text = print(&tree);
        match parse(&text) {
            Ok(reparsed) => assert_eq!(reparsed, tree, "seed {seed}: {text}"),
            Err(error) => panic!("seed {seed}: {text} does not parse: {error}"),
        }
    }
}

#[test]
fn e0h_1_3_printing_is_canonical() {
    for text in [
        "sum ( (orders.amount) ) filter (where orders.status = 'OPEN')",
        "case when a then -(-1) else 'x''y' end",
        "a || b + c",
        "not a is null and b not between 1 and 2",
    ] {
        let tree = support::tree(text);
        let printed = print(&tree);
        assert_eq!(print(&support::tree(&printed)), printed, "{text}");
    }
    assert_eq!(
        print(&support::tree(
            "sum((orders.amount)) filter (where (orders.status)='OPEN')"
        )),
        "SUM(orders.amount) FILTER (WHERE orders.status = 'OPEN')"
    );
}
