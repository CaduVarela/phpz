use super::*;

#[test]
fn posix_shells_get_an_export() {
    assert_eq!(
        path_line(Path::new("/home/me/.phpz/current/bin"), false),
        "export PATH=\"/home/me/.phpz/current/bin:$PATH\""
    );
}

#[test]
fn powershell_gets_an_assignment_to_env_path() {
    assert_eq!(
        path_line(Path::new(r"C:\Users\me\.phpz\current"), true),
        r#"$env:Path = "C:\Users\me\.phpz\current;" + $env:Path"#
    );
}
