use crate::platform::interaction;

/// What to do about a download whose checksum is missing.
#[derive(Debug, PartialEq)]
pub enum Consent {
    NotNeeded,
    Given,
    Ask,
    Refused,
}

/// Whether a download without a published checksum needs consent.
pub fn consent_for(
    missing_checksum_is_unusual: bool,
    allow_unverified: bool,
    interactive: bool,
) -> Consent {
    if !missing_checksum_is_unusual {
        Consent::NotNeeded
    } else if allow_unverified {
        Consent::Given
    } else if interactive {
        Consent::Ask
    } else {
        Consent::Refused
    }
}

/// Get consent to install `subject` without a published checksum.
pub fn require(
    missing_checksum_is_unusual: bool,
    subject: &str,
    allow_unverified: bool,
) -> Result<(), String> {
    match consent_for(
        missing_checksum_is_unusual,
        allow_unverified,
        interaction::is_interactive(),
    ) {
        Consent::NotNeeded | Consent::Given => Ok(()),
        Consent::Refused => Err(format!(
            "{} has no published checksum, so the download cannot be verified. \
             Run phpz in a terminal to confirm, or pass --allow-unverified.",
            subject
        )),
        Consent::Ask => {
            eprintln!(
                "{} has no published checksum, so the download cannot be verified.",
                subject
            );
            eprintln!("It is fetched over HTTPS from the official site, but nothing proves the file is intact.");
            match interaction::confirm("Install it anyway?", false) {
                Some(true) => Ok(()),
                _ => Err("installation cancelled".to_string()),
            }
        }
    }
}

/// Get consent to build from a source signed by a key phpz does not know.
pub fn require_known_signer(fingerprint: &str, allow_unverified: bool) -> Result<(), String> {
    if allow_unverified {
        return Ok(());
    }
    let message = format!(
        "The source is signed by key {}, which is in the php.net keyring but not among the keys phpz knows.",
        fingerprint
    );
    if !interaction::is_interactive() {
        return Err(format!(
            "{} Pass --allow-unverified to build anyway.",
            message
        ));
    }
    eprintln!("{}", message);
    match interaction::confirm("Build it anyway?", false) {
        Some(true) => Ok(()),
        _ => Err("installation cancelled".to_string()),
    }
}
