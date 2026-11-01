use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ini::{IniDocument, PHP_SECTION};
use crate::settings::{ExtensionSettings, Settings};

/// Extensions PHP loads with `zend_extension` instead of `extension`.
const ZEND_EXTENSIONS: &[&str] = &["opcache", "xdebug"];

/// "php_curl.dll", "C:\php\ext\php_curl.dll", "curl.so" and "curl" are all the extension "curl".
/// PHP 7.0 to 7.3 call GD "gd2" in php.ini and ship php_gd2.dll, so "gd2" is "gd".
pub fn extension_name(value: &str) -> String {
    let file = value.trim().trim_matches('"').rsplit(['/', '\\']).next().unwrap_or("");
    let lower = file.to_lowercase();
    let without_prefix = lower.strip_prefix("php_").unwrap_or(&lower);
    let stem = without_prefix
        .strip_suffix(".dll")
        .or_else(|| without_prefix.strip_suffix(".so"))
        .unwrap_or(without_prefix);
    match stem {
        "gd2" => "gd".to_string(),
        other => other.to_string(),
    }
}

fn key_for(name: &str) -> &'static str {
    if ZEND_EXTENSIONS.contains(&name) {
        "zend_extension"
    } else {
        "extension"
    }
}

/// Extensions a version can switch on and off: the DLLs in its `ext` directory (Windows layout),
/// or the `.so` files of a build compiled here (`lib/php/extensions/<api>/`). A prebuilt static
/// build has none.
pub fn available_shared(version_directory: &Path) -> BTreeSet<String> {
    let mut found: BTreeSet<String> = fs::read_dir(version_directory.join("ext"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|file| file.to_lowercase().starts_with("php_") && file.to_lowercase().ends_with(".dll"))
        .map(|file| extension_name(&file))
        .collect();

    for api_directory in fs::read_dir(version_directory.join("lib/php/extensions")).into_iter().flatten().flatten() {
        found.extend(
            fs::read_dir(api_directory.path())
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|file| file.ends_with(".so"))
                .map(|file| extension_name(&file)),
        );
    }
    found
}

pub fn php_executable(version_directory: &Path) -> PathBuf {
    if cfg!(windows) {
        version_directory.join("php.exe")
    } else {
        version_directory.join("bin").join("php")
    }
}

/// Everything PHP reports as loaded (`php -m`): enabled shared extensions and built-in ones.
/// PHP runs with this version's own php.ini, whatever is set in the environment.
pub fn loaded_modules(version_directory: &Path) -> Result<BTreeSet<String>, String> {
    let output = Command::new(php_executable(version_directory))
        .arg("-m")
        .env_remove("PHP_INI_SCAN_DIR")
        .env("PHPRC", version_directory)
        .output()
        .map_err(|error| format!("could not run PHP: {}", error))?;
    if !output.status.success() {
        return Err("PHP failed to list its modules".to_string());
    }
    Ok(parse_modules(&String::from_utf8_lossy(&output.stdout)))
}

/// Module names from `php -m`, lowercase, skipping "[PHP Modules]" headers and any warnings.
fn parse_modules(output: &str) -> BTreeSet<String> {
    const NOISE: &[&str] = &["Warning", "Notice", "Deprecated", "Fatal", "PHP "];

    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('['))
        .filter(|line| !NOISE.iter().any(|prefix| line.starts_with(prefix)))
        .map(|line| match line.to_lowercase().as_str() {
            "zend opcache" => "opcache".to_string(),
            other => other.to_string(),
        })
        .collect()
}

/// Extensions the php.ini switches on.
pub fn enabled_in_ini(ini: &IniDocument) -> BTreeSet<String> {
    ["extension", "zend_extension"]
        .iter()
        .flat_map(|key| ini.values(PHP_SECTION, key))
        .map(|value| extension_name(&value))
        .collect()
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Changed,
    Unchanged,
    /// Compiled into PHP: always on and impossible to switch off.
    BuiltIn,
    Unavailable,
}

