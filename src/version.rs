/// Platform-specific download URL resolver for PHP versions

#[cfg(target_os = "windows")]
pub fn get_download_url(version: &str) -> Option<String> {
    println!("Resolving Windows download URL for {}", version);

    // TODO: fetch https://windows.php.net/downloads/releases/releases.json
    // TODO: parse JSON with serde
    // TODO: select build (default: x64 + non-thread-safe)

    None
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn get_download_url(version: &str) -> Option<String> {
    println!("Resolving Unix download URL for {}", version);

    Some(format!(
        "https://www.php.net/distributions/php-{}.tar.gz",
        version
    ))
}
