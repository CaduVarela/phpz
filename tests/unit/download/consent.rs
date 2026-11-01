use phpz::download::consent::*;

#[test]
fn builds_without_a_checksum_from_sources_that_never_publish_one_need_no_consent() {
    for allow in [false, true] {
        for interactive in [false, true] {
            assert_eq!(consent_for(false, allow, interactive), Consent::NotNeeded);
        }
    }
}

#[test]
fn unusual_missing_checksum_asks_a_person_at_a_terminal() {
    assert_eq!(consent_for(true, false, true), Consent::Ask);
}

#[test]
fn unusual_missing_checksum_is_refused_in_scripts() {
    assert_eq!(consent_for(true, false, false), Consent::Refused);
}

#[test]
fn the_flag_gives_consent_with_or_without_a_terminal() {
    assert_eq!(consent_for(true, true, false), Consent::Given);
    assert_eq!(consent_for(true, true, true), Consent::Given);
}
