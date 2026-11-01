use super::*;

fn lookup_from<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |name| {
        pairs
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.to_string())
    }
}

#[test]
fn nothing_is_reported_when_no_variable_is_set() {
    assert!(find_overrides(lookup_from(&[])).is_empty());
}

#[test]
fn set_variables_are_reported_in_a_stable_order() {
    let found = find_overrides(lookup_from(&[
        ("PHPRC", "C:\\php"),
        ("PHP_INI_SCAN_DIR", "C:\\conf.d"),
    ]));
    let names: Vec<&str> = found.iter().map(|item| item.name).collect();
    assert_eq!(names, ["PHP_INI_SCAN_DIR", "PHPRC"]);
    assert_eq!(found[0].value, "C:\\conf.d");
}

#[test]
fn empty_value_counts_as_unset() {
    assert!(find_overrides(lookup_from(&[("PHPRC", "")])).is_empty());
}

#[test]
fn unrelated_variables_are_ignored() {
    assert!(find_overrides(lookup_from(&[("PATH", "/usr/bin")])).is_empty());
}

#[test]
fn warning_names_the_variable_its_value_and_how_to_remove_it() {
    let found = find_overrides(lookup_from(&[("PHP_INI_SCAN_DIR", "C:\\scoop\\conf.d")]));
    let text = describe(&found[0]);
    assert!(text.contains("PHP_INI_SCAN_DIR"));
    assert!(text.contains("C:\\scoop\\conf.d"));
    assert!(text.contains("$env:PHP_INI_SCAN_DIR = $null"));
    assert!(text.contains("SetEnvironmentVariable(\"PHP_INI_SCAN_DIR\", $null, \"User\")"));
    assert!(text.contains("other PHP installation"));
}
