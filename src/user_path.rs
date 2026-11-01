// Only the Windows registry code calls the text helpers, but they are tested everywhere.
#![cfg_attr(not(windows), allow(dead_code))]

const SEPARATOR: char = ';';

/// Entries are the same when they differ only by case or a trailing backslash.
fn same_entry(left: &str, right: &str) -> bool {
    let normalize = |entry: &str| entry.trim().trim_end_matches(['\\', '/']).to_lowercase();
    normalize(left) == normalize(right)
}

fn contains_entry(path: &str, entry: &str) -> bool {
    path.split(SEPARATOR).any(|existing| same_entry(existing, entry))
}

/// `path` with `entry` placed first, or None when it is already there.
fn add_entry(path: &str, entry: &str) -> Option<String> {
    if contains_entry(path, entry) {
        return None;
    }
    Some(if path.is_empty() {
        entry.to_string()
    } else {
        format!("{}{}{}", entry, SEPARATOR, path)
    })
}

/// `path` without `entry`, or None when it was not there.
fn remove_entry(path: &str, entry: &str) -> Option<String> {
    if !contains_entry(path, entry) {
        return None;
    }
    let remaining: Vec<&str> = path
        .split(SEPARATOR)
        .filter(|existing| !same_entry(existing, entry))
        .collect();
    Some(remaining.join(&SEPARATOR.to_string()))
}

#[cfg(windows)]
mod registry {
    use std::borrow::Cow;

    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
    };
    use winreg::enums::{RegType, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_EXPAND_SZ};
    use winreg::{RegKey, RegValue};

    /// The stored text plus its registry type, so a write puts back the same kind of value.
    pub struct StoredPath {
        pub text: String,
        pub kind: RegType,
    }

    const SUBKEY: &str = "Environment";
    const VALUE_NAME: &str = "Path";

    pub fn read() -> Result<StoredPath, String> {
        let key = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(SUBKEY, KEY_READ)
            .map_err(|error| format!("opening the user environment: {}", error))?;

        match key.get_raw_value(VALUE_NAME) {
            Ok(value) => Ok(StoredPath {
                text: decode(&value.bytes),
                kind: value.vtype,
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(StoredPath {
                text: String::new(),
                kind: REG_EXPAND_SZ,
            }),
            Err(error) => Err(format!("reading the user PATH: {}", error)),
        }
    }

    pub fn write(text: &str, kind: RegType) -> Result<(), String> {
        let key = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(SUBKEY, KEY_READ | KEY_WRITE)
            .map_err(|error| format!("opening the user environment: {}", error))?;
        let value = RegValue {
            bytes: Cow::Owned(encode(text)),
            vtype: kind,
        };
        key.set_raw_value(VALUE_NAME, &value)
            .map_err(|error| format!("writing the user PATH: {}", error))?;
        notify_environment_changed();
        Ok(())
    }

    /// Registry strings are UTF-16 with a terminating NUL.
    fn decode(bytes: &[u8]) -> String {
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        String::from_utf16_lossy(&units).trim_end_matches('\0').to_string()
    }

    fn encode(text: &str) -> Vec<u8> {
        text.encode_utf16()
            .chain(std::iter::once(0))
            .flat_map(u16::to_le_bytes)
            .collect()
    }

    /// Tell running programs (Explorer, new terminals) that the environment changed.
    fn notify_environment_changed() {
        let parameter: Vec<u16> = "Environment\0".encode_utf16().collect();
        let mut result: usize = 0;
        // SAFETY: `parameter` is a NUL-terminated UTF-16 string that outlives the call,
        // and `result` is a valid pointer for the duration of it.
        unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0,
                parameter.as_ptr() as isize,
                SMTO_ABORTIFHUNG,
                5000,
                &mut result,
            );
        }
    }
}

#[cfg(windows)]
pub fn is_configured(entry: &str) -> bool {
    registry::read()
        .map(|stored| contains_entry(&stored.text, entry))
        .unwrap_or(false)
}

/// Put `entry` first on the user PATH. Returns false when it was already there.
#[cfg(windows)]
pub fn configure(entry: &str) -> Result<bool, String> {
    let stored = registry::read()?;
    match add_entry(&stored.text, entry) {
        Some(updated) => registry::write(&updated, stored.kind).map(|_| true),
        None => Ok(false),
    }
}

/// Take `entry` off the user PATH. Returns false when it was not there.
#[cfg(windows)]
pub fn unconfigure(entry: &str) -> Result<bool, String> {
    let stored = registry::read()?;
    match remove_entry(&stored.text, entry) {
        Some(updated) => registry::write(&updated, stored.kind).map(|_| true),
        None => Ok(false),
    }
}

#[cfg(not(windows))]
pub fn is_configured(_entry: &str) -> bool {
    false
}

#[cfg(not(windows))]
pub fn configure(_entry: &str) -> Result<bool, String> {
    Err("the user PATH is only available on Windows".to_string())
}

#[cfg(not(windows))]
pub fn unconfigure(_entry: &str) -> Result<bool, String> {
    Err("the user PATH is only available on Windows".to_string())
}

#[cfg(test)]
mod tests;
