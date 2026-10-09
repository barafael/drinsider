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
    (Modifiers::NONE, Key::Q, Command::Exit),
    (Modifiers::NONE, Key::Escape, Command::Exit),
];

/// Consumes this frame's key presses and returns the matching commands.
///
/// While a prompt is open only prompt commands are produced: typed text,
/// Backspace, Enter (confirm) and Escape (cancel), as in binsider.
pub fn poll(ctx: &egui::Context, prompt_open: bool) -> Vec<Command> {
    ctx.input_mut(|input| {
        let mut commands = Vec::new();
        // Several presses of one key can arrive in a single frame (fast typing,
        // key repeat); count them all instead of consuming just one.
        let mut press = |modifiers, key, command: Command| {
            for _ in 0..input.count_and_consume_key(modifiers, key) {
                commands.push(command.clone());
            }
        };
        if prompt_open {
            press(
                Modifiers::NONE,
                Key::Escape,
                Command::Input(InputCommand::Exit),
            );
            press(
                Modifiers::NONE,
                Key::Enter,
                Command::Input(InputCommand::Confirm),
            );
            press(
                Modifiers::NONE,
                Key::Backspace,
                Command::Input(InputCommand::Backspace),
            );
            input.events.retain(|event| match event {
                Event::Text(text) => {
                    commands.push(Command::Input(InputCommand::Handle(text.clone())));
                    false
                }
                _ => true,
            });
        } else {
            #[cfg(target_arch = "wasm32")]
            press(Modifiers::COMMAND, Key::O, Command::OpenFile);
            for (modifiers, key, command) in BINDINGS {
                press(*modifiers, *key, command.clone());
            }
        }
        // Ctrl+C never arrives as a key press: egui-winit turns it into a `Copy`
        // event and drops the key event, so this is the only place it is handled.
        let before = input.events.len();
        input.events.retain(|event| !matches!(event, Event::Copy));
        if input.events.len() != before {
            commands.push(Command::Exit);
        }
        commands
    })
}
