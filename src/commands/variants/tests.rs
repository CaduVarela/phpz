use super::*;

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| name.to_string()).collect()
}

#[test]
fn a_build_with_the_same_extensions_says_so() {
    let same = set(&["curl", "gd"]);
    assert_eq!(
        describe_difference(&same, &same),
        "the default's extensions"
    );
}

#[test]
fn what_is_added_and_what_is_lacking_are_both_shown() {
    let default = set(&["curl", "pdo_sqlite"]);
    let bulk = set(&["curl", "intl", "opcache"]);
    assert_eq!(
        describe_difference(&bulk, &default),
        "adds intl, opcache; lacks pdo_sqlite"
    );
}

#[test]
fn a_smaller_build_only_lacks() {
    assert_eq!(
        describe_difference(&set(&["curl"]), &set(&["curl", "gd"])),
        "lacks gd"
    );
}

#[test]
fn long_lists_are_cut_short_with_a_count() {
    let default = set(&["a"]);
    let many = set(&["a", "b", "c", "d", "e", "f", "g", "h", "i"]);
    assert_eq!(
        describe_difference(&many, &default),
        "adds b, c, d, e, f, g and 2 more"
    );
}
