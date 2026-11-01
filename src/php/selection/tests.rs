use super::*;
use crate::extensions::settings::ExtensionSettings;
use crate::project::Pinned;

fn installed() -> Vec<String> {
    ["8.2.35", "8.3.9", "8.3.32", "8.4.27"]
        .iter()
        .map(|v| v.to_string())
        .collect()
}

fn project_pinning(request: &str) -> Project {
    Project {
        directory: PathBuf::from("/p"),
        php: Some(Pinned {
            request: request.to_string(),
            file: PathBuf::from("/p/.php-version"),
        }),
        extensions: ExtensionSettings::default(),
    }
}

#[test]
fn without_a_project_the_default_applies() {
    let chosen = choose(None, &installed(), Some("8.4.27".to_string())).unwrap();
    assert_eq!(chosen, Some(("8.4.27".to_string(), Origin::Global)));
}

#[test]
fn nothing_is_chosen_without_a_project_or_a_default() {
    assert_eq!(choose(None, &installed(), None).unwrap(), None);
}

#[test]
fn a_pin_beats_the_default_and_picks_the_newest_match() {
    let project = project_pinning("8.3");
    let chosen = choose(Some(&project), &installed(), Some("8.4.27".to_string())).unwrap();
    assert_eq!(
        chosen,
        Some((
            "8.3.32".to_string(),
            Origin::Project(PathBuf::from("/p/.php-version"))
        ))
    );
}

#[test]
fn an_exact_pin_picks_exactly_that_version() {
    let project = project_pinning("8.3.9");
    assert_eq!(
        choose(Some(&project), &installed(), None)
            .unwrap()
            .unwrap()
            .0,
        "8.3.9"
    );
}

#[test]
fn a_pin_that_is_not_installed_is_an_error_even_with_a_default() {
    let project = project_pinning("7.4");
    let error = choose(Some(&project), &installed(), Some("8.4.27".to_string())).unwrap_err();
    assert!(
        error.contains("7.4") && error.contains("/p/.php-version"),
        "{}",
        error
    );
    assert!(error.contains("phpz install 7.4"), "{}", error);
}

#[test]
fn a_project_that_pins_nothing_leaves_the_default_in_charge() {
    let project = Project {
        php: None,
        ..project_pinning("8.3")
    };
    let chosen = choose(Some(&project), &installed(), Some("8.4.27".to_string())).unwrap();
    assert_eq!(chosen.unwrap().1, Origin::Global);
}
