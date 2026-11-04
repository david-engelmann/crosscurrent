//! The ported test_states corpus, test for test, with the
//! decimal grammar pins of XC-28 standing after it.

mod common;

use common::{state, state_err};
use crosscurrent::CrosscurrentError;
use crosscurrent::values_equal;

const FULL: &str = "\
interface shard-store
author engineer-a

replicas = 3
timeout = 30
operation store = ack
label primary = node-a
excludes = legacy-mode
breaking timeout
";

#[test]
fn parses_a_full_branch_state() {
    let found = state(FULL, "branch-a.state");
    assert_eq!(found.interface, "shard-store");
    assert_eq!(found.author, "engineer-a");
    assert_eq!(found.terms["timeout"].value, "30");
    assert_eq!(found.operations["store"].value, "ack");
    assert_eq!(found.labels["primary"].value, "node-a");
    assert_eq!(
        found.excludes,
        std::collections::BTreeSet::from(["legacy-mode".to_string()])
    );
    assert_eq!(
        found.breaking,
        std::collections::BTreeSet::from(["timeout".to_string()])
    );
    assert_eq!(found.terms["timeout"].line, "timeout = 30");
}

#[test]
fn comments_and_blank_lines_are_not_meaningful() {
    let text = "# a recorded note\n\ninterface shard-store\n# another\nauthor engineer-a\n\n";
    let found = state(text, "s");
    assert_eq!(found.interface, "shard-store");
    assert_eq!(found.author, "engineer-a");
}

#[test]
fn statement_order_never_matters() {
    let first = "interface shard-store\nauthor engineer-a\n\ntimeout = 30\nreplicas = 3\n";
    let second = "interface shard-store\nauthor engineer-a\n\nreplicas = 3\ntimeout = 30\n";
    let a = state(first, "a");
    let b = state(second, "b");
    assert_eq!(a.terms, b.terms);
}

#[test]
fn value_whitespace_is_collapsed_at_parse() {
    let text = "interface shard-store\nauthor engineer-a\n\nnote = two   words\n";
    let found = state(text, "s");
    assert_eq!(found.terms["note"].value, "two words");
}

#[test]
fn label_names_are_never_dereferenced() {
    let text = "interface shard-store\nauthor engineer-a\n\n\
                label primary = node-a\nnote = primary\n";
    let found = state(text, "s");
    assert_eq!(found.terms["note"].value, "primary");
}

#[test]
fn malformed_branch_states_are_refused_by_name() {
    let cases: [(&str, &str); 16] = [
        ("", "missing interface line"),
        (
            "timeout = 30\nauthor engineer-a\n",
            "missing interface line",
        ),
        (
            "interface 9bad\nauthor engineer-a\n",
            "missing interface line",
        ),
        ("interface shard-store\n", "missing author line"),
        (
            "interface shard-store\ntimeout = 30\n",
            "missing author line",
        ),
        (
            "interface shard-store\nauthor engineer-a\ntimeout = 30\nauthor engineer-b\n",
            "author line out of place",
        ),
        (
            "interface shard-store\nauthor engineer-a\ninterface shard-cache\n",
            "more than one interface line",
        ),
        (
            "interface shard-store\nauthor engineer-a\nthis is not a statement\n",
            "matches no statement form",
        ),
        (
            "interface shard-store\nauthor engineer-a\ntimeout = 30\ntimeout = 45\n",
            "duplicate term name timeout",
        ),
        (
            "interface shard-store\nauthor engineer-a\noperation store = ack\noperation store = receipt\n",
            "duplicate operation name store",
        ),
        (
            "interface shard-store\nauthor engineer-a\nlabel a = x\nlabel a = y\n",
            "duplicate label name a",
        ),
        (
            "interface shard-store\nauthor engineer-a\ntimeout =\n",
            "empty value",
        ),
        (
            "interface shard-store\nauthor engineer-a\ntimeout = 30\nbreaking ghost\n",
            "names ghost, which the state does not define",
        ),
        (
            "interface shard-store\nauthor engineer-a\ntimeout = 30\nbreaking timeout\nbreaking timeout\n",
            "duplicate breaking line for timeout",
        ),
        (
            "interface shard-store\nauthor engineer-a\nlegacy-mode = on\nexcludes = legacy-mode\n",
            "defines legacy-mode and also excludes it",
        ),
        (
            "interface shard-store\nauthor engineer-a\nexcludes = a\nexcludes = b\n",
            "more than one excludes line",
        ),
    ];
    for (text, fragment) in cases {
        let error = state_err(text, "branch.state");
        let CrosscurrentError::State(inner) = &error else {
            panic!("expected a state refusal, got {error:?}");
        };
        assert!(
            inner.0.contains(fragment),
            "refusal {:?} lacks {:?} for input {:?}",
            inner.0,
            fragment,
            text
        );
        assert!(inner.0.contains("branch.state"), "{}", inner.0);
    }
}

#[test]
fn values_compare_by_decimal_worth() {
    let equal: [(&str, &str); 17] = [
        ("30", "30.0"),
        ("1e3", "1000"),
        ("-0", "0"),
        ("+3", "3"),
        ("5.", "5"),
        (".5", "0.5"),
        ("1_000", "1000"),
        ("1__0", "10"),
        ("1_0.5", "10.5"),
        ("  42  ", "42"),
        ("1e12", "1e12"),
        ("1e\u{661}\u{662}", "1e12"),
        ("\u{661}\u{662}\u{663}", "123"),
        ("0.1e1000000000000000000", "1e999999999999999999"),
        ("10e999999999999999998", "1e999999999999999999"),
        ("0.1e-1999999999999999996", "1e-1999999999999999997"),
        ("1e00000000000000000005", "100000"),
    ];
    for (left, right) in equal {
        assert!(values_equal(left, right), "{left:?} should equal {right:?}");
        assert!(values_equal(right, left), "{right:?} should equal {left:?}");
    }
}

#[test]
fn values_the_decimal_grammar_refuses_or_leaves_nonfinite_never_agree() {
    let unequal: [(&str, &str); 15] = [
        ("inf", "infinity"),
        ("nan", "NaN"),
        ("snan", "nan"),
        ("1e1000000000000000000", "10e999999999999999999"),
        ("100e999999999999999998", "1e1000000000000000000"),
        ("1e-1999999999999999998", "0.1e-1999999999999999997"),
        ("0", "0e-9999999999999999999999"),
        ("1 0", "10"),
        ("1e", "1"),
        ("1e+", "1"),
        (".", "0"),
        ("\u{bd}", "0.5"),
        ("9007199254740993", "9007199254740992"),
        ("10", "1_0.0.0"),
        ("ab\u{20ac}", "cd\u{20ac}"),
    ];
    for (left, right) in unequal {
        assert!(
            !values_equal(left, right),
            "{left:?} should not equal {right:?}"
        );
        assert!(
            !values_equal(right, left),
            "{right:?} should not equal {left:?}"
        );
    }
    // The identical text shortcut stands ahead of the
    // grammar, so a spelling the grammar refuses still
    // agrees with itself character for character.
    assert!(values_equal("nan", "nan"));
    assert!(values_equal(
        "1e1000000000000000000",
        "1e1000000000000000000"
    ));
}

#[test]
fn a_five_thousand_digit_coefficient_compares_exactly() {
    let spelled = format!("1{}", "0".repeat(4999));
    assert!(values_equal(&spelled, "1e4999"));
    let off_by_one = format!("1{}1", "0".repeat(4998));
    assert!(!values_equal(&spelled, &off_by_one));
}
