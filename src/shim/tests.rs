use super::*;

#[test]
fn only_a_program_named_php_is_the_shim() {
    for name in ["php", "php.exe", "PHP.EXE", "/home/me/.phpz/shims/php"] {
        assert!(is_shim_name(OsStr::new(name)), "{}", name);
    }
    for name in ["phpz", "phpz.exe", "php8", "php-cgi", "xphp", "/usr/bin/phpize", ""] {
        assert!(!is_shim_name(OsStr::new(name)), "{}", name);
    }
}

#[test]
fn unix_runs_the_real_binary_with_its_own_ini_settings() {
    let version = Path::new("/h/versions/8.3.32");
    let plan = launch_plan(version, version, false, true);
    assert_eq!(plan.program, PathBuf::from("/h/versions/8.3.32/libexec/php"));
    assert_eq!(
        plan.set,
        [
            ("PHPRC", PathBuf::from("/h/versions/8.3.32")),
            ("PHP_INI_SCAN_DIR", PathBuf::from("/h/versions/8.3.32/conf.d")),
        ]
    );
    assert!(plan.remove.is_empty());
}

#[test]
fn unix_falls_back_to_the_wrapper_when_there_is_no_libexec() {
    let version = Path::new("/h/versions/8.3.32");
    let plan = launch_plan(version, version, false, false);
    assert_eq!(plan.program, PathBuf::from("/h/versions/8.3.32/bin/php"));
    assert!(plan.set.is_empty() && plan.remove.is_empty());
}

#[test]
fn windows_clears_the_scan_directory_another_php_exported() {
    let version = Path::new(r"C:\h\versions\8.3.36");
    let plan = launch_plan(version, version, true, false);
    assert_eq!(plan.program, PathBuf::from(r"C:\h\versions\8.3.36").join("php.exe"));
    assert_eq!(plan.set, [("PHPRC", PathBuf::from(r"C:\h\versions\8.3.36"))]);
    assert_eq!(plan.remove, ["PHP_INI_SCAN_DIR"]);
}

#[cfg(windows)]
#[test]
fn a_windows_path_to_the_copy_is_the_shim() {
    assert!(is_shim_name(OsStr::new(r"C:\Users\me\.phpz\shims\php.exe")));
    assert!(!is_shim_name(OsStr::new(r"C:\Users\me\.cargo\bin\phpz.exe")));
}

#[test]
fn a_project_ini_directory_replaces_the_version_directory_for_php_ini_only() {
    let version = Path::new("/h/versions/8.3.32");
    let generated = Path::new("/h/projects/abc");
    let plan = launch_plan(version, generated, false, true);

    assert_eq!(plan.program, PathBuf::from("/h/versions/8.3.32/libexec/php"));
    assert_eq!(
        plan.set,
        [
            ("PHPRC", PathBuf::from("/h/projects/abc")),
            ("PHP_INI_SCAN_DIR", PathBuf::from("/h/versions/8.3.32/conf.d")),
        ]
    );
}
