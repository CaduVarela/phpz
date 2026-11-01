use super::*;

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| name.to_string()).collect()
}

fn names(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

const INI: &str = "\
[PHP]
extension_dir = \"ext\"
;extension=bz2
extension=curl
;extension=exif      ; Must be after mbstring as it depends on it
;extension=ldap
;zend_extension=opcache

[Date]
date.timezone = UTC
";

fn available() -> BTreeSet<String> {
    set(&["bz2", "curl", "exif", "gmp", "ldap", "opcache"])
}

fn never_runs() -> Result<BTreeSet<String>, String> {
    panic!("PHP must not run for a shared extension")
}

fn php_modules() -> Result<BTreeSet<String>, String> {
    Ok(set(&["core", "ctype", "curl", "date"]))
}

#[test]
fn every_spelling_of_an_extension_gives_the_same_name() {
    for value in ["curl", "php_curl.dll", "PHP_CURL.DLL", "curl.so", "\"php_curl.dll\"", r"C:\php\ext\php_curl.dll", "/usr/lib/php/curl.so"] {
        assert_eq!(extension_name(value), "curl", "{}", value);
    }
    assert_eq!(extension_name("php_oci8_19.dll"), "oci8_19");
}

#[test]
fn module_list_skips_headers_warnings_and_maps_opcache() {
    let output = "\
Warning: Module \"curl\" is already loaded in Unknown on line 0
PHP Warning:  Module \"gd\" is already loaded in Unknown on line 0
[PHP Modules]
Core
ctype
curl
[Zend Modules]
Zend OPcache
";
    assert_eq!(parse_modules(output), set(&["core", "ctype", "curl", "opcache"]));
}

#[test]
fn enabling_uncomments_the_template_line_in_place() {
    let mut ini = IniDocument::parse(INI);
    assert_eq!(enable(&mut ini, "bz2", &available(), never_runs).unwrap(), Outcome::Changed);
    assert!(ini.render().contains("extension_dir = \"ext\"\nextension=bz2\nextension=curl\n"));
}

#[test]
fn enabling_keeps_the_comment_after_the_line() {
    let mut ini = IniDocument::parse(INI);
    enable(&mut ini, "exif", &available(), never_runs).unwrap();
    assert!(ini.render().contains("\nextension=exif      ; Must be after mbstring as it depends on it\n"));
}

#[test]
fn zend_extensions_use_their_own_key() {
    let mut ini = IniDocument::parse(INI);
    enable(&mut ini, "opcache", &available(), never_runs).unwrap();
    assert_eq!(ini.values(PHP_SECTION, "zend_extension"), ["opcache"]);
}

#[test]
fn enabling_an_enabled_extension_changes_nothing() {
    let mut ini = IniDocument::parse(INI);
    assert_eq!(enable(&mut ini, "curl", &available(), never_runs).unwrap(), Outcome::Unchanged);
    assert_eq!(ini.render(), INI);
}

#[test]
fn an_extension_without_a_template_line_is_added_next_to_the_others() {
    let mut ini = IniDocument::parse(INI);
    assert_eq!(enable(&mut ini, "gmp", &available(), never_runs).unwrap(), Outcome::Changed);
    assert!(ini.render().contains(";extension=ldap\nextension=gmp\n"));
}

#[test]
fn old_series_get_dll_names() {
    let mut ini = IniDocument::parse("[PHP]\n;extension=php_bz2.dll\nextension=php_curl.dll\n");
    enable(&mut ini, "gmp", &set(&["bz2", "curl", "gmp"]), never_runs).unwrap();
    assert!(ini.render().contains("extension=php_gmp.dll\n"));
}

#[test]
fn any_spelling_can_be_used_to_enable() {
    let mut ini = IniDocument::parse(INI);
    assert_eq!(enable(&mut ini, "php_ldap.dll", &available(), never_runs).unwrap(), Outcome::Changed);
    assert!(ini.values(PHP_SECTION, "extension").contains(&"ldap".to_string()));
}

#[test]
fn built_in_and_missing_extensions_are_told_apart() {
    let mut ini = IniDocument::parse(INI);
    assert_eq!(enable(&mut ini, "ctype", &available(), php_modules).unwrap(), Outcome::BuiltIn);
    assert_eq!(enable(&mut ini, "xdebug", &available(), php_modules).unwrap(), Outcome::Unavailable);
    assert_eq!(ini.render(), INI);
}

#[test]
fn disabling_comments_the_line_out_and_is_repeatable() {
    let mut ini = IniDocument::parse(INI);
    assert_eq!(disable(&mut ini, "curl", &available(), never_runs).unwrap(), Outcome::Changed);
    assert!(ini.render().contains(";extension=curl\n"));
    assert_eq!(disable(&mut ini, "curl", &available(), never_runs).unwrap(), Outcome::Unchanged);
}

#[test]
fn enable_then_disable_restores_the_file() {
    let mut ini = IniDocument::parse(INI);
    enable(&mut ini, "bz2", &available(), never_runs).unwrap();
    disable(&mut ini, "bz2", &available(), never_runs).unwrap();
    assert_eq!(ini.render(), INI);
}

#[test]
fn built_in_extensions_cannot_be_disabled() {
    let mut ini = IniDocument::parse(INI);
    assert_eq!(disable(&mut ini, "ctype", &available(), php_modules).unwrap(), Outcome::BuiltIn);
    assert_eq!(disable(&mut ini, "xdebug", &available(), php_modules).unwrap(), Outcome::Unavailable);
}

#[test]
fn failing_to_run_php_is_an_error_only_when_it_is_needed() {
    let mut ini = IniDocument::parse(INI);
    let broken = || Err("PHP failed to list its modules".to_string());
    assert!(enable(&mut ini, "ctype", &available(), broken).is_err());
    assert!(enable(&mut ini, "bz2", &available(), broken).is_ok());
}

#[test]
fn listing_separates_enabled_disabled_failing_and_built_in() {
    let listing = classify(
        &set(&["bz2", "curl", "ldap", "gmp"]),
        &set(&["core", "ctype", "curl"]),
        &set(&["curl", "ldap"]),
    );
    assert_eq!(listing.enabled, names(&["curl"]));
    assert_eq!(listing.failing, names(&["ldap"]));
    assert_eq!(listing.disabled, names(&["bz2", "gmp"]));
    assert_eq!(listing.built_in, names(&["core", "ctype"]));
}

#[test]
fn names_wrap_without_exceeding_the_width() {
    let lines = wrap_names(&names(&["alpha", "beta", "gamma", "delta"]), 14);
    assert_eq!(lines, ["alpha, beta", "gamma, delta"]);
    assert!(lines.iter().all(|line| line.len() <= 14));
    assert!(wrap_names(&[], 80).is_empty());
}
