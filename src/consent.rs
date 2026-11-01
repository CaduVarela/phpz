use crate::interaction;

/// What to do about a download whose checksum is missing.
#[derive(Debug, PartialEq)]
enum Consent {
    NotNeeded,
    Given,
    Ask,
    Refused,
}

/// Only a download that should have had a checksum needs consent. Scripts cannot answer a
/// question, so they must opt in with `--allow-unverified`.
fn consent_for(missing_checksum_is_unusual: bool, allow_unverified: bool, interactive: bool) -> Consent {
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

/// Get consent to install `subject` ("PHP 7.3.33", "xdebug 3.5.3") without a published checksum.
/// Ok when none is needed, it was given with the flag, or the person at the terminal agreed.
pub fn require(missing_checksum_is_unusual: bool, subject: &str, allow_unverified: bool) -> Result<(), String> {
    match consent_for(missing_checksum_is_unusual, allow_unverified, interaction::is_interactive()) {
        Consent::NotNeeded | Consent::Given => Ok(()),
        Consent::Refused => Err(format!(
            "{} has no published checksum, so the download cannot be verified. \
             Run phpz in a terminal to confirm, or pass --allow-unverified.",
            subject
        )),
        Consent::Ask => {
            eprintln!("{} has no published checksum, so the download cannot be verified.", subject);
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
