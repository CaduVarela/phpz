//! Line-preserving php.ini editor.
//!
//! The file is kept as a list of lines. Only the lines touched by `set` and
//! `remove` change, so comments, ordering, blank lines and repeated keys
//! (such as `extension=`) survive every edit.

use std::fs;
use std::io;
use std::path::Path;

/// Section that holds regular PHP settings.
pub const PHP_SECTION: &str = "PHP";

pub struct IniDocument {
    lines: Vec<String>,
    newline: &'static str,
}

enum LineKind<'a> {
    Section(&'a str),
    Entry { key: &'a str, raw_value: &'a str },
    Other,
}

fn classify(line: &str) -> LineKind<'_> {
    let trimmed = line.trim();

    if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
        return LineKind::Other;
    }

    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return LineKind::Section(trimmed[1..trimmed.len() - 1].trim());
    }

    match trimmed.split_once('=') {
        Some((key, raw_value)) => LineKind::Entry {
            key: key.trim(),
            raw_value: raw_value.trim(),
        },
        None => LineKind::Other,
    }
}

/// Turn the raw text after `=` into the effective value:
/// surrounding quotes are removed and trailing `; comments` are dropped.
fn effective_value(raw_value: &str) -> String {
    let mut characters = raw_value.chars();
    if let Some(quote) = characters.next().filter(|c| *c == '"' || *c == '\'') {
        let rest = characters.as_str();
        return match rest.find(quote) {
            Some(end) => rest[..end].to_string(),
            None => rest.to_string(),
        };
    }

    match raw_value.find(';') {
        Some(position) => raw_value[..position].trim().to_string(),
        None => raw_value.to_string(),
    }
}

impl IniDocument {
    pub fn parse(content: &str) -> Self {
        let newline = if content.contains("\r\n") { "\r\n" } else { "\n" };
        Self {
            lines: content.lines().map(str::to_string).collect(),
            newline,
        }
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        Ok(Self::parse(&fs::read_to_string(path)?))
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let temporary_path = path.with_extension("ini.tmp");
        fs::write(&temporary_path, self.render())?;
        fs::rename(&temporary_path, path)
    }

    pub fn render(&self) -> String {
        let mut output = self.lines.join(self.newline);
        if !self.lines.is_empty() {
            output.push_str(self.newline);
        }
        output
    }

    /// Indexes of the active (not commented) lines that define `key` in `section`.
    fn find_entries(&self, section: &str, key: &str) -> Vec<usize> {
        let mut current_section = "";
        let mut found = Vec::new();

        for (index, line) in self.lines.iter().enumerate() {
            match classify(line) {
                LineKind::Section(name) => current_section = name,
                LineKind::Entry { key: line_key, .. }
                    if current_section == section && line_key == key =>
                {
                    found.push(index)
                }
                _ => {}
            }
        }
        found
    }

    /// Effective value of `key`. As in PHP, the last definition wins.
    pub fn get(&self, section: &str, key: &str) -> Option<String> {
        let index = *self.find_entries(section, key).last()?;
        match classify(&self.lines[index]) {
            LineKind::Entry { raw_value, .. } => Some(effective_value(raw_value)),
            _ => None,
        }
    }

    /// Set `key` to `value` (written exactly as given, so quotes are the caller's choice).
    pub fn set(&mut self, section: &str, key: &str, value: &str) {
        let new_line = format!("{} = {}", key, value);

        if let Some(&index) = self.find_entries(section, key).last() {
            self.lines[index] = new_line;
            return;
        }

        match self.section_insertion_index(section) {
            Some(index) => self.lines.insert(index, new_line),
            None => {
                if self.lines.last().map(|line| !line.trim().is_empty()).unwrap_or(false) {
                    self.lines.push(String::new());
                }
                self.lines.push(format!("[{}]", section));
                self.lines.push(new_line);
            }
        }
    }

    /// Remove every active definition of `key`. Returns how many lines were removed.
    pub fn remove(&mut self, section: &str, key: &str) -> usize {
        let indexes = self.find_entries(section, key);
        for index in indexes.iter().rev() {
            self.lines.remove(*index);
        }
        indexes.len()
    }

    /// Where a new key goes: right after the last non-blank line of the section.
    fn section_insertion_index(&self, section: &str) -> Option<usize> {
        let mut insertion_index = None;
        let mut inside_section = false;

        for (index, line) in self.lines.iter().enumerate() {
            match classify(line) {
                LineKind::Section(name) => {
                    inside_section = name == section;
                    if inside_section {
                        insertion_index = Some(index + 1);
                    }
                }
                _ if inside_section && !line.trim().is_empty() => {
                    insertion_index = Some(index + 1);
                }
                _ => {}
            }
        }
        insertion_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
; main settings
[PHP]
memory_limit = 128M ; old value
extension=curl
extension=gd

; a comment that must survive
display_errors = Off

[Date]
date.timezone = UTC
";

    #[test]
    fn render_round_trips_untouched_content() {
        assert_eq!(IniDocument::parse(SAMPLE).render(), SAMPLE);
    }

    #[test]
    fn get_strips_inline_comments_and_quotes() {
        let document = IniDocument::parse("[PHP]\na = 1 ; note\nb = \"two words\"\n");
        assert_eq!(document.get("PHP", "a").as_deref(), Some("1"));
        assert_eq!(document.get("PHP", "b").as_deref(), Some("two words"));
        assert_eq!(document.get("PHP", "missing"), None);
    }

    #[test]
    fn get_ignores_commented_lines_and_other_sections() {
        let document = IniDocument::parse("[PHP]\n;a = 1\n[Date]\na = 2\n");
        assert_eq!(document.get("PHP", "a"), None);
        assert_eq!(document.get("Date", "a").as_deref(), Some("2"));
    }

    #[test]
    fn set_replaces_existing_line_and_keeps_everything_else() {
        let mut document = IniDocument::parse(SAMPLE);
        document.set("PHP", "memory_limit", "512M");
        let expected = SAMPLE.replace("memory_limit = 128M ; old value", "memory_limit = 512M");
        assert_eq!(document.render(), expected);
    }

    #[test]
    fn set_inserts_new_key_at_end_of_section() {
        let mut document = IniDocument::parse(SAMPLE);
        document.set("PHP", "max_execution_time", "60");
        let expected = SAMPLE.replace(
            "display_errors = Off\n",
            "display_errors = Off\nmax_execution_time = 60\n",
        );
        assert_eq!(document.render(), expected);
    }

    #[test]
    fn set_creates_missing_section() {
        let mut document = IniDocument::parse("[Date]\ndate.timezone = UTC\n");
        document.set("PHP", "memory_limit", "1G");
        assert_eq!(
            document.render(),
            "[Date]\ndate.timezone = UTC\n\n[PHP]\nmemory_limit = 1G\n"
        );
    }

    #[test]
    fn remove_deletes_all_repeated_definitions() {
        let mut document = IniDocument::parse("[PHP]\nextension=a\nx=1\nextension=a\n");
        assert_eq!(document.remove("PHP", "extension"), 2);
        assert_eq!(document.render(), "[PHP]\nx=1\n");
    }

    #[test]
    fn crlf_line_endings_are_preserved() {
        let mut document = IniDocument::parse("[PHP]\r\na = 1\r\n");
        document.set("PHP", "b", "2");
        assert_eq!(document.render(), "[PHP]\r\na = 1\r\nb = 2\r\n");
    }
}
