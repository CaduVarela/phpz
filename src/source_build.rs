use std::env;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::project;

const RELEASES_URL: &str = "https://www.php.net/releases/index.php?json&version=";
const DISTRIBUTIONS_URL: &str = "https://www.php.net/distributions";
const KEYRING_URL: &str = "https://www.php.net/distributions/php-keyring.gpg";

/// The oldest series the configure recipe below is written for.
pub const OLDEST_BUILDABLE: (u32, u32) = (7, 4);

/// Before 8.1 PHP cannot use OpenSSL 3, which current Linux distributions ship, so the build may fail.
pub const FIRST_OPENSSL_3_SERIES: (u32, u32) = (8, 1);

/// A PHP release as php.net publishes its source.
#[derive(Debug, PartialEq, Clone)]
pub struct SourceRelease {
    pub version: String,
    pub url: String,
    pub sha256: String,
}

impl SourceRelease {
    pub fn signature_url(&self) -> String {
        format!("{}.asc", self.url)
    }

    /// (major, minor) of the version.
    pub fn series(&self) -> (u32, u32) {
        let mut parts = self.version.split('.').filter_map(|part| part.parse().ok());
        (parts.next().unwrap_or(0), parts.next().unwrap_or(0))
    }
}

/// The release described by php.net's JSON: its tar.gz and the SHA-256 it publishes for it.
pub fn parse_release(json: &str) -> Result<SourceRelease, String> {
    let release: Value = serde_json::from_str(json).map_err(|error| format!("invalid release data: {}", error))?;
    let version = release
        .get("version")
        .and_then(Value::as_str)
        .ok_or("php.net has no release matching that version")?;
    let archive = release
        .get("source")
        .and_then(Value::as_array)
        .and_then(|files| {
            files.iter().find(|file| {
                file.get("filename").and_then(Value::as_str).map(|name| name.ends_with(".tar.gz")).unwrap_or(false)
            })
        })
        .ok_or("php.net lists no tar.gz source for that release")?;

    let name = archive.get("filename").and_then(Value::as_str).ok_or("invalid release data")?;
    let sha256 = archive
        .get("sha256")
        .and_then(Value::as_str)
        .ok_or("php.net publishes no SHA-256 for that release")?;
    Ok(SourceRelease {
        version: version.to_string(),
        url: format!("{}/{}", DISTRIBUTIONS_URL, name),
        sha256: sha256.to_lowercase(),
    })
}

/// "8", "8.3" or "8.3.9" to the newest matching release, from php.net.
pub fn resolve(input: &str) -> Result<SourceRelease, String> {
    if !project::is_valid_request(input) {
        return Err(format!("invalid version '{}'. Expected MAJOR, MAJOR.MINOR or MAJOR.MINOR.PATCH", input));
    }
    let response = reqwest::blocking::get(format!("{}{}", RELEASES_URL, input))
        .map_err(|error| format!("failed to reach php.net: {}", error))?;
    let body = response
        .text()
        .map_err(|error| format!("failed to read the answer of php.net: {}", error))?;
    parse_release(&body).map_err(|error| format!("{} ('{}')", error, input))
}

/// One piece of PHP the recipe can build, and what it needs from the system.
struct Component {
    name: &'static str,
    flags: &'static [&'static str],
    /// pkg-config modules that must all be present.
    packages: &'static [&'static str],
    /// What to install on Debian and Ubuntu, shown as a hint when the libraries are missing.
    hint: &'static str,
}

/// Needs nothing beyond a compiler. `opcache` is a `.so` so that it can be switched on and off.
const CORE_FLAGS: &[&str] = &[
    "--disable-all",
    "--enable-cli",
    "--disable-cgi",
    "--disable-phpdbg",
    "--enable-mysqlnd",
    "--enable-pdo",
    "--with-pdo-mysql=mysqlnd",
    "--with-mysqli=mysqlnd",
    "--enable-bcmath",
    "--enable-calendar",
    "--enable-ctype",
    "--enable-exif",
    "--enable-fileinfo",
    "--enable-filter",
    "--enable-ftp",
    "--enable-pcntl",
    "--enable-posix",
    "--enable-session",
    "--enable-sockets",
    "--enable-tokenizer",
];

/// From this series on opcache is always part of PHP, so there is no `opcache.so` to switch.
const OPCACHE_ALWAYS_BUILT_IN_FROM: (u32, u32) = (8, 5);

