use super::*;

const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

#[test]
fn matching_checksum_is_accepted_regardless_of_case() {
    assert!(check_sha256(ABC_SHA256, ABC_SHA256).is_ok());
    assert!(check_sha256(ABC_SHA256, &ABC_SHA256.to_uppercase()).is_ok());
}

#[test]
fn different_checksum_is_rejected_with_both_values() {
    let error = check_sha256(ABC_SHA256, "00").unwrap_err();
    assert!(error.contains("checksum mismatch"));
    assert!(error.contains(ABC_SHA256) && error.contains("00"));
}

#[test]
fn sha256_of_known_input_matches_the_standard_test_vector() {
    assert_eq!(format!("{:x}", Sha256::digest(b"abc")), ABC_SHA256);
}
