use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::{interaction, paths, shim};
use crate::user_path as registry_path;


const BLOCK_START: &str = "# >>> phpz >>>";
const BLOCK_END: &str = "# <<< phpz <<<";
const BLOCK_BODY: &str = "export PATH=\"$HOME/.phpz/shims:$PATH\"";

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

/// Where phpz puts itself on PATH.
pub enum SetupTarget {
    /// A line block in the profile file of a Unix shell.
    Profile(Shell),
    /// The user PATH stored in the Windows registry.
    WindowsUserPath,
}

impl SetupTarget {
    pub fn detect() -> Option<SetupTarget> {
        if cfg!(windows) {
            return Some(SetupTarget::WindowsUserPath);
        }
        Shell::detect().map(SetupTarget::Profile)
    }

    /// The directory that has to be on PATH (Windows only): the one with the `php` shim.
    fn windows_entry() -> Result<String, String> {
        Ok(paths::shims_directory()?.to_string_lossy().into_owned())
    }

    /// The entry phpz 0.2 added to the user PATH, which `--undo` also takes out.
    fn legacy_windows_entry() -> Result<String, String> {
        Ok(paths::active_bin_directory()?.to_string_lossy().into_owned())
    }

    /// Short name used in messages, e.g. "~/.bashrc".
    pub fn description(&self) -> String {
        match self {
            SetupTarget::Profile(shell) => shell
                .profile_path()
                .map(|path| tilde_path(&path))
                .unwrap_or_else(|_| "your shell profile".to_string()),
            SetupTarget::WindowsUserPath => "your user PATH".to_string(),
        }
    }

    pub fn is_configured(&self) -> bool {
        match self {
            SetupTarget::Profile(shell) => shell
                .profile_path()
                .and_then(|path| fs::read_to_string(path).map_err(|error| error.to_string()))
                .map(|content| block_is_current(&content))
                .unwrap_or(false),
            SetupTarget::WindowsUserPath => Self::windows_entry()
                .map(|entry| registry_path::is_configured(&entry))
                .unwrap_or(false),
        }
    }

    /// Put the shims on PATH, or bring an older setup up to date. Returns false when nothing changed.
    pub fn configure(&self) -> Result<bool, String> {
        shim::ensure()?;
        match self {
            SetupTarget::Profile(shell) => {
                let profile = shell.profile_path()?;
                let content = read_profile(&profile)?;
                let updated = if content.contains(BLOCK_START) {
                    refresh_block(&content)?
                } else {
                    add_block(&content)
                };
                match updated {
                    Some(updated) => write_profile(&profile, updated).map(|_| true),
                    None => Ok(false),
                }
            }
            SetupTarget::WindowsUserPath => registry_path::configure(&Self::windows_entry()?),
        }
    }

    /// Remove phpz from PATH. Returns false when it was not there.
    pub fn unconfigure(&self) -> Result<bool, String> {
        match self {
            SetupTarget::Profile(shell) => {
                let profile = shell.profile_path()?;
                match remove_block(&read_profile(&profile)?)? {
                    Some(updated) => write_profile(&profile, updated).map(|_| true),
                    None => Ok(false),
                }
            }
            SetupTarget::WindowsUserPath => {
                let shims = registry_path::unconfigure(&Self::windows_entry()?)?;
                let legacy = registry_path::unconfigure(&Self::legacy_windows_entry()?)?;
                Ok(shims || legacy)
            }
        }
    }

    /// How to make a freshly added change take effect in the current terminal.
    pub fn apply_hint(&self) -> String {
        match self {
            SetupTarget::Profile(_) => {
                format!("Open a new terminal or run: source {}", self.description())
            }
            SetupTarget::WindowsUserPath => match Self::windows_entry() {
                Ok(entry) => format!(
                    "Open a new terminal or run: $env:Path = \"{};\" + $env:Path",
                    entry
                ),
                Err(_) => "Open a new terminal to apply it.".to_string(),
            },
        }
    }
}

