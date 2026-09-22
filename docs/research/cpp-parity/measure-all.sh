#!/bin/bash
# Rebuild all four pairs and print the size/import numbers of docs/research/cpp-parity.md.
#
#   docs/research/cpp-parity/measure-all.sh
#
# Needs SYMDEV_EPOCROOT, SYMDEV_GXX, SYMDEV_LD, SYMDEV_GCC_LIB, SYMDEV_GCC_TARGET_LIB and a
# built symdev (`cargo build -p symdev-cli --bins`). No signing, no emulator: sizes only.
set -eu
HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO=$(cd "$HERE/../../.." && pwd)
SYMDEV=${SYMDEV:-$REPO/target/debug/symdev}

row() {  # <dir> <elf/exe stem>
    (cd "$1" && "$SYMDEV" build >/dev/null 2>&1) || { echo "BUILD FAILED: $1"; return 1; }
    echo "== $1"
    python3 "$HERE/measure.py" "$1/build/$2.elf" "$1/build/$2.exe" |
        grep -E '^(section\.(text|rodata|data|bss|plt|ARM)|e32_file_bytes|e32\.(code|data|bss)_size|import_dlls|import_ordinals_total|import_slots_total|import\[)'
}

row "$HERE/hello"  cpphello
row "$REPO/symbian-rs/examples/hello"  hello
row "$HERE/files"  cppfiles
row "$REPO/symbian-rs/examples/files"  filesdemo
row "$HERE/locale" cpplocale
row "$REPO/symbian-rs/examples/locale" localedemo
row "$HERE/ui"     cppui
row "$REPO/symbian-rs/examples/ui"     uidemo