/// How a new php.ini line spells an extension. PHP 7.0 and 7.1 name extensions by DLL
/// ("extension=php_curl.dll"), later versions use short names, and PHP 7.0 to 7.3 call GD "gd2".
fn entry_value(ini: &IniDocument, name: &str) -> String {
    let text = ini.render();
    let token = if name == "gd" && text.contains("gd2") { "gd2" } else { name };

    if text.contains("extension=php_") {
        format!("php_{}.dll", token)
    } else {
        token.to_string()
    }
}

/// Switch an extension on. `loaded` only runs when the extension is not one of the shared ones.
pub fn enable(
    ini: &mut IniDocument,
    name: &str,
    available: &BTreeSet<String>,
    loaded: impl FnOnce() -> Result<BTreeSet<String>, String>,
) -> Result<Outcome, String> {
    let name = extension_name(name);

    if !available.contains(&name) {
        return Ok(if loaded()?.contains(&name) {
            Outcome::BuiltIn
        } else {
            Outcome::Unavailable
        });
    }
    if enabled_in_ini(ini).contains(&name) {
        return Ok(Outcome::Unchanged);
    }

    let matches = |value: &str| extension_name(value) == name;
    let key = key_for(&name);
    if !ini.uncomment_entry(PHP_SECTION, key, matches) {
        let value = entry_value(ini, &name);
        ini.add_entry_near(PHP_SECTION, key, &value);
    }
    Ok(Outcome::Changed)
}

/// Switch an extension off by commenting its line out, so the line stays where the template had it.
pub fn disable(
    ini: &mut IniDocument,
    name: &str,
    available: &BTreeSet<String>,
    loaded: impl FnOnce() -> Result<BTreeSet<String>, String>,
) -> Result<Outcome, String> {
    let name = extension_name(name);

    if !available.contains(&name) {
        return Ok(if loaded()?.contains(&name) {
            Outcome::BuiltIn
        } else {
            Outcome::Unavailable
        });
    }

    let matches = |value: &str| extension_name(value) == name;
    let changed = ini.comment_out_entries(PHP_SECTION, "extension", matches)
        + ini.comment_out_entries(PHP_SECTION, "zend_extension", matches);
    Ok(if changed > 0 { Outcome::Changed } else { Outcome::Unchanged })
}

#[derive(Debug, PartialEq)]
pub struct Listing {
    pub enabled: Vec<String>,
    pub disabled: Vec<String>,
    /// Switched on in php.ini but PHP did not load it (missing file, wrong build, ...).
    pub failing: Vec<String>,
    pub built_in: Vec<String>,
}

pub fn classify(
    available: &BTreeSet<String>,
    loaded: &BTreeSet<String>,
    in_ini: &BTreeSet<String>,
) -> Listing {
    let names = |set: Vec<&String>| set.into_iter().cloned().collect::<Vec<String>>();

    Listing {
        enabled: names(available.iter().filter(|name| loaded.contains(*name)).collect()),
        failing: names(
            available
                .iter()
                .filter(|name| !loaded.contains(*name) && in_ini.contains(*name))
                .collect(),
        ),
        disabled: names(
            available
                .iter()
                .filter(|name| !loaded.contains(*name) && !in_ini.contains(*name))
                .collect(),
        ),
        built_in: names(loaded.iter().filter(|name| !available.contains(*name)).collect()),
    }
}

/// Why an extension was skipped because the version does not have it.
pub const NOT_AVAILABLE: &str = "not available in this PHP";

/// Of the wanted `names`, those this PHP has neither as a shared extension nor built in.
/// `loaded` only runs when some name is not a shared extension.
pub fn missing_extensions(
    names: &[String],
    available: &BTreeSet<String>,
    loaded: impl FnOnce() -> Result<BTreeSet<String>, String>,
) -> Result<Vec<String>, String> {
    let not_shared: Vec<&String> = names.iter().filter(|name| !available.contains(*name)).collect();
    if not_shared.is_empty() {
        return Ok(Vec::new());
    }
    let loaded = loaded()?;
    Ok(not_shared.into_iter().filter(|name| !loaded.contains(*name)).cloned().collect())
}

