use super::*;
use std::path::PathBuf;

fn selection(version: &str, origin: Origin) -> Selection {
    Selection { version: version.to_string(), origin }
}

#[test]
fn the_global_default_is_active_when_no_project_applies() {
    let effective = selection("8.4.27", Origin::Global);
    assert_eq!(label("8.4.27", Some(&effective), Some("8.4.27")), "* 8.4.27 (active)");
    assert_eq!(label("8.3.32", Some(&effective), Some("8.4.27")), "  8.3.32");
}

#[test]
fn a_project_pin_is_active_and_the_default_is_still_shown() {
    let effective = selection("8.2.35", Origin::Project(PathBuf::from("/p/.php-version")));
    assert_eq!(
        label("8.2.35", Some(&effective), Some("8.4.27")),
        "* 8.2.35 (active, set by /p/.php-version)"
    );
    assert_eq!(label("8.4.27", Some(&effective), Some("8.4.27")), "  8.4.27 (default)");
    assert_eq!(label("8.3.32", Some(&effective), Some("8.4.27")), "  8.3.32");
}

#[test]
fn nothing_is_marked_when_nothing_applies() {
    assert_eq!(label("8.3.32", None, None), "  8.3.32");
}

#[test]
fn a_default_is_marked_even_when_a_pin_could_not_be_honored() {
    assert_eq!(label("8.4.27", None, Some("8.4.27")), "  8.4.27 (default)");
}
