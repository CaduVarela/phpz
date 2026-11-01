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

        self.insert_into_section(section, new_line);
    }

    /// Effective values of every active `key` line in `section`, in file order.
    pub fn values(&self, section: &str, key: &str) -> Vec<String> {
        self.find_entries(section, key)
            .into_iter()
            .filter_map(|index| match classify(&self.lines[index]) {
                LineKind::Entry { raw_value, .. } => Some(effective_value(raw_value)),
                _ => None,
            })
            .collect()
    }

    /// Activate the first commented `;key=value` line of `section` whose value satisfies `matches`.
    /// The semicolon must be directly followed by the key, so prose comments are never activated.
    pub fn uncomment_entry(&mut self, section: &str, key: &str, matches: impl Fn(&str) -> bool) -> bool {
        let mut current_section = String::new();

        for index in 0..self.lines.len() {
            if let LineKind::Section(name) = classify(&self.lines[index]) {
                current_section = name.to_string();
                continue;
            }
            if current_section != section {
                continue;
            }
            let Some(rest) = self.lines[index].strip_prefix(';').filter(|rest| rest.starts_with(key)) else {
                continue;
            };
            if let LineKind::Entry { key: line_key, raw_value } = classify(rest) {
                if line_key == key && matches(&effective_value(raw_value)) {
                    self.lines[index] = rest.to_string();
                    return true;
                }
            }
        }
        false
    }

    /// Comment out every active `key` line of `section` whose value satisfies `matches`.
    /// Returns how many lines changed.
    pub fn comment_out_entries(&mut self, section: &str, key: &str, matches: impl Fn(&str) -> bool) -> usize {
        let mut changed = 0;
        for index in self.find_entries(section, key) {
            let value = match classify(&self.lines[index]) {
                LineKind::Entry { raw_value, .. } => effective_value(raw_value),
                _ => continue,
            };
            if matches(&value) {
                self.lines[index] = format!(";{}", self.lines[index].trim_start());
                changed += 1;
            }
        }
        changed
    }

    /// Add `key=value` next to the other `key` lines of `section` (active or commented out),
    /// or at the end of the section when there are none.
    pub fn add_entry_near(&mut self, section: &str, key: &str, value: &str) {
        let new_line = format!("{}={}", key, value);
        let mut current_section = String::new();
        let mut anchor = None;

        for (index, line) in self.lines.iter().enumerate() {
            if let LineKind::Section(name) = classify(line) {
                current_section = name.to_string();
                continue;
            }
            if current_section != section {
                continue;
            }
            let entry_line = line.strip_prefix(';').filter(|rest| rest.starts_with(key)).unwrap_or(line);
            if let LineKind::Entry { key: line_key, .. } = classify(entry_line) {
                if line_key == key {
                    anchor = Some(index + 1);
                }
            }
        }

        match anchor {
            Some(index) => self.lines.insert(index, new_line),
            None => self.insert_into_section(section, new_line),
        }
    }

    /// Put `new_line` at the end of `section`, creating the section when it does not exist.
    fn insert_into_section(&mut self, section: &str, new_line: String) {
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
mod tests;
