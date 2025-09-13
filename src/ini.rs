pub struct IniFile {
    sections: HashMap<String, HashMap<String, String>>, // [section] -> (key -> value)
}

impl IniFile {
    pub fn get(&self, section: &str, key: &str) -> Option<&String>;
    pub fn set(&mut self, section: &str, key: &str, value: &str);
    pub fn remove(&mut self, section: &str, key: &str);
    pub fn parse(content: &str) -> Self { ... } // string to IniFile
    pub fn to_string(&self) -> String; // IniFile to string
}
