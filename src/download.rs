use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use sha2::{Digest, Sha256};

/// Download `url` to `destination`, returning its SHA-256 (lowercase hex).
pub fn fetch(
    url: &str,
    destination: &Path,
    expected_sha256: Option<&str>,
) -> Result<String, String> {
    println!("Downloading {}", url);
    let mut response = reqwest::blocking::get(url)
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("download failed: {}", error))?;

    let total_bytes = response.content_length();
    let mut file = File::create(destination)
        .map_err(|error| format!("creating {}: {}", destination.display(), error))?;
    let mut buffer = [0u8; 64 * 1024];
    let mut downloaded_bytes: u64 = 0;
    let mut hasher = Sha256::new();

    loop {
        let bytes_read = response
            .read(&mut buffer)
            .map_err(|error| format!("download failed: {}", error))?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])
            .map_err(|error| error.to_string())?;
        hasher.update(&buffer[..bytes_read]);
        downloaded_bytes += bytes_read as u64;

        let megabytes = downloaded_bytes as f64 / 1e6;
        match total_bytes {
            Some(total) if total > 0 => {
                eprint!(
                    "\r  {:>3}% ({:.1} MB)",
                    downloaded_bytes * 100 / total,
                    megabytes
                )
            }
            _ => eprint!("\r  {:.1} MB", megabytes),
        }
    }
    eprintln!();

    if let Some(total) = total_bytes {
        if downloaded_bytes != total {
            return Err(format!(
                "incomplete download ({} of {} bytes)",
                downloaded_bytes, total
            ));
        }
    }

    let actual = format!("{:x}", hasher.finalize());
    if let Some(expected) = expected_sha256 {
        check_sha256(&actual, expected)?;
    }
    Ok(actual)
}

/// Tell the user whether the download was verified.
pub fn announce_checksum(verified: bool) {
    if verified {
        println!("Verified SHA-256 checksum");
    } else {
        println!("Note: no checksum is published for this build, so it was not verified.");
    }
}

pub fn check_sha256(actual: &str, expected: &str) -> Result<(), String> {
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(format!(
            "checksum mismatch (expected {}, got {}). The download was discarded.",
            expected, actual
        ))
    }
}

#[cfg(test)]
mod tests;
