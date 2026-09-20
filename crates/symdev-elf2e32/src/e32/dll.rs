//! `E32Dll`: what makes an image a DLL.
use super::exports::E32Exports;

/// What makes an image a DLL: its exports, and whether writable static data is allowed
/// (`--dlldata`, MMP `EPOCALLOWDLLDATA`).
#[derive(Debug, Clone, Copy)]
pub struct E32Dll<'a> {
    pub exports: &'a E32Exports,
    pub allow_data: bool,
}
