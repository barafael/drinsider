//! Binary analyzer: parses the ELF file and gathers the data shown in the UI.
//!
//! Ported from binsider (https://github.com/orhun/binsider),
//! Copyright Orhun Parmaksız, MIT OR Apache-2.0.

use crate::elf::Elf;
use crate::error::Result;
use crate::file::FileInfo;
use elf::{ElfBytes, endian::AnyEndian};
use std::path::PathBuf;

/// Binary analyzer.
#[derive(Debug)]
pub struct Analyzer {
    /// Path of the analyzed file (just the name for browser uploads).
    pub path: PathBuf,
    /// Raw bytes of the file (strings extraction, hexdump).
    pub bytes: Vec<u8>,
    /// General file information.
    pub file: FileInfo,
    /// Parsed ELF properties.
    pub elf: Elf,
    /// Library dependencies.
    pub dependencies: Vec<Dependency>,
    /// Whether dependency paths were resolved against this system.
    pub resolved: bool,
}

/// A shared library the binary links against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    /// Library name, e.g. `libc.so.6`.
    pub name: String,
    /// Resolved path, `None` if not found (or not resolvable in the browser).
    pub path: Option<PathBuf>,
}

impl Analyzer {
    /// Analyzes an in-memory file without touching the filesystem: file
    /// metadata is limited to the size, and dependencies are read from the
    /// ELF (`DT_NEEDED`, `PT_INTERP`) without resolving their paths.
    pub fn from_bytes(name: &str, bytes: Vec<u8>) -> Result<Self> {
        let elf_bytes = ElfBytes::<AnyEndian>::minimal_parse(&bytes)?;
        let mut dependencies = needed_libraries(&elf_bytes, &bytes);
        let elf = Elf::try_from(elf_bytes)?;
        sort_dependencies(&mut dependencies);
        Ok(Self {
            path: PathBuf::from(name),
            file: FileInfo::from_bytes(name, bytes.len()),
            bytes,
            elf,
            dependencies,
            resolved: false,
        })
    }
}

/// binsider's order: `lib*` entries first, then alphabetical.
fn sort_dependencies(dependencies: &mut [Dependency]) {
    dependencies.sort_by(|a, b| {
        let (a_lib, b_lib) = (a.name.starts_with("lib"), b.name.starts_with("lib"));
        b_lib.cmp(&a_lib).then_with(|| a.name.cmp(&b.name))
    });
}

/// The interpreter and `DT_NEEDED` entries, unresolved.
fn needed_libraries(elf: &ElfBytes<'_, AnyEndian>, bytes: &[u8]) -> Vec<Dependency> {
    let mut names = Vec::new();
    if let Some(interpreter) = interpreter(elf, bytes) {
        names.push(interpreter);
    }
    if let (Ok(Some(dynamic)), Ok(Some((_, strtab)))) = (elf.dynamic(), elf.dynamic_symbol_table()) {
        for entry in dynamic.iter().filter(|d| d.d_tag == elf::abi::DT_NEEDED) {
            if let Ok(name) = strtab.get(entry.d_val() as usize) {
                names.push(name.to_string());
            }
        }
    }
    names.into_iter().map(|name| Dependency { name, path: None }).collect()
}

/// The program interpreter from `PT_INTERP`, e.g. `/lib64/ld-linux-x86-64.so.2`.
fn interpreter(elf: &ElfBytes<'_, AnyEndian>, bytes: &[u8]) -> Option<String> {
    let segment = elf.segments()?.iter().find(|s| s.p_type == elf::abi::PT_INTERP)?;
    let start = usize::try_from(segment.p_offset).ok()?;
    let end = start.checked_add(usize::try_from(segment.p_filesz).ok()?)?;
    let raw = bytes.get(start..end)?;
    let raw = raw.split(|b| *b == 0).next()?;
    Some(String::from_utf8_lossy(raw).into_owned())
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::{Analyzer, Dependency, sort_dependencies};
    use crate::elf::Elf;
    use crate::error::Result;
    use crate::file::FileInfo;
    use elf::{ElfBytes, endian::AnyEndian};
    use lddtree::DependencyAnalyzer;
    use std::{fs, path::Path};

    impl Analyzer {
        /// Reads and analyzes the file at `path`.
        pub fn new(path: &Path) -> Result<Self> {
            let bytes = fs::read(path)?;
            let elf = Elf::try_from(ElfBytes::<AnyEndian>::minimal_parse(&bytes)?)?;
            let file = FileInfo::new(path)?;
            let dependencies = extract_libs(path)?;
            Ok(Self {
                path: path.to_path_buf(),
                bytes,
                file,
                elf,
                dependencies,
                resolved: true,
            })
        }
    }

    /// Resolves the library dependencies like `ldd(1)`, including the
    /// interpreter (binsider's lddtree 0.4 listed it as a library too).
    ///
    /// lddtree lists unresolved libraries with the bare name as `path` and no
    /// `realpath`; those become `path: None`.
    pub fn extract_libs(path: &Path) -> Result<Vec<Dependency>> {
        let tree = DependencyAnalyzer::default().analyze(path)?;
        let mut dependencies: Vec<Dependency> = tree
            .libraries
            .into_iter()
            .map(|(name, lib)| Dependency {
                name,
                path: lib.realpath.or_else(|| lib.path.is_absolute().then_some(lib.path)),
            })
            .collect();
        if let Some(interpreter) = tree.interpreter
            && !dependencies.iter().any(|d| d.name == interpreter)
        {
            let path = fs::canonicalize(&interpreter).ok();
            dependencies.push(Dependency { name: interpreter, path });
        }
        sort_dependencies(&mut dependencies);
        Ok(dependencies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = include_bytes!("../assets/sample");

    #[test]
    fn sample_from_bytes() {
        let analyzer = Analyzer::from_bytes("sample", SAMPLE.to_vec()).expect("parse sample");
        assert!(!analyzer.resolved);
        let names: Vec<_> = analyzer.dependencies.iter().map(|d| d.name.as_str()).collect();
        assert!(names.iter().any(|n| n.starts_with("libc.so")), "{names:?}");
        assert!(names.iter().any(|n| n.starts_with("libm.so")), "{names:?}");
        assert!(names.iter().any(|n| n.contains("ld-linux")), "{names:?}");
        assert!(analyzer.dependencies.iter().all(|d| d.path.is_none()));
        assert!(Analyzer::from_bytes("text", b"hello".to_vec()).is_err());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn analyze_bin_ls() {
        let analyzer = Analyzer::new(std::path::Path::new("/bin/ls")).expect("analyze /bin/ls");
        assert!(analyzer.resolved);
        assert_eq!(analyzer.file.name, "ls");
        let has = |pred: &dyn Fn(&Dependency) -> bool| analyzer.dependencies.iter().any(pred);
        assert!(has(&|d| d.name.starts_with("libc.so") && d.path.as_ref().is_some_and(|p| p.is_absolute())));
        // The interpreter is listed, like binsider does.
        assert!(has(&|d| d.name.starts_with('/') && d.name.contains("ld-linux")));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn non_elf_is_an_error() {
        assert!(Analyzer::new(std::path::Path::new("/etc/hostname")).is_err());
        assert!(Analyzer::new(std::path::Path::new("/nonexistent/file")).is_err());
    }
}
