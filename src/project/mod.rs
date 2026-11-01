pub mod ini;
pub mod shim;

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::extensions::extension_name;
use crate::extensions::pecl;
use crate::extensions::settings::ExtensionSettings;

/// Holds only a version, so tools that already read this file keep working.
pub const VERSION_FILE: &str = ".php-version";

/// Holds a version and the extensions of the project, and wins over `.php-version`.
pub const PROJECT_FILE: &str = ".phpz.toml";

/// A PHP version a project file asks for, as written: "8", "8.3" or "8.3.9".
#[derive(Debug, PartialEq, Clone)]
pub struct Pinned {
    pub request: String,
    pub file: PathBuf,
}

#[derive(Debug, PartialEq)]
pub struct Project {
    pub directory: PathBuf,
    pub php: Option<Pinned>,
    pub extensions: ExtensionSettings,
}

#[derive(Debug, Deserialize, Default)]
struct ProjectFile {
    php: Option<String>,
    #[serde(default)]
    extensions: ExtensionSettings,
}

/// "8", "8.3" or "8.3.9": one to three numbers separated by dots.
pub fn is_valid_request(text: &str) -> bool {
    let parts: Vec<&str> = text.split('.').collect();
    parts.len() <= 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
}

/// A version as people write it in `.php-version` ("8.3", "v8.3", "php-8.3"), or None if it is not one.
pub fn normalize_request(text: &str) -> Option<String> {
    let text = text.trim();
    let version = text
        .strip_prefix("php-")
        .or_else(|| text.strip_prefix('v'))
        .unwrap_or(text);
    is_valid_request(version).then(|| version.to_string())
}

/// The version in a `.php-version` file: the first line that is not blank or a comment.
fn parse_version_file(text: &str, file: &Path) -> Result<Option<String>, String> {
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        return normalize_request(line)
            .map(Some)
            .ok_or_else(|| format!("invalid PHP version '{}' in {}", line, file.display()));
    }
    Ok(None)
}

/// Extension names as plain lowercase names; paths are rejected.
fn clean_names(names: Vec<String>, file: &Path) -> Result<Vec<String>, String> {
    let mut cleaned = Vec::new();
    for name in names {
        let normalized = extension_name(&name);
        if !pecl::is_valid_name(&normalized) || name.contains(['/', '\\']) {
            return Err(format!(
                "invalid extension name '{}' in {}",
                name,
                file.display()
            ));
        }
        if !cleaned.contains(&normalized) {
            cleaned.push(normalized);
        }
    }
    Ok(cleaned)
}

fn parse_project_file(text: &str, file: &Path) -> Result<ProjectFile, String> {
    let mut parsed: ProjectFile =
        toml::from_str(text).map_err(|error| format!("invalid {}: {}", file.display(), error))?;
    if let Some(php) = &parsed.php {
        if !is_valid_request(php) {
            return Err(format!(
                "invalid PHP version '{}' in {}",
                php,
                file.display()
            ));
        }
    }
    parsed.extensions.enable = clean_names(std::mem::take(&mut parsed.extensions.enable), file)?;
    parsed.extensions.disable = clean_names(std::mem::take(&mut parsed.extensions.disable), file)?;
    Ok(parsed)
}

fn read_optional(path: &Path) -> Result<Option<String>, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("reading {}: {}", path.display(), error)),
    }
}

/// The project defined by the files in `directory` itself.
fn read_directory(directory: &Path) -> Result<Option<Project>, String> {
    let project_path = directory.join(PROJECT_FILE);
    let version_path = directory.join(VERSION_FILE);
    let project_text = read_optional(&project_path)?;
    let version_text = read_optional(&version_path)?;

    let mut project = Project {
        directory: directory.to_path_buf(),
        php: None,
        extensions: ExtensionSettings::default(),
    };

    let has_project_file = project_text.is_some();
    if let Some(text) = project_text {
        let parsed = parse_project_file(&text, &project_path)?;
        project.php = parsed.php.map(|request| Pinned {
            request,
            file: project_path.clone(),
        });
        project.extensions = parsed.extensions;
    }
    if project.php.is_none() {
        if let Some(text) = version_text {
            project.php = parse_version_file(&text, &version_path)?.map(|request| Pinned {
                request,
                file: version_path.clone(),
            });
        }
    }

    Ok((has_project_file || project.php.is_some()).then_some(project))
}

/// The nearest project at or above `start`.
pub fn find(start: &Path) -> Result<Option<Project>, String> {
    let mut directory = Some(start);
    while let Some(current) = directory {
        if let Some(project) = read_directory(current)? {
            return Ok(Some(project));
        }
        directory = current.parent();
    }
    Ok(None)
}

/// Whether `directory` has a `.phpz.toml` that already picks a PHP version.
pub fn project_file_pins_php(directory: &Path) -> Result<bool, String> {
    let path = directory.join(PROJECT_FILE);
    match read_optional(&path)? {
        Some(text) => Ok(parse_project_file(&text, &path)?.php.is_some()),
        None => Ok(false),
    }
}

#[cfg(test)]
mod tests;
