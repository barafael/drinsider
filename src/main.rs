//! drinsider: a windowed port of binsider's keyboard-driven ELF analyzer.

mod analyzer;
mod ansi;
mod app;
mod command;
mod elf;
mod error;
mod file;
#[cfg(not(target_arch = "wasm32"))]
mod geometry;
mod list;
#[cfg(target_os = "linux")]
mod portal;
mod state;
mod strings;
mod tracer;
mod ui;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(target_arch = "wasm32")]
fn main() {
    web::start();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let (files, options) = match cli::parse(std::env::args_os().skip(1)) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let saved = geometry::Geometry::load();
    let viewport = egui::ViewportBuilder::default()
        .with_title("drinsider")
        .with_app_id("drinsider")
        .with_inner_size([1280.0, 860.0])
        .with_min_inner_size([720.0, 480.0]);
    let native_options = eframe::NativeOptions {
        viewport: match saved {
            Some(geometry) => geometry.apply(viewport),
            None => viewport,
        },
        // drinsider restores the window itself; see `geometry`.
        persist_window: false,
        ..Default::default()
    };
    let tracker = geometry::Tracker::new(saved.and_then(|g| g.position));
    let result = eframe::run_native(
        "drinsider",
        native_options,
        Box::new(move |cc| {
            Ok(Box::new(
                app::DrinsiderApp::new(cc, |ctx| state::State::new(&files, options, ctx))
                    .with_window(tracker),
            ))
        }),
    );
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

/// binsider's command line, without clap.
#[cfg(not(target_arch = "wasm32"))]
mod cli {
    use crate::state::{Options, Tab};
    use std::{env, ffi::OsString, path::PathBuf, process};

    const USAGE: &str = "\
Usage: drinsider [OPTIONS] [FILE]...

Analyze ELF binaries in a window. Without FILE, analyzes itself.
Several files form a breadcrumb (Backspace goes back); the last is shown.
The last FILE may carry arguments for tracing: drinsider \"ls -la\"

Options:
  -n, --min-len <N>        Minimum length of strings [default: 15]
  -t, --tab <TAB>          Initial tab: general, static, dynamic, strings, hexdump
      --syscall-number     Display system call numbers during dynamic analysis
      --no-abbrev          Print un-abbreviated strings during dynamic analysis
      --string-limit <N>   Maximum string argument size during dynamic analysis
  -h, --help               Print this help
  -V, --version            Print the version";

    /// Parses the arguments into the breadcrumb of files and the options.
    pub fn parse(
        args: impl IntoIterator<Item = OsString>,
    ) -> Result<(Vec<PathBuf>, Options), String> {
        let mut options = Options::default();
        let mut files = Vec::new();
        let mut args = args.into_iter();
        let number = |flag: &str, value: Option<OsString>| -> Result<usize, String> {
            value
                .and_then(|v| v.to_str()?.parse().ok())
                .ok_or_else(|| format!("{flag} needs a number\n\n{USAGE}"))
        };
        while let Some(arg) = args.next() {
            match arg.to_str() {
                Some("-h" | "--help") => {
                    println!("{USAGE}");
                    process::exit(0);
                }
                Some("-V" | "--version") => {
                    println!("drinsider {}", env!("CARGO_PKG_VERSION"));
                    process::exit(0);
                }
                Some("-n" | "--min-len") => {
                    options.min_strings_len = number("--min-len", args.next())?.max(1)
                }
                Some("-t" | "--tab") => {
                    let value = args.next().unwrap_or_default();
                    let name = value.to_string_lossy();
                    options.tab = Tab::from_name(&name)
                        .ok_or_else(|| format!("unknown tab `{name}`\n\n{USAGE}"))?;
                }
                Some("--syscall-number") => options.trace.syscall_number = true,
                Some("--no-abbrev") => options.trace.no_abbrev = true,
                Some("--string-limit") => {
                    options.trace.string_limit = Some(number("--string-limit", args.next())?)
                }
                Some(flag) if flag.starts_with('-') && flag.len() > 1 => {
                    return Err(format!("unknown option `{flag}`\n\n{USAGE}"));
                }
                _ => files.push(PathBuf::from(arg)),
            }
        }
        if options.trace.no_abbrev && options.trace.string_limit.is_some() {
            return Err(format!(
                "--no-abbrev conflicts with --string-limit\n\n{USAGE}"
            ));
        }
        if files.is_empty() {
            files.push(env::current_exe().map_err(|e| e.to_string())?);
        }
        // binsider: the last file may be "binary arg1 arg2".
        let last = files.pop().expect("at least one file");
        let mut parts = last
            .to_string_lossy()
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let last = if !last.exists() && parts.len() > 1 {
            options.arguments = parts.split_off(1);
            PathBuf::from(parts.remove(0))
        } else {
            last
        };
        files.push(last);
        Ok((files.into_iter().map(resolve).collect(), options))
    }

    /// Resolves a bare command name through `$PATH`; returns the input unchanged otherwise.
    fn resolve(path: PathBuf) -> PathBuf {
        if path.exists() || path.components().count() > 1 {
            return path;
        }
        env::var_os("PATH")
            .and_then(|paths| {
                env::split_paths(&paths)
                    .map(|dir| dir.join(&path))
                    .find(|c| c.is_file())
            })
            .unwrap_or(path)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn parse_strs(args: &[&str]) -> Result<(Vec<PathBuf>, Options), String> {
            parse(args.iter().map(OsString::from))
        }

        #[test]
        fn flags_files_and_arguments() {
            let (files, options) = parse_strs(&[
                "-n",
                "8",
                "--tab",
                "static-analysis",
                "--syscall-number",
                "/bin/cat",
                "ls -la",
            ])
            .unwrap();
            assert_eq!(options.min_strings_len, 8);
            assert_eq!(options.tab, Tab::StaticAnalysis);
            assert!(options.trace.syscall_number);
            assert_eq!(options.arguments, vec!["-la".to_string()]);
            assert_eq!(files[0], PathBuf::from("/bin/cat"));
            assert!(files[1].is_absolute() && files[1].ends_with("ls"));
        }

        #[test]
        fn rejects_bad_input() {
            assert!(parse_strs(&["--no-abbrev", "--string-limit", "4"]).is_err());
            assert!(parse_strs(&["-t", "nope"]).is_err());
            assert!(parse_strs(&["-n", "x"]).is_err());
            assert!(parse_strs(&["--bogus"]).is_err());
        }
    }
}
