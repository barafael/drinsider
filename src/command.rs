//! Keyboard commands.
//!
//! Ported from binsider (https://github.com/orhun/binsider),
//! Copyright Orhun Parmaksız, MIT OR Apache-2.0.

use egui::{Event, Key, Modifiers};

/// Possible scroll areas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollType {
    /// Main application tabs.
    Tab,
    /// Inner tables / horizontal movement.
    Table,
    /// Main list.
    List,
    /// Focused block.
    Block,
}

/// Application command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Open the documentation URL.
    OpenRepo,
    /// Show details / activate the selected item.
    ShowDetails,
    /// Move forward.
    Next(ScrollType, usize),
    /// Move backward.
    Previous(ScrollType, usize),
    /// Go to top.
    Top,
    /// Go to bottom.
    Bottom,
    /// Increment value.
    Increment,
    /// Decrement value.
    Decrement,
    /// Search / jump input command.
    Input(InputCommand),
    /// Trace system calls.
    TraceCalls,
    /// Toggle human readable format.
    HumanReadable,
    /// Jump to an offset (hexdump).
    Goto,
    /// Copy the selected value to the clipboard.
    Copy,
    /// Start or end selecting a range (hexdump visual mode).
    Visual,
    /// Pick a file to analyze (browser only).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    OpenFile,
    /// Exit the application.
    Exit,
}

/// Input prompt command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputCommand {
    /// Open the search prompt.
    Enter,
    /// Confirm the prompt.
    Confirm,
    /// Leave the prompt and clear it.
    Exit,
    /// Delete a character; outside the prompt: go back / reopen the query.
    Backspace,
    /// Text typed into the prompt.
    Handle(String),
}

/// Normal-mode key bindings, checked in order.
///
/// `Modifiers::NONE` ignores shift when matching, so shifted bindings come
/// first; `COMMAND` (Ctrl on Linux) bindings come before the bare letters.
const BINDINGS: &[(Modifiers, Key, Command)] = &[
    (
        Modifiers::SHIFT,
        Key::Tab,
        Command::Previous(ScrollType::Tab, 1),
    ),
    (Modifiers::NONE, Key::Tab, Command::Next(ScrollType::Tab, 1)),
    (
        Modifiers::COMMAND,
        Key::D,
        Command::Next(ScrollType::List, 5),
    ),
    (
        Modifiers::COMMAND,
        Key::U,
        Command::Previous(ScrollType::List, 5),
    ),
    (
        Modifiers::COMMAND,
        Key::F,
        Command::Input(InputCommand::Enter),
    ),
    (
        Modifiers::NONE,
        Key::ArrowRight,
        Command::Next(ScrollType::Table, 1),
    ),
    (Modifiers::NONE, Key::L, Command::Next(ScrollType::Table, 1)),
    (
        Modifiers::NONE,
        Key::ArrowLeft,
        Command::Previous(ScrollType::Table, 1),
    ),
    (
        Modifiers::NONE,
        Key::H,
        Command::Previous(ScrollType::Table, 1),
    ),
    (
        Modifiers::NONE,
        Key::ArrowDown,
        Command::Next(ScrollType::List, 1),
    ),
    (Modifiers::NONE, Key::J, Command::Next(ScrollType::List, 1)),
    (
        Modifiers::NONE,
        Key::ArrowUp,
        Command::Previous(ScrollType::List, 1),
    ),
    (
        Modifiers::NONE,
        Key::K,
        Command::Previous(ScrollType::List, 1),
    ),
    (
        Modifiers::NONE,
        Key::PageDown,
        Command::Next(ScrollType::List, 5),
    ),
    (
        Modifiers::NONE,
        Key::PageUp,
        Command::Previous(ScrollType::List, 5),
    ),
    (Modifiers::NONE, Key::N, Command::Next(ScrollType::Block, 1)),
    (
        Modifiers::NONE,
        Key::P,
        Command::Previous(ScrollType::Block, 1),
    ),
    (Modifiers::NONE, Key::T, Command::Top),
    (Modifiers::NONE, Key::Home, Command::Top),
    (Modifiers::NONE, Key::B, Command::Bottom),
    (Modifiers::NONE, Key::End, Command::Bottom),
    (Modifiers::NONE, Key::Plus, Command::Increment),
    (Modifiers::NONE, Key::Minus, Command::Decrement),
    (
        Modifiers::NONE,
        Key::Slash,
        Command::Input(InputCommand::Enter),
    ),
    (
        Modifiers::NONE,
        Key::Backspace,
        Command::Input(InputCommand::Backspace),
    ),
    (Modifiers::NONE, Key::Enter, Command::ShowDetails),
    (Modifiers::NONE, Key::O, Command::OpenRepo),
    (Modifiers::NONE, Key::R, Command::TraceCalls),
    (Modifiers::NONE, Key::S, Command::HumanReadable),
    (Modifiers::NONE, Key::G, Command::Goto),
    (Modifiers::NONE, Key::Y, Command::Copy),
    (Modifiers::NONE, Key::V, Command::Visual),
    (Modifiers::NONE, Key::Q, Command::Exit),
    (Modifiers::NONE, Key::Escape, Command::Exit),
];

