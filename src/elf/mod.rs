//! ELF properties as stringified tables.
//!
//! Ported from binsider (https://github.com/orhun/binsider),
//! Copyright Orhun Parmaksız, MIT OR Apache-2.0.

/// ELF dynamic section.
pub mod dynamic;
/// ELF header.
pub mod header;
/// ELF notes.
pub mod notes;
/// ELF relocations.
pub mod relocations;
/// ELF symbols.
pub mod symbols;

use dynamic::Dynamic;
use elf::{ElfBytes, ParseError, endian::AnyEndian};
use header::{FileHeaders, ProgramHeaders, SectionHeaders};
use notes::Notes;
use relocations::Relocations;
use symbols::{DynamicSymbols, Symbols};

/// ELF property for receiving information.
pub trait Property<'a> {
    /// Returns the items.
    fn items(&self) -> Vec<Vec<String>>;
}

/// The tables shown as sub-tabs of the static analysis tab, in order.
pub const INFO_TABS: [Info; 6] = [
    Info::ProgramHeaders,
    Info::SectionHeaders,
    Info::Symbols,
    Info::DynamicSymbols,
    Info::Dynamics,
    Info::Relocations,
];

/// ELF information table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Info {
    /// Program headers (segments).
    ProgramHeaders,
    /// Section headers.
    SectionHeaders,
    /// Symbols.
    Symbols,
    /// Dynamic symbols.
    DynamicSymbols,
    /// Dynamics.
    Dynamics,
    /// Relocations.
    Relocations,
}

impl Info {
    /// Returns the title.
    pub fn title(self) -> &'static str {
        match self {
            Info::ProgramHeaders => "Program Headers / Segments",
            Info::SectionHeaders => "Section Headers",
            Info::Symbols => "Symbols",
            Info::DynamicSymbols => "Dynamic Symbols",
            Info::Dynamics => "Dynamic",
            Info::Relocations => "Relocations",
        }
    }

    /// Returns the column headers.
    pub fn headers(self) -> &'static [&'static str] {
        match self {
            Info::ProgramHeaders => &[
                "Type", "Offset", "VirtAddr", "PhysAddr", "FileSiz", "MemSiz", "Flags", "Align",
            ],
            Info::SectionHeaders => &[
                "Name", "Type", "Addr", "Offset", "Size", "EntSiz", "Flags", "Link", "Info",
                "Align",
            ],
            Info::Symbols => &["Name", "Type", "Value", "Siz", "Bind", "Vis", "Ndx"],
            Info::DynamicSymbols => &["Name", "Reqs", "Type", "Value", "Siz", "Bind", "Vis", "Ndx"],
            Info::Dynamics => &["Tag", "Value"],
            Info::Relocations => &["Type", "Symbol", "Offset", "Addend"],
        }
    }
}

/// ELF wrapper.
#[derive(Debug)]
pub struct Elf {
    /// File headers.
    pub file_headers: FileHeaders,
    /// Program headers.
    pub program_headers: ProgramHeaders,
    /// Section headers.
    pub section_headers: SectionHeaders,
    /// Symbols.
    pub symbols: Symbols,
    /// Dynamic symbols.
    pub dynamic_symbols: DynamicSymbols,
    /// Dynamic.
    pub dynamic: Dynamic,
    /// Relocations.
    pub relocations: Relocations,
    /// Notes.
    pub notes: Notes,
}

impl<'a> TryFrom<ElfBytes<'a, AnyEndian>> for Elf {
    type Error = ParseError;
    fn try_from(elf_bytes: ElfBytes<'a, AnyEndian>) -> Result<Self, Self::Error> {
        Ok(Self {
            file_headers: FileHeaders::from(elf_bytes.ehdr),
            program_headers: ProgramHeaders::from(match elf_bytes.segments() {
                Some(segments) => segments.iter().collect(),
                None => vec![],
            }),
            section_headers: SectionHeaders::try_from(elf_bytes.section_headers_with_strtab()?)
                .unwrap_or_default(),
            symbols: Symbols::try_from(elf_bytes.symbol_table()?).unwrap_or_default(),
            dynamic_symbols: DynamicSymbols::try_from((
                elf_bytes.dynamic_symbol_table()?,
                elf_bytes.symbol_version_table()?,
            ))
            .unwrap_or_default(),
            dynamic: Dynamic::try_from(elf_bytes.dynamic()?).unwrap_or_default(),
            relocations: Relocations::try_from(&elf_bytes).unwrap_or_default(),
            notes: Notes::try_from(&elf_bytes).unwrap_or_default(),
        })
    }
}

impl Elf {
    /// Returns the rows of the given table.
    pub fn rows(&self, info: Info) -> Vec<Vec<String>> {
        match info {
            Info::ProgramHeaders => self.program_headers.items(),
            Info::SectionHeaders => self.section_headers.items(),
            Info::Symbols => self.symbols.items(),
            Info::DynamicSymbols => self.dynamic_symbols.items(),
            Info::Dynamics => self.dynamic.items(),
            Info::Relocations => self.relocations.items(),
        }
    }

    /// Toggles human readable sizes in the program and section headers.
    pub fn toggle_readability(&mut self) {
        self.program_headers.toggle_readability();
        self.section_headers.toggle_readability();
    }
}
