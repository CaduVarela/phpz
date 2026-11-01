use phpz::extensions::settings::*;

#[test]
fn empty_file_gives_empty_settings() {
    assert_eq!(Settings::parse("").unwrap(), Settings::default());
    assert!(Settings::parse("").unwrap().extensions.is_empty());
}

#[test]
fn partial_file_fills_in_the_rest() {
    let settings = Settings::parse("[extensions]\nenable = [\"intl\"]\n").unwrap();
    assert_eq!(settings.extensions.enable, ["intl"]);
    assert!(settings.extensions.disable.is_empty());
}

#[test]
fn invalid_file_is_an_error() {
    assert!(Settings::parse("[extensions\nenable =").is_err());
    assert!(Settings::parse("[extensions]\nenable = \"intl\"\n").is_err());
}

#[test]
fn settings_survive_a_render_and_parse_round_trip() {
    let mut settings = Settings::default();
    settings.want_enabled("opcache");
    settings.want_enabled("intl");
    settings.want_disabled("gd");
    assert_eq!(Settings::parse(&settings.render()).unwrap(), settings);
}

#[test]
fn rendered_file_starts_with_an_explanation() {
    assert!(Settings::default().render().starts_with("# phpz settings."));
}

#[test]
fn names_are_kept_sorted_and_unique() {
    let mut settings = Settings::default();
    for name in ["opcache", "intl", "opcache", "gd"] {
        settings.want_enabled(name);
    }
    assert_eq!(settings.extensions.enable, ["gd", "intl", "opcache"]);
}

#[test]
fn a_name_lives_in_one_list_only() {
    let mut settings = Settings::default();
    settings.want_enabled("gd");
    settings.want_disabled("gd");
    assert!(settings.extensions.enable.is_empty());
    assert_eq!(settings.extensions.disable, ["gd"]);

    settings.want_enabled("gd");
    assert_eq!(settings.extensions.enable, ["gd"]);
    assert!(settings.extensions.disable.is_empty());
}

#[test]
fn any_spelling_of_an_extension_is_stored_by_its_name() {
    let mut settings = Settings::default();
    settings.want_enabled("php_intl.dll");
    settings.want_enabled("INTL");
    assert_eq!(settings.extensions.enable, ["intl"]);
}
