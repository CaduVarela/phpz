use phpz::commands::list::*;
use phpz::php::selection::Origin;
use phpz::php::selection::Selection;
use phpz::php::variant::Origin as BuildOrigin;
use phpz::php::variant::Variant;
use std::path::PathBuf;

fn selection(version: &str, origin: Origin) -> Selection {
    Selection {
        version: version.to_string(),
        origin,
        project: None,
    }
}

#[test]
fn the_global_default_is_active_when_no_project_applies() {
    let effective = selection("8.4.27", Origin::Global);
    assert_eq!(
        label("8.4.27", Some(&effective), Some("8.4.27"), None),
        "* 8.4.27 (active)"
    );
    assert_eq!(
        label("8.3.32", Some(&effective), Some("8.4.27"), None),
        "  8.3.32"
    );
}

#[test]
fn a_project_pin_is_active_and_the_default_is_still_shown() {
    let effective = selection("8.2.35", Origin::Project(PathBuf::from("/p/.php-version")));
    assert_eq!(
        label("8.2.35", Some(&effective), Some("8.4.27"), None),
        "* 8.2.35 (active, set by /p/.php-version)"
    );
    assert_eq!(
        label("8.4.27", Some(&effective), Some("8.4.27"), None),
        "  8.4.27 (default)"
    );
    assert_eq!(
        label("8.3.32", Some(&effective), Some("8.4.27"), None),
        "  8.3.32"
    );
}

#[test]
fn nothing_is_marked_when_nothing_applies() {
    assert_eq!(label("8.3.32", None, None, None), "  8.3.32");
}

#[test]
fn a_default_is_marked_even_when_a_pin_could_not_be_honored() {
    assert_eq!(
        label("8.4.27", None, Some("8.4.27"), None),
        "  8.4.27 (default)"
    );
}

#[test]
fn a_build_other_than_the_default_is_shown_in_brackets() {
    let effective = selection("8.4.27", Origin::Global);
    assert_eq!(
        label(
            "8.4.27",
            Some(&effective),
            Some("8.4.27"),
            Some(BuildOrigin::Prebuilt(Variant::Bulk))
        ),
        "* 8.4.27 (active) [bulk]"
    );
    assert_eq!(
        label(
            "8.3.32",
            Some(&effective),
            Some("8.4.27"),
            Some(BuildOrigin::Prebuilt(Variant::GnuBulk))
        ),
        "  8.3.32 [gnu-bulk]"
    );
}

#[test]
fn the_default_build_is_not_repeated_on_every_line() {
    assert_eq!(
        label(
            "8.3.32",
            None,
            None,
            Some(BuildOrigin::Prebuilt(Variant::Common))
        ),
        "  8.3.32"
    );
}

#[test]
fn a_version_compiled_here_is_marked_as_source() {
    assert_eq!(
        label("8.3.36", None, None, Some(BuildOrigin::Source)),
        "  8.3.36 [source]"
    );
}