/// This frame's keyboard events, read in the order they were typed.
///
/// Each command must run before the next event is read, because the prompt
/// it may open or close decides what the following keys mean: in binsider,
/// `/` followed by `p` searches for "p" instead of switching blocks.
pub struct Keys {
    events: std::iter::Peekable<std::vec::IntoIter<Event>>,
    /// The previous event was this key, and it ran a command. On the desktop
    /// a key's text follows it (the browser sends it first), and the `/`
    /// that opened a prompt must not be typed into it.
    command_key: Option<Key>,
}

impl Keys {
    /// Takes the key presses, text and copy events out of this frame's input.
    pub fn take(ctx: &egui::Context) -> Self {
        let events = ctx.input_mut(|input| {
            let (keys, rest) = std::mem::take(&mut input.events)
                .into_iter()
                .partition(|event| {
                    matches!(
                        event,
                        Event::Key { pressed: true, .. } | Event::Text(_) | Event::Copy
                    )
                });
            input.events = rest;
            keys
        });
        Self {
            events: Vec::into_iter(events).peekable(),
            command_key: None,
        }
    }

    /// The next command, given whether a prompt is open now.
    ///
    /// While a prompt is open only prompt commands are produced: typed text,
    /// Backspace, Enter (confirm) and Escape (cancel), as in binsider.
    pub fn next(&mut self, prompt_open: bool) -> Option<Command> {
        while let Some(event) = self.events.next() {
            let command_key = self.command_key.take();
            let own_text = |text: &str| {
                command_key.is_some_and(|k| k.symbol_or_name().eq_ignore_ascii_case(text))
            };
            let command = match event {
                // Ctrl+C never arrives as a key press: egui-winit turns it into a `Copy`
                // event and drops the key event.
                Event::Copy => Some(Command::Exit),
                Event::Text(mut text) if prompt_open && !own_text(&text) => {
                    // Text typed within one frame is one command, so a long list is
                    // filtered once per frame instead of once per character.
                    let typing = |e: &Event| match e {
                        Event::Text(_) => true,
                        Event::Key { key, .. } => {
                            !matches!(key, Key::Escape | Key::Enter | Key::Backspace)
                        }
                        _ => false,
                    };
                    while let Some(next) = self.events.next_if(typing) {
                        if let Event::Text(more) = next {
                            text.push_str(&more);
                        }
                    }
                    Some(Command::Input(InputCommand::Handle(text)))
                }
                Event::Key { key, modifiers, .. } if prompt_open => match key {
                    Key::Escape => Some(Command::Input(InputCommand::Exit)),
                    Key::Enter => Some(Command::Input(InputCommand::Confirm)),
                    Key::Backspace => Some(Command::Input(InputCommand::Backspace)),
                    _ => None,
                }
                .filter(|_| modifiers.matches_logically(Modifiers::NONE)),
                // A held Escape must not run on from closing something into quitting.
                Event::Key {
                    key,
                    repeat: true,
                    modifiers,
                    ..
                } if binding(key, modifiers) == Some(Command::Exit) => None,
                Event::Key { key, modifiers, .. } => {
                    let command = binding(key, modifiers);
                    self.command_key =
                        (cfg!(not(target_arch = "wasm32")) && command.is_some()).then_some(key);
                    command
                }
                _ => None,
            };
            if command.is_some() {
                return command;
            }
        }
        None
    }
}

/// The commands a status bar hint such as `j/k`, `/`, `Shift+Tab` or
/// `Ctrl+O` stands for.
pub fn hinted(keys: &str) -> Vec<Command> {
    let tokens: Vec<&str> = if keys == "/" {
        vec![keys]
    } else {
        keys.split('/').collect()
    };
    tokens
        .into_iter()
        .filter_map(|token| {
            let (modifiers, key) = match token {
                "Shift+Tab" => (Modifiers::SHIFT, Key::Tab),
                "Ctrl+O" => (Modifiers::COMMAND, Key::O),
                "Bksp" => (Modifiers::NONE, Key::Backspace),
                key => (Modifiers::NONE, Key::from_name(key)?),
            };
            binding(key, modifiers)
        })
        .collect()
}

/// Whether two commands are the same action, whatever the step size:
/// `j`, Down and PageDown all move the list forward.
pub fn same_action(a: &Command, b: &Command) -> bool {
    match (a, b) {
        (Command::Next(x, _), Command::Next(y, _))
        | (Command::Previous(x, _), Command::Previous(y, _)) => x == y,
        _ => a == b,
    }
}

