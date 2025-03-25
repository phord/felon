use crossterm::terminal::ClearType;
use std::io::{stdout, Write};
use crate::config::Config;
use crossterm::{QueueableCommand, cursor, terminal};
use crate::styled_text::styled_line::RGB_BLACK;
use crate::input_line::InputLine;

pub enum InputAction {
    Waiting,
    Done,
    Cancel,
}

pub trait Prompt {
    fn run(&mut self, timeout: u64) -> std::io::Result<InputAction>;
    fn start(&mut self, prompt: &str) -> std::io::Result<()>;
    fn end(&mut self) -> std::io::Result<()>;

    fn get_height(&self) -> u16 {
        1
    }

}

pub struct SwitchPrompt {
    prompt: String,
    mode: SwitchPromptMode,
}

/*
        -       Followed  by  one  of the command line option letters (see OPTIONS below), this will change the setting of that
                option and print a message describing the new setting.  If a ^P (CONTROL-P) is entered immediately after the dash,
                the setting of the option is changed but no message is printed.  If the option letter has a numeric value (such
                as -b or -h), or a string value (such as -P or -t), a new value may be entered after the option letter.  If no new
                value is entered, a message describing the current setting is printed and nothing is changed.

        --     Like the - command, but takes a long option name (see OPTIONS below) rather than a single option letter.  You must
                press ENTER or RETURN after typing the option name.  A ^P immediately after the second dash suppresses printing of
                a message describing the new setting, as in the - command.

        -+     Followed by one of the command line option letters this will reset the option to its default setting and print
                a message describing the new setting.  (The "-+X" command does the same thing as "-+X" on the command line.)
                This does not work for string-valued options.

        --+    Like the -+ command, but takes a long option name rather than a single option letter.

        -!     Followed by one of the command line option letters, this will reset the option to the "opposite" of its default
                setting and print a message describing the new setting.  This does not work for numeric or string-valued options.

        --!    Like the -! command, but takes a long option name rather than a single option letter.

        _      (Underscore.)  Followed by one of the command line option letters, this will print a message describing the current
                setting of that option.  The setting of the option is not changed.

        __     (Double underscore.)  Like the _ (underscore) command, but takes a long option name rather than a single option
                letter.  You must press ENTER or RETURN after typing the option name.
 */

 enum SwitchPromptMode {
    Init,           // -
    InitLong,       // --
    Reset,          // -+
    ResetOpposite,  // -!
    Describe,       // _
    DescribeLong,   // __
    Finished,       // ...x
}


impl SwitchPrompt {
    pub fn new(config: &Config, prompt: &str) -> Self {
        Self {
            prompt: prompt.to_string(),
            mode: SwitchPromptMode::Init,
        }
    }
}

impl Prompt for SwitchPrompt {
    fn start(&mut self, prompt: &str) -> std::io::Result<()> {
        let (_width, height) = terminal::size().expect("Unable to get terminal size");
        let mut stdout = stdout();
        stdout.queue(cursor::MoveTo(0, height - self.get_height()))?;
        // if color?: stdout.queue(crossterm::style::SetBackgroundColor(RGB_BLACK))?;
        stdout.queue(terminal::Clear(ClearType::UntilNewLine))?;
        stdout.queue(style::PrintStyledContent("-".to_string())).expect("Terminal exists");
        stdout().queue(cursor::Show)?;
        stdout.flush()
    }

    fn end(&mut self) -> std::io::Result<()> {
        let mut stdout = stdout();
        stdout().queue(cursor::Hide)?;
        stdout.flush()
    }

    fn run(&mut self, timeout: u64) -> std::io::Result<InputAction> {
        match self.mode {
            SwitchPromptMode::Init => {
            }
        }
    }
}