/// Compiled in when the libraries are there.
const BUILT_IN: &[Component] = &[
    Component { name: "openssl", flags: &["--with-openssl"], packages: &["openssl"], hint: "libssl-dev" },
    Component { name: "curl", flags: &["--with-curl"], packages: &["libcurl"], hint: "libcurl4-openssl-dev" },
    Component { name: "zlib", flags: &["--with-zlib"], packages: &["zlib"], hint: "zlib1g-dev" },
    Component { name: "mbstring", flags: &["--enable-mbstring"], packages: &["oniguruma"], hint: "libonig-dev" },
    Component {
        name: "xml",
        flags: &["--with-libxml", "--enable-dom", "--enable-simplexml", "--enable-xml", "--enable-xmlreader", "--enable-xmlwriter"],
        packages: &["libxml-2.0"],
        hint: "libxml2-dev",
    },
    Component { name: "sqlite", flags: &["--with-pdo-sqlite", "--with-sqlite3"], packages: &["sqlite3"], hint: "libsqlite3-dev" },
    Component { name: "zip", flags: &["--with-zip"], packages: &["libzip"], hint: "libzip-dev" },
    Component {
        name: "gd",
        flags: &["--with-zlib", "--enable-gd", "--with-jpeg", "--with-freetype"],
        packages: &["zlib", "libpng", "libjpeg", "freetype2"],
        hint: "libpng-dev libjpeg-dev libfreetype-dev",
    },
];

/// Built as `.so` files, so `phpz ext enable|disable` can switch them.
const SHARED: &[Component] = &[
    Component { name: "intl", flags: &["--enable-intl=shared"], packages: &["icu-i18n"], hint: "libicu-dev" },
    Component { name: "soap", flags: &["--with-libxml", "--enable-soap=shared"], packages: &["libxml-2.0"], hint: "libxml2-dev" },
    Component { name: "sodium", flags: &["--with-sodium=shared"], packages: &["libsodium"], hint: "libsodium-dev" },
];

#[derive(Debug, PartialEq)]
pub struct Skipped {
    pub name: &'static str,
    pub hint: &'static str,
}

/// What will be configured, given the libraries this system has.
#[derive(Debug, PartialEq)]
pub struct Plan {
    pub flags: Vec<String>,
    pub built_in: Vec<&'static str>,
    pub shared: Vec<&'static str>,
    pub skipped: Vec<Skipped>,
}

pub fn plan(series: (u32, u32), has_package: impl Fn(&str) -> bool) -> Plan {
    let mut flags: Vec<String> = CORE_FLAGS.iter().map(|flag| flag.to_string()).collect();
    let mut built_in = Vec::new();
    let mut shared: Vec<&'static str> = Vec::new();
    let mut skipped = Vec::new();

    // Needs nothing from the system. Switchable as a .so until it became part of PHP itself.
    if series < OPCACHE_ALWAYS_BUILT_IN_FROM {
        flags.push("--enable-opcache=shared".to_string());
        shared.push("opcache");
    }

    let mut consider = |component: &Component, is_shared: bool| {
        if component.packages.iter().all(|package| has_package(package)) {
            flags.extend(component.flags.iter().map(|flag| flag.to_string()));
            if is_shared {
                shared.push(component.name);
            } else {
                built_in.push(component.name);
            }
        } else {
            skipped.push(Skipped { name: component.name, hint: component.hint });
        }
    };
    for component in BUILT_IN {
        consider(component, false);
    }
    for component in SHARED {
        consider(component, true);
    }
    Plan { flags, built_in, shared, skipped }
}

/// Whether pkg-config knows a library.
pub fn pkg_config_has(package: &str) -> bool {
    Command::new("pkg-config")
        .args(["--exists", package])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// The first directory of `path_variable` that has an executable file called `name`.
fn find_in_path(name: &str, path_variable: &std::ffi::OsStr) -> Option<PathBuf> {
    env::split_paths(path_variable)
        .map(|directory| directory.join(name))
        .find(|candidate| is_executable_file(candidate))
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

pub fn tool_exists(name: &str) -> bool {
    env::var_os("PATH").and_then(|path| find_in_path(name, &path)).is_some()
}

/// The tools a build cannot do without that this system lacks, with what to install on Debian and Ubuntu.
pub fn missing_tools(has_tool: impl Fn(&str) -> bool) -> Vec<(&'static str, &'static str)> {
    let mut missing = Vec::new();
    if !has_tool("cc") && !has_tool("gcc") {
        missing.push(("a C compiler", "build-essential"));
    }
    if !has_tool("make") {
        missing.push(("make", "build-essential"));
    }
    missing
}

/// The last `count` lines of a log, for an error message.
pub fn tail_lines(text: &str, count: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(count)..].join("\n")
}

/// What to show of a failed build's log: the first errors, because with parallel jobs the end of
/// the log is usually warnings printed after the real failure. Without recognizable errors, its end.
pub fn failure_excerpt(log: &str) -> (&'static str, String) {
    let errors: Vec<&str> = log
        .lines()
        .filter(|line| line.contains(": error:") || line.contains("configure: error") || (line.starts_with("make") && line.contains("***")))
        .take(6)
        .collect();
    if errors.is_empty() {
        ("The end of", tail_lines(log, 20))
    } else {
        ("The first errors in", errors.join("\n"))
    }
}

/// Run one build step with its output going to the log. On failure the error carries the end of the log.
fn run_logged(mut command: Command, step: &str, log_path: &Path) -> Result<(), String> {
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|error| format!("opening {}: {}", log_path.display(), error))?;
    let status = command
        .stdout(Stdio::from(log.try_clone().map_err(|error| error.to_string())?))
        .stderr(Stdio::from(log))
        .status()
        .map_err(|error| format!("could not start {}: {}", step, error))?;
    if status.success() {
        return Ok(());
    }
    let (what, excerpt) = fs::read_to_string(log_path)
        .map(|text| failure_excerpt(&text))
        .unwrap_or(("The end of", String::new()));
    Err(format!("{} failed. {} {}:\n{}", step, what, log_path.display(), excerpt))
}

