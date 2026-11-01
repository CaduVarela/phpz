use super::*;

#[test]
fn override_wins_over_the_home_directory() {
    let home = resolve_home(Some("/custom/place".into()), Some("/home/user".into()));
    assert_eq!(home.unwrap(), PathBuf::from("/custom/place"));
}

#[test]
fn empty_override_is_ignored() {
    let home = resolve_home(Some("".into()), Some("/home/user".into()));
    assert_eq!(home.unwrap(), PathBuf::from("/home/user/.phpz"));
}

#[test]
fn default_is_dot_phpz_in_the_home_directory() {
    let home = resolve_home(None, Some("/home/user".into()));
    assert_eq!(home.unwrap(), PathBuf::from("/home/user/.phpz"));
}

#[test]
fn missing_home_directory_is_an_error() {
    assert!(resolve_home(None, None).is_err());
}
