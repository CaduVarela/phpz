use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// File in a version directory that records which build it came from.
const FILE_NAME: &str = ".phpz-variant";

/// The builds static-php-cli publishes, smallest download first (so the derived order is the size order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Variant {
    /// A handful of core extensions.
    Minimal,
    /// The usual web stack. Fully static.
    Common,
    /// Many more extensions (intl, opcache, imagick...). Fully static, but it lacks some `common` has.
    Bulk,
    /// The same extensions as `bulk`, built dynamically against glibc so it can load `.so` files.
    GnuBulk,
}

/// What `phpz install` gets without `--variant`.
pub const DEFAULT: Variant = Variant::Common;

impl Variant {
    pub const ALL: [Variant; 4] = [
        Variant::Minimal,
        Variant::Common,
        Variant::Bulk,
        Variant::GnuBulk,
    ];

    /// The name used on the command line and in the server's directory.
    pub fn name(self) -> &'static str {
        match self {
            Variant::Minimal => "minimal",
            Variant::Common => "common",
            Variant::Bulk => "bulk",
            Variant::GnuBulk => "gnu-bulk",
        }
    }

    pub fn parse(text: &str) -> Option<Variant> {
        match text.trim().to_lowercase().replace('_', "-").as_str() {
            "minimal" => Some(Variant::Minimal),
            "common" => Some(Variant::Common),
            "bulk" => Some(Variant::Bulk),
            "gnu-bulk" | "gnubulk" => Some(Variant::GnuBulk),
            _ => None,
        }
    }

    /// For clap: a name that is not a build lists the ones that are.
    pub fn parse_argument(text: &str) -> Result<Variant, String> {
        Variant::parse(text).ok_or_else(|| {
            format!(
                "unknown build '{}'. Choose one of: {}",
                text,
                Variant::names()
            )
        })
    }

    pub fn names() -> String {
        Variant::ALL
            .iter()
            .map(|variant| variant.name())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Whether a build can run on a system, or why not.
fn runs_on(variant: Variant, operating_system: &str, musl: bool) -> Result<(), String> {
    if variant != Variant::GnuBulk {
        return Ok(());
    }
    if operating_system != "linux" {
        return Err("the gnu-bulk build exists for Linux only".to_string());
    }
    if musl {
        return Err(
            "the gnu-bulk build needs glibc, and this system uses musl (Alpine)".to_string(),
        );
    }
    Ok(())
}

/// A musl system (Alpine) has a loader named /lib/ld-musl-*.
fn is_musl() -> bool {
    fs::read_dir("/lib")
        .map(|entries| {
            entries
                .flatten()
                .any(|entry| entry.file_name().to_string_lossy().starts_with("ld-musl-"))
        })
        .unwrap_or(false)
}

pub fn runs_here(variant: Variant) -> Result<(), String> {
    runs_on(variant, std::env::consts::OS, is_musl())
}

/// Where a version came from: one of the prebuilt static builds, or compiled here from the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Prebuilt(Variant),
    Source,
}

impl Origin {
    pub fn name(self) -> &'static str {
        match self {
            Origin::Prebuilt(variant) => variant.name(),
            Origin::Source => "source",
        }
    }
}

/// Where a version directory came from; no record means the default prebuilt build.
pub fn read_origin(version_directory: &Path) -> Origin {
    match fs::read_to_string(version_directory.join(FILE_NAME)) {
        Ok(text) if text.trim() == "source" => Origin::Source,
        Ok(text) => Origin::Prebuilt(Variant::parse(&text).unwrap_or(DEFAULT)),
        Err(_) => Origin::Prebuilt(DEFAULT),
    }
}

pub fn write_source(version_directory: &Path) -> Result<(), String> {
    let path = version_directory.join(FILE_NAME);
    fs::write(&path, "source\n").map_err(|error| format!("writing {}: {}", path.display(), error))
}

pub fn write(version_directory: &Path, variant: Variant) -> Result<(), String> {
    let path = version_directory.join(FILE_NAME);
    fs::write(&path, format!("{}\n", variant.name()))
        .map_err(|error| format!("writing {}: {}", path.display(), error))
}

/// The smallest build among `candidates` that has every wanted extension.
pub fn smallest_covering(
    candidates: &[(Variant, BTreeSet<String>)],
    wanted: &BTreeSet<String>,
) -> Option<Variant> {
    candidates
        .iter()
        .filter(|(_, extensions)| wanted.is_subset(extensions))
        .map(|(variant, _)| *variant)
        .min()
}

/// Extensions the current build has that the target lacks: what moving to it would lose.
pub fn lost_by_switching(current: &BTreeSet<String>, target: &BTreeSet<String>) -> Vec<String> {
    current.difference(target).cloned().collect()
}

#[cfg(test)]
mod tests;
