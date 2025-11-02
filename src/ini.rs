use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

type Section = HashMap<String, String>;
type Sections = HashMap<String, Section>;

/// Represents an INI configuration file with sections and key-value pairs
///
/// # Example
/// ```
/// let mut ini = IniFile::new();
/// ini.set("PHP", "memory_limit", "256M");
/// let value = ini.get("PHP", "memory_limit");
/// ```
#[derive(Debug, Clone)]
pub struct IniFile {
    sections: Sections,
}

impl IniFile {
    /// Creates a new empty INI file
    pub fn new() -> Self {
        Self {
            sections: HashMap::new(),
        }
    }

    /// Retrieves a value from the specified section and key
    pub fn get(&self, section: &str, key: &str) -> Option<&String> {
        self.sections.get(section)?.get(key)
    }

    /// Sets a value for the specified section and key
    pub fn set(&mut self, section: &str, key: &str, value: &str) {
        self.sections
            .entry(section.to_string())
            .or_insert_with(HashMap::new)
            .insert(key.to_string(), value.to_string());
    }

    /// Removes a key from the specified section
    pub fn remove(&mut self, section: &str, key: &str) -> bool {
        self.sections
            .get_mut(section)
            .and_then(|s| s.remove(key))
            .is_some()
    }

    /// Parses INI content from a string
    pub fn parse(content: &str) -> Self {
        let mut ini = Self::new();
        let mut current_section = String::new();

        for line in content.lines() {
            match parse_line(line) {
                Line::Section(name) => current_section = name,
                Line::KeyValue(key, value) => ini.set(&current_section, &key, &value),
                Line::Empty | Line::Comment => continue,
            }
        }

        ini
    }

    /// Converts the INI file to string format
    pub fn to_string(&self) -> String {
        let mut output = String::new();

        for (section_name, section_data) in self.sorted_sections() {
            output.push_str(&format_section(section_name, section_data));
        }

        output
    }

    /// Loads an INI file from disk
    pub fn load_from_file(path: &Path) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(Self::parse(&content))
    }

    /// Saves the INI file to disk
    pub fn save_to_file(&self, path: &Path) -> io::Result<()> {
        fs::write(path, self.to_string())
    }

    fn sorted_sections(&self) -> Vec<(&String, &Section)> {
        let mut sections: Vec<_> = self.sections.iter().collect();
        sections.sort_by_key(|(name, _)| *name);
        sections
    }
}

impl Default for IniFile {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents different types of lines in an INI file
#[derive(Debug, PartialEq)]
enum Line {
    Section(String),
    KeyValue(String, String),
    Comment,
    Empty,
}

/// Parses a single line from an INI file
fn parse_line(line: &str) -> Line {
    let trimmed = line.trim();

    if trimmed.is_empty() {
        return Line::Empty;
    }

    if is_comment(trimmed) {
        return Line::Comment;
    }

    if let Some(section) = parse_section_header(trimmed) {
        return Line::Section(section);
    }

    if let Some((key, value)) = parse_key_value(trimmed) {
        return Line::KeyValue(key, value);
    }

    Line::Empty
}

/// Checks if a line is a comment
fn is_comment(line: &str) -> bool {
    line.starts_with(';') || line.starts_with('#')
}

/// Parses a section header like [section_name]
fn parse_section_header(line: &str) -> Option<String> {
    if line.starts_with('[') && line.ends_with(']') {
        Some(line[1..line.len() - 1].trim().to_string())
    } else {
        None
    }
}

/// Parses a key=value pair
fn parse_key_value(line: &str) -> Option<(String, String)> {
    let eq_pos = line.find('=')?;
    let key = line[..eq_pos].trim().to_string();
    let value = extract_value(&line[eq_pos + 1..]);

    Some((key, value))
}

/// Extracts and cleans the value from a key=value pair
fn extract_value(raw_value: &str) -> String {
    let trimmed = raw_value.trim();
    let unquoted = remove_quotes(trimmed);
    remove_inline_comment(unquoted)
}

/// Removes surrounding quotes from a value
fn remove_quotes(value: &str) -> String {
    if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

/// Removes inline comments from a value
fn remove_inline_comment(value: String) -> String {
    if let Some(comment_pos) = value.find(';') {
        value[..comment_pos].trim().to_string()
    } else {
        value
    }
}

/// Formats a section and its key-value pairs for output
fn format_section(name: &str, data: &Section) -> String {
    let mut output = String::new();

    if !name.is_empty() {
        output.push_str(&format!("[{}]\n", name));
    }

    for (key, value) in sorted_pairs(data) {
        output.push_str(&format_key_value(key, value));
    }

    output.push('\n');
    output
}

/// Returns sorted key-value pairs from a section
fn sorted_pairs(section: &Section) -> Vec<(&String, &String)> {
    let mut pairs: Vec<_> = section.iter().collect();
    pairs.sort_by_key(|(key, _)| *key);
    pairs
}

/// Formats a key-value pair, adding quotes if needed
fn format_key_value(key: &str, value: &str) -> String {
    if needs_quotes(value) {
        format!("{}=\"{}\"\n", key, value)
    } else {
        format!("{}={}\n", key, value)
    }
}

/// Determines if a value needs to be quoted
fn needs_quotes(value: &str) -> bool {
    value.contains(' ')
        || value.contains(';')
        || value.contains('#')
        || value.contains('=')
}