/// Path with the home directory written as "~".
fn tilde_path_with_home(path: &Path, home: Option<&Path>) -> String {
    match home.and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

fn tilde_path(path: &Path) -> String {
    tilde_path_with_home(path, dirs::home_dir().as_deref())
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

/// Content with the body of an existing phpz block replaced by the current one, so a setup made
/// by an older phpz is brought up to date in place. Ok(None) when it already is, or when there is
/// no block; an error when the block is not closed.
pub fn refresh_block(content: &str) -> Result<Option<String>, String> {
    let lines: Vec<&str> = content.split_inclusive('\n').collect();
    let Some(start) = lines.iter().position(|line| line.trim() == BLOCK_START) else {
        return Ok(None);
    };
    let end = lines[start..]
        .iter()
        .position(|line| line.trim() == BLOCK_END)
        .map(|offset| start + offset)
        .ok_or("found the start of the phpz block but not its end; fix it by hand")?;

    let body: Vec<&str> = lines[start + 1..end].iter().map(|line| line.trim()).collect();
    if body == [BLOCK_BODY] {
        return Ok(None);
    }

    let line_ending = if lines[start].ends_with("\r\n") { "\r\n" } else { "\n" };
    let new_body = format!("{}{}", BLOCK_BODY, line_ending);
    Ok(Some(
        [&lines[..=start], &[new_body.as_str()], &lines[end..]].concat().concat(),
    ))
}

/// Whether the profile has a block, and it is the one this phpz writes.
fn block_is_current(content: &str) -> bool {
    content.contains(BLOCK_START) && matches!(refresh_block(content), Ok(None))
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

/// What this process has on PATH from phpz.
#[derive(Debug, PartialEq)]
enum PathState {
    /// The shims directory: every folder gets the version that applies to it.
    Shims,
    /// Only what phpz 0.2 added: the default version works, project versions do not.
    Legacy,
    Missing,
}

fn path_state() -> PathState {
    let (Ok(shims), Ok(legacy)) = (paths::shims_directory(), paths::active_bin_directory()) else {
        return PathState::Missing;
    };
    let Some(path) = std::env::var_os("PATH") else {
        return PathState::Missing;
    };
    let entries: Vec<PathBuf> = std::env::split_paths(&path).collect();
    if entries.contains(&shims) {
        PathState::Shims
    } else if entries.contains(&legacy) {
        PathState::Legacy
    } else {
        PathState::Missing
    }
}

/// Whether phpz is on PATH in this process in any form.
pub fn path_is_active() -> bool {
    path_state() != PathState::Missing
}

/// The note `phpz use` prints when the new version is not reachable from this terminal yet.
pub fn path_note() -> Option<String> {
    match path_state() {
        PathState::Shims => return None,
        PathState::Legacy => {
            return Some(
                "Note: run 'phpz setup' to update your PATH, so that the version can follow each project."
                    .to_string(),
            )
        }
        PathState::Missing => {}
    }
    Some(match SetupTarget::detect() {
        Some(target) if target.is_configured() => format!("Note: {}", target.apply_hint()),
        _ => "Note: run 'phpz setup' to add phpz to your PATH.".to_string(),
    })
}

/// After the first install, offer to put phpz on PATH.
/// Stays silent in scripts and CI (no terminal), when PATH is already set up,
/// and after the question was asked once.
pub fn offer_setup() {
    if path_is_active() || !interaction::is_interactive() {
        return;
    }
    let Some(target) = SetupTarget::detect() else { return };
    let Ok(home) = paths::phpz_home() else { return };
    let marker = home.join(PROMPTED_MARKER);

    if target.is_configured() || marker.exists() {
        return;
    }
    let _ = fs::write(&marker, "");

    let question = format!("Add phpz to your PATH by editing {}?", target.description());
    match interaction::confirm(&question, true) {
        Some(true) => {}
        Some(false) => {
            println!("Skipped. Run 'phpz setup' whenever you want to do it.");
            return;
        }
        None => return,
    }

    match target.configure() {
        Ok(_) => println!("Updated {}. {}", target.description(), target.apply_hint()),
        Err(error) => eprintln!("Error: {}", error),
    }
}

#[cfg(test)]
mod tests;
