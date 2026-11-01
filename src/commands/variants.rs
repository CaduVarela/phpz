use std::collections::BTreeSet;

use crate::variant::{self, Variant};
use crate::version;

/// Show the builds of PHP that can be installed and what each one has
pub fn run() {
    if let Err(error) = show() {
        eprintln!("Error: {}", error);
        std::process::exit(1);
    }
}

fn show() -> Result<(), String> {
    if cfg!(windows) {
        println!("Builds are a Linux and macOS choice. Windows has one official build per version.");
        return Ok(());
    }

    let default_extensions = version::variant_extensions(variant::DEFAULT)?;
    println!("Builds of PHP for this system (phpz install <version> --variant <name>):\n");

    for candidate in Variant::ALL {
        let name = format!("{}{}", candidate.name(), if candidate == variant::DEFAULT { " (default)" } else { "" });
        if let Err(reason) = variant::runs_here(candidate) {
            println!("{:<17} not available here: {}", name, reason);
            continue;
        }
        let extensions = match version::variant_extensions(candidate) {
            Ok(extensions) => extensions,
            Err(error) => {
                println!("{:<17} could not be read: {}", name, error);
                continue;
            }
        };
        let size = version::newest_build(candidate)
            .ok()
            .and_then(|build| build.size)
            .unwrap_or_else(|| "?".to_string());
        println!(
            "{:<17} {:>3} extensions, {:>6}   {}",
            name,
            extensions.len(),
            size,
            describe_difference(&extensions, &default_extensions)
        );
    }
    println!("\nA larger build is never installed unless you ask, or agree when phpz asks.");
    Ok(())
}

/// What a build has and lacks compared to the default, with long lists cut short.
fn describe_difference(extensions: &BTreeSet<String>, default_extensions: &BTreeSet<String>) -> String {
    let adds: Vec<&String> = extensions.difference(default_extensions).collect();
    let lacks: Vec<&String> = default_extensions.difference(extensions).collect();
    if adds.is_empty() && lacks.is_empty() {
        return "the default's extensions".to_string();
    }

    let mut parts = Vec::new();
    if !adds.is_empty() {
        parts.push(format!("adds {}", shorten(&adds, 6)));
    }
    if !lacks.is_empty() {
        parts.push(format!("lacks {}", shorten(&lacks, 4)));
    }
    parts.join("; ")
}

fn shorten(names: &[&String], limit: usize) -> String {
    let shown: Vec<&str> = names.iter().take(limit).map(|name| name.as_str()).collect();
    if names.len() > limit {
        format!("{} and {} more", shown.join(", "), names.len() - limit)
    } else {
        shown.join(", ")
    }
}

#[cfg(test)]
mod tests;
