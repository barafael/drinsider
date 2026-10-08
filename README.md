# drinsider

**Analyze ELF binaries like a normal person.**

drinsider is a windowed port of [binsider](https://github.com/orhun/binsider), the
terminal "swiss army knife for reverse engineers" by [@orhun](https://github.com/orhun).
It keeps binsider's concept, tabs and keyboard-driven interface, but renders them with
native [egui](https://github.com/emilk/egui) widgets in a desktop window or the browser.

**Try it in the browser:** https://barafael.github.io/drinsider/
(a small sample binary is loaded; drop your own ELF file on the page or press `Ctrl+O`).

## Tabs

| Tab | What it shows |
|---|---|
| **General** | `stat(1)`-style file information and the library dependencies (`ldd(1)`). `Enter` analyzes the selected library; `Backspace` goes back. |
| **Static** | File headers, ELF notes and six tables: program headers, section headers, symbols, dynamic symbols, dynamic section and relocations. |
| **Dynamic** | Runs the binary under ptrace and lists its system calls in color, like `strace(1)`. `Enter` shows the summary. Desktop only. |
| **Strings** | Printable strings with their offsets, like `strings(1)`. `+`/`-` change the minimum length. |
| **Hexdump** | Hex and ASCII view with a grid of values decoded at the cursor, in heh's layout. Read-only. |

## Keys

| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Next / previous tab |
| `j` `k` / arrows | Move the selection; `PageUp`/`PageDown`, `Ctrl+d`/`Ctrl+u` move by 5 |
| `h` `l` | Scroll the file info (General), switch table (Static), move the cursor (Hexdump) |
| `t` `b` / `Home` `End` | Top / bottom |
| `n` `p` | Cycle the focused block (Static), toggle endianness (Hexdump) |
| `/` or `Ctrl+f` | Search; `Enter` keeps the filter, `Esc` clears it |
| `Enter` | Details, run the trace, or open the selected library |
| `s` | Human readable sizes (Static) |
| `g` | Jump to an offset (Hexdump) |
| `r` | Re-run the trace (Dynamic) |
| `o` | Open the binsider documentation |
| `q` / `Esc` / `Ctrl+c` | Close the popup, or quit |

## Desktop

```sh
cargo run --release -- /bin/ls
```

The command line follows binsider: several files form a breadcrumb, `-n/--min-len`
sets the minimum string length, `-t/--tab` picks the first tab, and the trace flags
`--syscall-number`, `--no-abbrev` and `--string-limit` are passed to lurk. The last
file may carry arguments for tracing: `drinsider "ls -la"`.

## Web

```sh
rustup target add wasm32-unknown-unknown
trunk serve
```

The browser build has no filesystem or ptrace: file metadata is limited to the size,
library paths are not resolved, and the Dynamic tab is unavailable.

## License

MIT or Apache-2.0, like binsider, from which the data layer and command handling are
ported. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
