//! `ElfLinker`: which linker wrote an ELF, where elf2e32 must read the two differently.

/// The linker that wrote an ELF, as far as elf2e32 reads it differently.
///
/// Every ELF the SDK's elf2e32 was observed on came from GNU ld 2.29.1
/// (`arm-none-symbianelf`); those are read by observed behaviour only. An ELF from LLVM
/// lld (`rust-lld`, experiment 109) also goes through five rules derived from the ARM ABI
/// and lld's documented behaviour, verified in EKA2L1 (experiment 109), never observed
/// from the SDK's elf2e32 — a narrow exception the owner accepted on 2026-10-03, for lld
/// ELFs only (`elf/lld.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfLinker {
    /// GNU ld, or any linker not recognised as lld: observed rules only.
    Gnu,
    /// LLVM lld, recognised by the `Linker: LLD <version>` string it writes into
    /// `.comment`.
    Lld,
}

impl ElfLinker {
    /// lld appends `Linker: LLD <version>` to `.comment` in every non-relocatable link
    /// (experiment 109: `rust-lld` 23.1.1 writes `Linker: LLD 23.1.1 (…)`). GNU ld 2.29.1
    /// writes no linker string; its `.comment` holds only the inputs' compiler strings
    /// (`GCC: (GNU) 12.1.0`, `rustc version …`, the SDK's `ARM Linker, RVCT2.2 …`).
    const LLD_COMMENT: &[u8] = b"Linker: LLD ";

    /// The linker that wrote an ELF whose `.comment` section holds `comment` (`None`: no
    /// `.comment`, read as GNU's).
    pub(super) fn from_comment(comment: Option<&[u8]>) -> Self {
        let lld = comment.is_some_and(|c| {
            c.split(|&b| b == 0)
                .any(|s| s.starts_with(Self::LLD_COMMENT))
        });
        if lld { Self::Lld } else { Self::Gnu }
    }
}
