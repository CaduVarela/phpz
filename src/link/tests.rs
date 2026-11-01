use super::*;

fn directory_with_file(parent: &Path, name: &str) -> std::path::PathBuf {
    let directory = parent.join(name);
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("marker.txt"), name).unwrap();
    directory
}

#[test]
fn link_exposes_the_files_of_its_target() {
    let root = tempfile::tempdir().unwrap();
    let target = directory_with_file(root.path(), "8.3.1");
    let link = root.path().join("current");

    replace_link(&target, &link).unwrap();

    assert_eq!(
        fs::read_to_string(link.join("marker.txt")).unwrap(),
        "8.3.1"
    );
}

#[test]
fn replacing_a_link_switches_the_target() {
    let root = tempfile::tempdir().unwrap();
    let first = directory_with_file(root.path(), "8.3.1");
    let second = directory_with_file(root.path(), "8.4.2");
    let link = root.path().join("current");

    replace_link(&first, &link).unwrap();
    replace_link(&second, &link).unwrap();

    assert_eq!(
        fs::read_to_string(link.join("marker.txt")).unwrap(),
        "8.4.2"
    );
    assert_eq!(fs::read_link(&link).unwrap().file_name().unwrap(), "8.4.2");
}

#[test]
fn removing_a_link_keeps_the_target_and_its_files() {
    let root = tempfile::tempdir().unwrap();
    let target = directory_with_file(root.path(), "8.3.1");
    let link = root.path().join("current");
    replace_link(&target, &link).unwrap();

    assert!(remove_link(&link).unwrap());

    assert!(!link.exists());
    assert_eq!(
        fs::read_to_string(target.join("marker.txt")).unwrap(),
        "8.3.1"
    );
}

#[test]
fn removing_a_missing_link_is_not_an_error() {
    let root = tempfile::tempdir().unwrap();
    assert!(!remove_link(&root.path().join("current")).unwrap());
}