/// What applying the global settings to one PHP version did.
#[derive(Debug, Default, PartialEq)]
pub struct SyncReport {
    pub enabled: Vec<String>,
    pub disabled: Vec<String>,
    /// Extensions that could not be applied here, with the reason.
    pub skipped: Vec<(String, String)>,
}

impl SyncReport {
    /// Something was skipped only because the extension is not installed in this PHP.
    pub fn has_unavailable(&self) -> bool {
        self.skipped.iter().any(|(_, reason)| reason == NOT_AVAILABLE)
    }

    pub fn changed(&self) -> bool {
        !self.enabled.is_empty() || !self.disabled.is_empty()
    }

    /// One line per kind of result, e.g. "enabled: intl, opcache".
    pub fn describe(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if !self.enabled.is_empty() {
            lines.push(format!("enabled: {}", self.enabled.join(", ")));
        }
        if !self.disabled.is_empty() {
            lines.push(format!("disabled: {}", self.disabled.join(", ")));
        }
        for (name, reason) in &self.skipped {
            lines.push(format!("skipped {}: {}", name, reason));
        }
        lines
    }
}

/// Make `ini` follow `settings`. An extension this PHP does not have is skipped, never an error,
/// because the same settings are applied to every version.
pub fn apply_settings(
    ini: &mut IniDocument,
    settings: &ExtensionSettings,
    available: &BTreeSet<String>,
    mut loaded: impl FnMut() -> Result<BTreeSet<String>, String>,
) -> Result<SyncReport, String> {
    let mut report = SyncReport::default();

    for name in &settings.enable {
        match enable(ini, name, available, &mut loaded)? {
            Outcome::Changed => report.enabled.push(name.clone()),
            Outcome::Unchanged | Outcome::BuiltIn => {}
            Outcome::Unavailable => report.skipped.push((name.clone(), NOT_AVAILABLE.to_string())),
        }
    }
    for name in &settings.disable {
        match disable(ini, name, available, &mut loaded)? {
            Outcome::Changed => report.disabled.push(name.clone()),
            // Nothing to switch off when this PHP does not have it.
            Outcome::Unchanged | Outcome::Unavailable => {}
            Outcome::BuiltIn => report
                .skipped
                .push((name.clone(), "built into this PHP, cannot be disabled".to_string())),
        }
    }
    Ok(report)
}

/// Apply the saved global settings to the PHP in `version_directory` and save its php.ini
/// if anything changed. None when there are no settings to apply.
pub fn apply_global_settings(version_directory: &Path) -> Result<Option<SyncReport>, String> {
    let settings = Settings::load()?;
    if settings.extensions.is_empty() {
        return Ok(None);
    }

    let ini_path = version_directory.join("php.ini");
    let mut ini = IniDocument::load(&ini_path)
        .map_err(|error| format!("reading {}: {}", ini_path.display(), error))?;
    let available = available_shared(version_directory);

    // `php -m` is slow enough to run at most once, and only if a built-in check needs it.
    let mut modules: Option<BTreeSet<String>> = None;
    let loaded = || {
        if modules.is_none() {
            modules = Some(loaded_modules(version_directory)?);
        }
        Ok(modules.clone().unwrap_or_default())
    };

    let report = apply_settings(&mut ini, &settings.extensions, &available, loaded)?;
    if report.changed() {
        ini.save(&ini_path)
            .map_err(|error| format!("writing {}: {}", ini_path.display(), error))?;
    }
    Ok(Some(report))
}

/// Names joined with ", " and wrapped into lines no longer than `width`.
pub fn wrap_names(names: &[String], width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for name in names {
        match lines.last_mut() {
            Some(line) if line.len() + 2 + name.len() <= width => {
                line.push_str(", ");
                line.push_str(name);
            }
            _ => lines.push(name.clone()),
        }
    }
    lines
}

#[cfg(test)]
mod tests;
