use std::collections::BTreeMap;

use crate::variant::{self, Variant};
use crate::{installed, version};

/// List versions available for download (newest patch of each minor unless `all`).
pub fn run(all: bool, variant: Option<Variant>) {
    if let Err(error) = list_remote(all, variant) {
        eprintln!("Error: {}", error);
        std::process::exit(1);
    }
}

fn list_remote(all: bool, variant: Option<Variant>) -> Result<(), String> {
    if cfg!(windows) && variant.is_some() {
        return Err("--variant is for Linux and macOS. Windows has one official build per version.".to_string());
    }
    let available = version::available_version_names(all, variant.unwrap_or(variant::DEFAULT))?;
    let installed_versions = installed::installed_versions()?;

    let shown: Vec<String> = if all {
        available
    } else {
        let mut newest_per_minor: BTreeMap<Vec<u32>, String> = BTreeMap::new();
        for name in available {
            let mut key = installed::version_sort_key(&name);
            key.truncate(2);
            // Input is sorted ascending, so later entries are newer patches.
            newest_per_minor.insert(key, name);
        }
        newest_per_minor.into_values().collect()
    };

    for name in shown {
        if installed_versions.contains(&name) {
            println!("{} (installed)", name);
        } else {
            println!("{}", name);
        }
    }
    Ok(())
}
