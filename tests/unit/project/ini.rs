use phpz::extensions::settings::ExtensionSettings;
use phpz::project::ini::*;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

const INI: &str =
    "[PHP]\nextension_dir = \"ext\"\n;extension=intl\nextension=curl\n;extension=gd\n";

struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    /// A version with intl, curl and gd available as DLLs.
    fn new() -> Fixture {
        let root = tempfile::tempdir().unwrap();
        let version = root.path().join("versions/8.3.36");
        fs::create_dir_all(version.join("ext")).unwrap();
        for name in ["intl", "curl", "gd"] {
            fs::write(version.join("ext").join(format!("php_{}.dll", name)), "").unwrap();
        }
        fs::write(version.join("php.ini"), INI).unwrap();
        Fixture { root }
    }

    fn version(&self) -> PathBuf {
        self.root.path().join("versions/8.3.36")
    }

    fn projects(&self) -> PathBuf {
        self.root.path().join("projects")
    }

    fn prepare(&self, enable: &[&str], disable: &[&str]) -> PathBuf {
        let wanted = ExtensionSettings {
            enable: enable.iter().map(|name| name.to_string()).collect(),
            disable: disable.iter().map(|name| name.to_string()).collect(),
        };
        prepare(
            &self.version(),
            "8.3.36",
            Path::new("/work/app"),
            &wanted,
            &self.projects(),
        )
        .unwrap()
    }
}

#[test]
fn a_project_without_extension_choices_uses_the_version_as_it_is() {
    let fixture = Fixture::new();
    assert_eq!(fixture.prepare(&[], &[]), fixture.version());
    assert!(!fixture.projects().exists());
}

#[test]
fn choices_that_change_nothing_use_the_version_as_it_is() {
    let fixture = Fixture::new();
    assert_eq!(fixture.prepare(&["curl"], &["gd"]), fixture.version());
    assert!(!fixture.projects().exists());
}

#[test]
fn enabling_and_disabling_writes_a_copy_and_leaves_the_version_alone() {
    let fixture = Fixture::new();
    let directory = fixture.prepare(&["intl"], &["curl"]);

    assert_ne!(directory, fixture.version());
    assert!(directory.starts_with(fixture.projects()));
    let generated = fs::read_to_string(directory.join("php.ini")).unwrap();
    assert!(generated.contains("\nextension=intl\n"), "{}", generated);
    assert!(generated.contains("\n;extension=curl\n"), "{}", generated);
    assert_eq!(
        fs::read_to_string(fixture.version().join("php.ini")).unwrap(),
        INI
    );
}

#[test]
fn the_relative_extension_directory_points_back_at_the_version() {
    let fixture = Fixture::new();
    let directory = fixture.prepare(&["gd"], &[]);
    let generated = fs::read_to_string(directory.join("php.ini")).unwrap();
    let expected = format!(
        "extension_dir = \"{}\"",
        fixture.version().join("ext").display()
    );
    assert!(generated.contains(&expected), "{}", generated);
}

#[test]
fn an_extension_the_version_does_not_have_is_skipped_quietly() {
    let fixture = Fixture::new();
    assert_eq!(fixture.prepare(&["swoole"], &["xdebug"]), fixture.version());
}

#[test]
fn asking_twice_gives_the_same_directory_and_does_not_rewrite_the_file() {
    let fixture = Fixture::new();
    let first = fixture.prepare(&["intl"], &[]);
    let written = fs::metadata(first.join("php.ini"))
        .unwrap()
        .modified()
        .unwrap();

    std::thread::sleep(std::time::Duration::from_millis(30));
    let second = fixture.prepare(&["intl"], &[]);

    assert_eq!(first, second);
    assert_eq!(
        fs::metadata(second.join("php.ini"))
            .unwrap()
            .modified()
            .unwrap(),
        written
    );
}

#[test]
fn a_changed_version_ini_is_picked_up() {
    let fixture = Fixture::new();
    let directory = fixture.prepare(&["intl"], &[]);

    fs::write(
        fixture.version().join("php.ini"),
        format!("{}memory_limit = 1G\n", INI),
    )
    .unwrap();
    fixture.prepare(&["intl"], &[]);

    assert!(fs::read_to_string(directory.join("php.ini"))
        .unwrap()
        .contains("memory_limit = 1G"));
}

#[test]
fn each_project_folder_and_version_has_its_own_directory() {
    let a = key(Path::new("/work/a"), "8.3.36");
    assert_eq!(a, key(Path::new("/work/a"), "8.3.36"));
    assert_ne!(a, key(Path::new("/work/b"), "8.3.36"));
    assert_ne!(a, key(Path::new("/work/a"), "8.4.27"));
    assert_eq!(a.len(), 16);
}

#[test]
fn a_version_without_a_php_ini_is_left_alone() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.version().join("php.ini")).unwrap();
    assert_eq!(fixture.prepare(&["intl"], &[]), fixture.version());
}

#[test]
fn a_php_ini_without_an_extension_directory_is_kept_without_one() {
    let fixture = Fixture::new();
    fs::write(
        fixture.version().join("php.ini"),
        "[PHP]\n;extension=intl\n",
    )
    .unwrap();
    let generated = fs::read_to_string(fixture.prepare(&["intl"], &[]).join("php.ini")).unwrap();
    assert!(!generated.contains("extension_dir"), "{}", generated);
}

#[test]
fn prune_removes_only_what_belongs_to_a_missing_project_or_version() {
    let fixture = Fixture::new();
    let live_project = fixture.root.path().join("live");
    let gone_project = fixture.root.path().join("gone");
    fs::create_dir_all(&live_project).unwrap();
    fs::create_dir_all(&gone_project).unwrap();
    let versions = fixture.root.path().join("versions");

    let wanted = |names: &[&str]| ExtensionSettings {
        enable: names.iter().map(|name| name.to_string()).collect(),
        disable: Default::default(),
    };
    let live = prepare(
        &fixture.version(),
        "8.3.36",
        &live_project,
        &wanted(&["intl"]),
        &fixture.projects(),
    )
    .unwrap();
    let gone = prepare(
        &fixture.version(),
        "8.3.36",
        &gone_project,
        &wanted(&["intl"]),
        &fixture.projects(),
    )
    .unwrap();
    let old_version = prepare(
        &fixture.version(),
        "7.4.33",
        &live_project,
        &wanted(&["intl"]),
        &fixture.projects(),
    )
    .unwrap();
    fs::remove_dir_all(&gone_project).unwrap();

    let listed = prune_in(&fixture.projects(), &versions, true).unwrap();
    assert_eq!(listed.len(), 2);
    assert!(gone.exists() && old_version.exists());

    prune_in(&fixture.projects(), &versions, false).unwrap();
    assert!(live.exists());
    assert!(!gone.exists());
    assert!(!old_version.exists());
}
