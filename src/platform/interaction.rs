use std::io::{self, IsTerminal, Write};

/// Whether a person can answer a question (input and output are terminals).
pub fn is_interactive() -> bool {
    io::stdin().is_terminal() && io::stderr().is_terminal()
}

/// Ask a yes/no question; None when no answer could be read.
pub fn confirm(question: &str, default_yes: bool) -> Option<bool> {
    let hint = if default_yes { "[Y/n]" } else { "[y/N]" };
    eprint!("{} {} ", question, hint);
    let _ = io::stderr().flush();

    let mut answer = String::new();
    if io::stdin().read_line(&mut answer).ok()? == 0 {
        return None;
    }
    Some(interpret_answer(&answer, default_yes))
}

pub fn interpret_answer(answer: &str, default_yes: bool) -> bool {
    match answer.trim().to_lowercase().as_str() {
        "" => default_yes,
        "y" | "yes" => true,
        _ => false,
    }
}
