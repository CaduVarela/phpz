use crate::interaction;

/// What to do about a download whose checksum is missing.
#[derive(Debug, PartialEq)]
enum Consent {
    NotNeeded,
    Given,
    Ask,
    Refused,
}

/// Whether a download without a published checksum needs consent.
fn consent_for(
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

#[cfg(test)]
mod tests;
