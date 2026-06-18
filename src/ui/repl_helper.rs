use std::borrow::Cow::{self, Borrowed};

use rustyline::highlight::Highlighter;
use rustyline::{Completer, Helper, Hinter, Validator};

const INPUT_PROMPT_PLAIN: &str = "You ";

pub fn input_prompt_plain() -> &'static str {
    INPUT_PROMPT_PLAIN
}

pub fn colored_input_prompt() -> String {
    format!("{} ", super::user_prefix())
}

#[derive(Completer, Helper, Hinter, Validator)]
pub struct ReplHelper {
    pub colored_prompt: String,
}

impl ReplHelper {
    pub fn new() -> Self {
        Self {
            colored_prompt: colored_input_prompt(),
        }
    }
}

impl Default for ReplHelper {
    fn default() -> Self {
        Self::new()
    }
}

impl Highlighter for ReplHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            Borrowed(&self.colored_prompt)
        } else {
            Borrowed(prompt)
        }
    }
}
