use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use crate::paths;

const BLOCK_START: &str = "# >>> phpz >>>";
const BLOCK_END: &str = "# <<< phpz <<<";
const BLOCK_BODY: &str = "export PATH=\"$HOME/.phpz/current/bin:$PATH\"";

/// Marker file recording that the first-install question was already asked.
const PROMPTED_MARKER: &str = ".setup-prompted";

#[derive(Debug, PartialEq)]
pub enum Shell {
    Bash,
    Zsh,
}

impl Shell {
    /// Detect the user's login shell from $SHELL.
    pub fn detect() -> Option<Shell> {
        let shell_path = std::env::var_os("SHELL")?;
        match Path::new(&shell_path).file_name()?.to_str()? {
            "bash" => Some(Shell::Bash),
            "zsh" => Some(Shell::Zsh),
            _ => None,
        }
    }

    pub fn profile_path(&self) -> Result<PathBuf, String> {
        let home = dirs::home_dir().ok_or("could not determine home directory")?;
        Ok(match self {
            Shell::Bash => home.join(".bashrc"),
            Shell::Zsh => std::env::var_os("ZDOTDIR")
                .map(PathBuf::from)
                .unwrap_or(home)
                .join(".zshrc"),
        })
    }
}

/// Content with the phpz block appended, or None when the block is already there.
pub fn add_block(content: &str) -> Option<String> {
    if content.contains(BLOCK_START) {
        return None;
    }

    let mut updated = content.to_string();
    if !updated.is_empty() {
        if !updated.ends_with('\n') {
            updated.push('\n');
        }
        updated.push('\n');
    }
    updated.push_str(&format!("{}\n{}\n{}\n", BLOCK_START, BLOCK_BODY, BLOCK_END));
    Some(updated)
}

/// Content without the phpz block (and the blank line `add_block` put before it).
/// Ok(None) when there is no block; an error when the block is not closed.
pub fn remove_block(content: &str) -> Result<Option<String>, String> {
    let lines: Vec<&str> = content.split_inclusive('\n').collect();
    let Some(start) = lines.iter().position(|line| line.trim() == BLOCK_START) else {
        return Ok(None);
    };
    let end = lines[start..]
        .iter()
        .position(|line| line.trim() == BLOCK_END)
        .map(|offset| start + offset)
        .ok_or("found the start of the phpz block but not its end; remove it by hand")?;

    let first_removed = if start > 0 && lines[start - 1].trim().is_empty() {
        start - 1
    } else {
        start
    };
    Ok(Some([&lines[..first_removed], &lines[end + 1..]].concat().concat()))
}

pub fn is_configured(shell: &Shell) -> bool {
    shell
        .profile_path()
        .and_then(|path| fs::read_to_string(path).map_err(|error| error.to_string()))
        .map(|content| content.contains(BLOCK_START))
        .unwrap_or(false)
}

fn read_profile(profile: &Path) -> Result<String, String> {
    match fs::read_to_string(profile) {
        Ok(content) => Ok(content),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(format!("reading {}: {}", profile.display(), error)),
    }
}

/// Written in place (no temporary file) so symlinked dotfiles keep working.
fn write_profile(profile: &Path, content: String) -> Result<(), String> {
    fs::write(profile, content).map_err(|error| format!("writing {}: {}", profile.display(), error))
}

/// Add the phpz block to the shell profile. Returns false when it was already there.
pub fn configure(shell: &Shell) -> Result<bool, String> {
    let profile = shell.profile_path()?;
    match add_block(&read_profile(&profile)?) {
        Some(updated) => write_profile(&profile, updated).map(|_| true),
        None => Ok(false),
    }
}

/// Remove the phpz block from the shell profile. Returns false when there was none.
pub fn unconfigure(shell: &Shell) -> Result<bool, String> {
    let profile = shell.profile_path()?;
    match remove_block(&read_profile(&profile)?)? {
        Some(updated) => write_profile(&profile, updated).map(|_| true),
        None => Ok(false),
    }
}

/// Whether the active version's bin directory is on PATH in this process.
pub fn path_is_active() -> bool {
    let Ok(link) = paths::current_link() else {
        return false;
    };
    let bin_directory = link.join("bin");
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).any(|entry| entry == bin_directory))
        .unwrap_or(false)
}

/// After the first install, offer to configure the shell profile.
/// Stays silent in scripts and CI (no terminal), when PATH is already set up,
/// and after the question was asked once.
pub fn offer_setup() {
    if path_is_active() || !(io::stdin().is_terminal() && io::stderr().is_terminal()) {
        return;
    }
    let Some(shell) = Shell::detect() else { return };
    let Ok(profile) = shell.profile_path() else { return };
    let Ok(home) = paths::phpz_home() else { return };
    let marker = home.join(PROMPTED_MARKER);

    if is_configured(&shell) || marker.exists() {
        return;
    }
    let _ = fs::write(&marker, "");

    eprint!("Add phpz to your PATH by editing {}? [Y/n] ", profile.display());
    let _ = io::stderr().flush();

    let mut answer = String::new();
    if io::stdin().read_line(&mut answer).is_err() {
        return;
    }
    if !matches!(answer.trim().to_lowercase().as_str(), "" | "y" | "yes") {
        println!("Skipped. Run 'phpz setup' whenever you want to do it.");
        return;
    }

    match configure(&shell) {
        Ok(_) => println!("Updated {}. Open a new terminal to apply it.", profile.display()),
        Err(error) => eprintln!("Error: {}", error),
    }
}

#[cfg(test)]
mod tests;