#[derive(Debug, PartialEq)]
pub enum Verdict {
    Good,
    Bad,
    Unknown,
}

/// What `gpg --verify` said.
pub fn signature_verdict(output: &str) -> Verdict {
    if output.contains("BAD signature") {
        Verdict::Bad
    } else if output.contains("Good signature") {
        Verdict::Good
    } else {
        Verdict::Unknown
    }
}

/// Check the archive against its detached signature with the keyring php.net publishes. Returns
/// false when gpg is not installed, so the caller can say that only the SHA-256 was checked.
pub fn verify_signature(release: &SourceRelease, archive: &Path, work_directory: &Path) -> Result<bool, String> {
    if !tool_exists("gpg") {
        return Ok(false);
    }
    let fetch = |url: &str, name: &str| -> Result<PathBuf, String> {
        let bytes = reqwest::blocking::get(url)
            .and_then(|response| response.error_for_status())
            .and_then(|response| response.bytes())
            .map_err(|error| format!("failed to download {}: {}", name, error))?;
        let path = work_directory.join(name);
        fs::write(&path, bytes).map_err(|error| format!("writing {}: {}", path.display(), error))?;
        Ok(path)
    };
    let signature = fetch(&release.signature_url(), "signature.asc")?;
    let keyring = fetch(KEYRING_URL, "php-keyring.gpg")?;

    let home = work_directory.join("gnupg");
    fs::create_dir_all(&home).map_err(|error| error.to_string())?;
    restrict_to_owner(&home);

    let gpg = |arguments: &[&std::ffi::OsStr]| -> Result<String, String> {
        let output = Command::new("gpg")
            .arg("--homedir")
            .arg(&home)
            .args(arguments)
            .output()
            .map_err(|error| format!("could not run gpg: {}", error))?;
        Ok(format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr)))
    };
    gpg(&["--quiet".as_ref(), "--import".as_ref(), keyring.as_os_str()])?;
    let verdict = signature_verdict(&gpg(&["--verify".as_ref(), signature.as_os_str(), archive.as_os_str()])?);
    match verdict {
        Verdict::Good => Ok(true),
        Verdict::Bad => Err("the signature of the PHP source is BAD; nothing was built".to_string()),
        Verdict::Unknown => Err("the signature of the PHP source could not be verified; nothing was built".to_string()),
    }
}

#[cfg(unix)]
fn restrict_to_owner(directory: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(directory, fs::Permissions::from_mode(0o700));
}

#[cfg(not(unix))]
fn restrict_to_owner(_directory: &Path) {}

/// Unpack the archive and return the directory of the source tree inside it.
pub fn unpack(archive: &Path, destination: &Path) -> Result<PathBuf, String> {
    let file = fs::File::open(archive).map_err(|error| error.to_string())?;
    tar::Archive::new(flate2::read::GzDecoder::new(file))
        .unpack(destination)
        .map_err(|error| format!("unpacking the source: {}", error))?;

    fs::read_dir(destination)
        .map_err(|error| error.to_string())?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.is_dir() && path.join("configure").exists())
        .ok_or_else(|| "the source archive does not contain a configure script".to_string())
}

/// Configure, compile and install into a staging root, with the final directory as the prefix so
/// that the paths built into php (the extension directory, php-config) are the final ones.
/// Returns where the installed tree is inside the staging root.
pub fn compile(
    source: &Path,
    final_directory: &Path,
    staging_root: &Path,
    plan: &Plan,
    jobs: usize,
    log_path: &Path,
) -> Result<PathBuf, String> {
    let mut configure = Command::new(source.join("configure"));
    configure
        .current_dir(source)
        .arg(format!("--prefix={}", final_directory.display()))
        .args(&plan.flags);
    run_logged(configure, "configure", log_path)?;

    let mut make = Command::new("make");
    make.current_dir(source).arg(format!("-j{}", jobs));
    run_logged(make, "compiling", log_path)?;

    let mut install = Command::new("make");
    install.current_dir(source).arg("install").arg(format!("INSTALL_ROOT={}", staging_root.display()));
    run_logged(install, "installing", log_path)?;

    // INSTALL_ROOT puts the tree at <root><prefix>, with the prefix's leading slash dropped.
    let relative = final_directory.strip_prefix("/").unwrap_or(final_directory);
    Ok(staging_root.join(relative))
}

#[cfg(test)]
mod tests;
