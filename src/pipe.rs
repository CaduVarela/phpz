/// Exit quietly when the reader of our output goes away (`phpz list --remote | head -1`),
/// instead of printing a panic message.
pub fn exit_quietly_on_closed_pipe() {
    restore_default_sigpipe();
    install_panic_hook();
}

/// Rust ignores SIGPIPE, which turns a closed pipe into a failed `println!`.
/// Restoring the default makes the process end silently, like other command line tools.
#[cfg(unix)]
fn restore_default_sigpipe() {
    // SAFETY: setting a signal disposition to its default has no memory-safety requirements.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

/// Any `println!` that fails because the pipe closed ends the process quietly.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("");

        if is_failed_print(message) {
            std::process::exit(1);
        }
        default_hook(info);
    }));
}

/// `println!` failures, and writers (such as clap_complete's) that panic with a BrokenPipe error.
fn is_failed_print(message: &str) -> bool {
    message.starts_with("failed printing to std") || message.contains("BrokenPipe")
}

#[cfg(test)]
mod tests;
