use phpz::extensions::build::*;
use std::fs;

#[test]
fn the_source_root_is_found_at_the_top_or_one_level_down() {
    let top = tempfile::tempdir().unwrap();
    fs::write(top.path().join("config.m4"), "").unwrap();
    assert_eq!(find_source_root(top.path()), Some(top.path().to_path_buf()));

    let nested = tempfile::tempdir().unwrap();
    fs::create_dir_all(nested.path().join("xdebug-3.5.3")).unwrap();
    fs::write(nested.path().join("package.xml"), "").unwrap();
    fs::write(nested.path().join("xdebug-3.5.3/config.m4"), "").unwrap();
    assert_eq!(
        find_source_root(nested.path()),
        Some(nested.path().join("xdebug-3.5.3"))
    );

    let empty = tempfile::tempdir().unwrap();
    assert_eq!(find_source_root(empty.path()), None);
}

#[test]
fn autoconf_is_needed_on_top_of_the_compiler() {
    let missing = missing_tools(|tool| tool != "autoconf");
    assert_eq!(missing, vec![("autoconf", "autoconf")]);
    assert!(missing_tools(|_| true).is_empty());
}