/// The normal-mode command bound to a key press. Earlier bindings win, so
/// Shift+Tab and Ctrl+key are matched before their plain versions.
fn binding(key: Key, modifiers: Modifiers) -> Option<Command> {
    #[cfg(target_arch = "wasm32")]
    if key == Key::O && modifiers.matches_logically(Modifiers::COMMAND) {
        return Some(Command::OpenFile);
    }
    BINDINGS
        .iter()
        .find(|(m, k, _)| *k == key && modifiers.matches_logically(*m))
        .map(|(_, _, command)| command.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(key: Key, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    /// Ctrl as egui reports it on Linux and Windows.
    const CTRL: Modifiers = Modifiers {
        ctrl: true,
        command: true,
        ..Modifiers::NONE
    };

    fn text(text: &str) -> Event {
        Event::Text(text.into())
    }

    /// Feeds one frame's events through `Keys`, opening and closing the
    /// prompt the way `State` does.
    fn run(events: Vec<Event>) -> Vec<Command> {
        let mut keys = Keys {
            events: events.into_iter().peekable(),
            command_key: None,
        };
        let (mut prompt_open, mut commands) = (false, Vec::new());
        while let Some(command) = keys.next(prompt_open) {
            match command {
                Command::Input(InputCommand::Enter) => prompt_open = true,
                Command::Input(InputCommand::Confirm | InputCommand::Exit) => prompt_open = false,
                _ => {}
            }
            commands.push(command);
        }
        commands
    }

    fn typed(text: &str) -> Command {
        Command::Input(InputCommand::Handle(text.into()))
    }

    #[test]
    fn keys_after_opening_the_prompt_are_typed() {
        // egui-winit sends a key's text right after the key.
        let slash = run(vec![
            key(Key::Slash, Modifiers::NONE),
            text("/"),
            key(Key::P, Modifiers::NONE),
            text("p"),
        ]);
        assert_eq!(slash, [Command::Input(InputCommand::Enter), typed("p")]);
        let ctrl_f = run(vec![
            key(Key::F, CTRL),
            key(Key::P, Modifiers::NONE),
            text("p"),
        ]);
        assert_eq!(ctrl_f, [Command::Input(InputCommand::Enter), typed("p")]);
    }

    #[test]
    fn text_typed_in_one_frame_is_one_command() {
        let commands = run(vec![
            key(Key::F, CTRL),
            key(Key::A, Modifiers::NONE),
            text("a"),
            key(Key::B, Modifiers::NONE),
            text("b"),
            key(Key::Backspace, Modifiers::NONE),
            text("c"),
        ]);
        assert_eq!(
            commands,
            [
                Command::Input(InputCommand::Enter),
                typed("ab"),
                Command::Input(InputCommand::Backspace),
                typed("c")
            ]
        );
    }

    #[test]
    fn hints_name_their_commands() {
        assert_eq!(
            hinted("j/k"),
            [
                Command::Next(ScrollType::List, 1),
                Command::Previous(ScrollType::List, 1)
            ]
        );
        assert_eq!(hinted("/"), [Command::Input(InputCommand::Enter)]);
        assert_eq!(hinted("Shift+Tab"), [Command::Previous(ScrollType::Tab, 1)]);
        assert_eq!(hinted("Bksp"), [Command::Input(InputCommand::Backspace)]);
        assert_eq!(hinted("y"), [Command::Copy]);
        assert!(same_action(
            &Command::Next(ScrollType::List, 5),
            &Command::Next(ScrollType::List, 1)
        ));
        assert!(!same_action(
            &Command::Next(ScrollType::List, 1),
            &Command::Next(ScrollType::Table, 1)
        ));
    }

    #[test]
    fn held_escape_does_not_repeat() {
        let repeat = Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: true,
            modifiers: Modifiers::NONE,
        };
        let held_j = Event::Key {
            key: Key::J,
            physical_key: None,
            pressed: true,
            repeat: true,
            modifiers: Modifiers::NONE,
        };
        let commands = run(vec![key(Key::Escape, Modifiers::NONE), repeat, held_j]);
        assert_eq!(
            commands,
            [Command::Exit, Command::Next(ScrollType::List, 1)]
        );
    }

    #[test]
    fn keys_run_in_typing_order() {
        let commands = run(vec![
            key(Key::Tab, Modifiers::NONE),
            key(Key::J, Modifiers::NONE),
            key(Key::Tab, Modifiers::SHIFT),
            key(Key::Enter, Modifiers::NONE),
        ]);
        assert_eq!(
            commands,
            [
                Command::Next(ScrollType::Tab, 1),
                Command::Next(ScrollType::List, 1),
                Command::Previous(ScrollType::Tab, 1),
                Command::ShowDetails,
            ]
        );
    }

    #[test]
    fn prompt_keys_and_copy() {
        let commands = run(vec![
            key(Key::F, CTRL),
            text("a"),
            key(Key::Backspace, Modifiers::NONE),
            key(Key::Enter, Modifiers::NONE),
            key(Key::J, Modifiers::NONE),
            text("j"),
            Event::Copy,
        ]);
        assert_eq!(
            commands,
            [
                Command::Input(InputCommand::Enter),
                typed("a"),
                Command::Input(InputCommand::Backspace),
                Command::Input(InputCommand::Confirm),
                Command::Next(ScrollType::List, 1),
                Command::Exit,
            ]
        );
    }
}
