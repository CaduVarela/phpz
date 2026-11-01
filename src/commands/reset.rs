use crate::platform::link;
use crate::platform::paths;

/// Deactivate phpz's PHP: remove the `current` link.
pub fn run() -> Result<(), String> {
    let current = paths::current_link()?;
    if link::remove_link(&current)? {
        println!("Deactivated the active PHP version.");
    } else {
        println!("No PHP version is active.");
    }
    Ok(())
}
