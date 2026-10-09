//! System call tracing with lurk, like `strace(1)`.
//!
//! Ported from binsider (https://github.com/orhun/binsider),
//! Copyright Orhun Parmaksız, MIT OR Apache-2.0.

use crate::ansi;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc;

/// Formatting options passed through to lurk (binsider's trace flags).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraceOptions {
    /// Display system call numbers.
    pub syscall_number: bool,
    /// Print un-abbreviated strings.
    pub no_abbrev: bool,
    /// Maximum string argument size to print.
    pub string_limit: Option<usize>,
}

/// Output of one trace run.
#[derive(Debug, Default, Clone)]
pub struct TraceData {
    /// One colored line per system call.
    pub syscalls: Vec<ansi::Line>,
    /// lurk's summary table.
    pub summary: String,
}

/// Result of one trace run.
pub type Traced = Result<TraceData, String>;

/// Runs `command` (binary and arguments) under ptrace on a worker thread.
///
/// The result arrives on the returned channel; `ctx` is repainted when it does.
#[cfg(not(target_arch = "wasm32"))]
pub fn trace(
    command: Vec<String>,
    options: TraceOptions,
    ctx: egui::Context,
) -> mpsc::Receiver<Traced> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(native::run(&command, options));
        ctx.request_repaint();
    });
    rx
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::{TraceData, TraceOptions, Traced};
    use crate::ansi;
    use console::Style;
    use lurk_cli::Tracer;
    use lurk_cli::args::Args;
    use lurk_cli::style::StyleConfig;
    use nix::sys::wait::{WaitPidFlag, waitpid};
    use nix::unistd::{ForkResult, fork};
    use std::io::Cursor;

    pub fn run(command: &[String], options: TraceOptions) -> Traced {
        // SAFETY: the child only calls `run_tracee`, which sets up ptrace and
        // execs the target; it never returns into this process's state.
        let pid = match unsafe { fork() } {
            Ok(ForkResult::Child) => {
                let result = lurk_cli::run_tracee(command, &[], &None);
                std::process::exit(if result.is_ok() { 0 } else { 1 });
            }
            Ok(ForkResult::Parent { child }) => child,
            Err(e) => return Err(format!("fork() failed: {e}")),
        };
        // Colors are kept even without a terminal; the UI renders them.
        console::set_colors_enabled(true);
        // binsider's colors.
        let style = StyleConfig {
            pid: Style::new().cyan(),
            syscall: Style::new().white().bold(),
            success: Style::new().green(),
            error: Style::new().red(),
            result: Style::new().yellow(),
            use_colors: true,
        };
        let args = Args {
            syscall_number: options.syscall_number,
            no_abbrev: options.no_abbrev,
            string_limit: options.string_limit,
            ..Args::default()
        };
        let mut syscalls = Vec::new();
        let mut tracer = Tracer::new(pid, args, Box::new(Cursor::new(&mut syscalls)), style)
            .map_err(|e| e.to_string())?;
        tracer.run_tracer().map_err(|e| e.to_string())?;
        let mut summary = Vec::new();
        tracer.set_output(Box::new(Cursor::new(&mut summary)));
        tracer.report_summary().map_err(|e| e.to_string())?;
        drop(tracer);
        let _ = waitpid(pid, Some(WaitPidFlag::WNOHANG));
        Ok(TraceData {
            syscalls: String::from_utf8_lossy(&syscalls)
                .lines()
                .map(ansi::parse)
                .collect(),
            summary: ansi::parse(&String::from_utf8_lossy(&summary)).text,
        })
    }
}
