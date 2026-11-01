use super::*;

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| name.to_string()).collect()
}

#[test]
fn names_round_trip_and_forgive_case_and_spelling() {
    for variant in Variant::ALL {
        assert_eq!(Variant::parse(variant.name()), Some(variant));
    }
    assert_eq!(Variant::parse(" BULK \n"), Some(Variant::Bulk));
    assert_eq!(Variant::parse("gnu_bulk"), Some(Variant::GnuBulk));
    assert_eq!(Variant::parse("gnubulk"), Some(Variant::GnuBulk));
    assert_eq!(Variant::parse("huge"), None);
    assert_eq!(Variant::parse(""), None);
}

#[test]
fn the_order_is_the_size_order_and_common_is_the_default() {
    assert!(Variant::Minimal < Variant::Common && Variant::Common < Variant::Bulk && Variant::Bulk < Variant::GnuBulk);
    assert_eq!(DEFAULT, Variant::Common);
    assert_eq!(Variant::names(), "minimal, common, bulk, gnu-bulk");
}

#[test]
fn only_gnu_bulk_has_system_requirements() {
    for variant in [Variant::Minimal, Variant::Common, Variant::Bulk] {
        assert!(runs_on(variant, "linux", true).is_ok());
        assert!(runs_on(variant, "macos", false).is_ok());
    }
    assert!(runs_on(Variant::GnuBulk, "linux", false).is_ok());
    assert!(runs_on(Variant::GnuBulk, "macos", false).unwrap_err().contains("Linux only"));
    assert!(runs_on(Variant::GnuBulk, "linux", true).unwrap_err().contains("musl"));
}

#[test]
fn a_version_remembers_its_build() {
    let directory = tempfile::tempdir().unwrap();
    write(directory.path(), Variant::GnuBulk).unwrap();
    assert_eq!(read_origin(directory.path()), Origin::Prebuilt(Variant::GnuBulk));
    assert_eq!(fs::read_to_string(directory.path().join(".phpz-variant")).unwrap(), "gnu-bulk\n");
}

#[test]
fn a_version_without_a_record_or_with_a_broken_one_counts_as_the_default() {
    let directory = tempfile::tempdir().unwrap();
    assert_eq!(read_origin(directory.path()), Origin::Prebuilt(DEFAULT));
    fs::write(directory.path().join(".phpz-variant"), "banana").unwrap();
    assert_eq!(read_origin(directory.path()), Origin::Prebuilt(DEFAULT));
}

#[test]
fn the_smallest_build_that_has_everything_wanted_is_chosen() {
    let candidates = vec![
        (Variant::GnuBulk, set(&["intl", "opcache", "sodium"])),
        (Variant::Bulk, set(&["intl", "opcache", "sodium"])),
        (Variant::Common, set(&["curl", "pdo_sqlite"])),
    ];
    assert_eq!(smallest_covering(&candidates, &set(&["intl"])), Some(Variant::Bulk));
    assert_eq!(smallest_covering(&candidates, &set(&["curl"])), Some(Variant::Common));
    assert_eq!(smallest_covering(&candidates, &set(&["intl", "sodium"])), Some(Variant::Bulk));
}

#[test]
fn nothing_is_chosen_when_no_single_build_has_it_all() {
    let candidates = vec![
        (Variant::Common, set(&["curl", "pdo_sqlite"])),
        (Variant::Bulk, set(&["intl", "opcache"])),
    ];
    assert_eq!(smallest_covering(&candidates, &set(&["intl", "pdo_sqlite"])), None);
    assert_eq!(smallest_covering(&candidates, &set(&["xdebug"])), None);
    assert_eq!(smallest_covering(&[], &set(&["intl"])), None);
}

#[test]
fn wanting_nothing_is_covered_by_the_smallest() {
    let candidates = vec![(Variant::Bulk, set(&["intl"])), (Variant::Common, set(&["curl"]))];
    assert_eq!(smallest_covering(&candidates, &set(&[])), Some(Variant::Common));
}

#[test]
fn switching_loses_what_the_target_lacks() {
    let common = set(&["curl", "pdo_sqlite", "pdo_pgsql", "redis"]);
    let bulk = set(&["curl", "redis", "intl"]);
    assert_eq!(lost_by_switching(&common, &bulk), ["pdo_pgsql", "pdo_sqlite"]);
    assert!(lost_by_switching(&bulk, &bulk).is_empty());
}

#[test]
fn a_version_built_here_is_recorded_as_such() {
    let directory = tempfile::tempdir().unwrap();
    write_source(directory.path()).unwrap();
    assert_eq!(read_origin(directory.path()), Origin::Source);
    assert_eq!(Origin::Source.name(), "source");
}

#[test]
fn a_prebuilt_version_reads_back_as_its_variant_and_unknown_records_as_the_default() {
    let directory = tempfile::tempdir().unwrap();
    assert_eq!(read_origin(directory.path()), Origin::Prebuilt(DEFAULT));
    write(directory.path(), Variant::Bulk).unwrap();
    assert_eq!(read_origin(directory.path()), Origin::Prebuilt(Variant::Bulk));
    assert_eq!(Origin::Prebuilt(Variant::Bulk).name(), "bulk");
    fs::write(directory.path().join(".phpz-variant"), "banana").unwrap();
    assert_eq!(read_origin(directory.path()), Origin::Prebuilt(DEFAULT));
}
