use std::io::{self, IsTerminal, Write};

/// Whether a person can answer a question: input and the place questions go to are terminals.
/// Scripts and CI are not.
pub fn is_interactive() -> bool {
    io::stdin().is_terminal() && io::stderr().is_terminal()
}

/// Ask a yes/no question on the terminal; an empty answer picks `default_yes`.
/// None when no answer could be read (closed input), which callers treat as "do nothing".
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

fn interpret_answer(answer: &str, default_yes: bool) -> bool {
    match answer.trim().to_lowercase().as_str() {
        "" => default_yes,
        "y" | "yes" => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests;
