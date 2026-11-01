/// Variables that make PHP read configuration from outside the version's directory.
const INI_VARIABLES: &[(&str, &str)] = &[
    (
        "PHP_INI_SCAN_DIR",
        "PHP also reads .ini files from there, which can load extensions twice and print warnings.",
    ),
    (
        "PHPRC",
        "PHP looks for php.ini there instead of in the version's own directory.",
    ),
];

#[derive(Debug, PartialEq)]
pub struct IniOverride {
    pub name: &'static str,
    pub value: String,
    pub effect: &'static str,
}

pub fn find_overrides(lookup: impl Fn(&str) -> Option<String>) -> Vec<IniOverride> {
    INI_VARIABLES
        .iter()
        .filter_map(|&(name, effect)| {
            let value = lookup(name).filter(|value| !value.is_empty())?;
            Some(IniOverride {
                name,
                value,
                effect,
            })
        })
        .collect()
}

pub fn describe(found: &IniOverride) -> String {
    format!(
        "Warning: {name} is set to \"{value}\".\n  {effect}\n  \
         To use phpz's PHP cleanly, clear it in this terminal:\n    \
         $env:{name} = $null\n  \
         To remove it for good (this also affects any other PHP installation that relies on it):\n    \
         [Environment]::SetEnvironmentVariable(\"{name}\", $null, \"User\")",
        name = found.name,
        value = found.value,
        effect = found.effect,
    )
}

/// Warn about variables that break the isolation between versions (Windows only).
pub fn warn_about_ini_overrides() {
    if !cfg!(windows) {
        return;
    }
    for found in find_overrides(|name| std::env::var(name).ok()) {
        eprintln!("{}", describe(&found));
    }
}
