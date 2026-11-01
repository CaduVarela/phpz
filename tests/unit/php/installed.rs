use phpz::php::installed::*;

fn names(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn sort_key_orders_numerically() {
    let mut versions = names(&["8.3.10", "8.3.9", "8.10.0", "8.2.30"]);
    versions.sort_by_key(|name| version_sort_key(name));
    assert_eq!(versions, names(&["8.2.30", "8.3.9", "8.3.10", "8.10.0"]));
}

#[test]
fn partial_request_selects_newest_matching_version() {
    let installed = names(&["8.2.30", "8.3.9", "8.3.10"]);
    assert_eq!(select_installed(&installed, "8").unwrap(), "8.3.10");
    assert_eq!(select_installed(&installed, "8.2").unwrap(), "8.2.30");
    assert_eq!(select_installed(&installed, "8.3.9").unwrap(), "8.3.9");
}

#[test]
fn missing_or_invalid_request_is_an_error() {
    let installed = names(&["8.3.10"]);
    assert!(select_installed(&installed, "7").is_err());
    assert!(select_installed(&installed, "8.3.1").is_err());
    assert!(select_installed(&installed, "abc").is_err());
}
