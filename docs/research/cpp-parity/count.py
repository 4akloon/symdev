#!/usr/bin/env python3
"""Author-written files and lines for one example, counted the same way on both sides.

    count.py <dir> [<extra file> ...]

Prints the file count, the raw line count and the "code" line count — raw minus
blank lines and minus lines whose first non-space characters start a comment
(`//`, `/*`, `*`, `#` only when it is an `.rls`/`.rss` comment is *not* stripped
because `#ifdef` is code there; `//!` and `///` in Rust are stripped as comments).

Generated files are skipped: anything under `build/` or `target/`, plus lock files.
"""

import os
import sys

SOURCE_EXT = {
    ".cpp", ".h", ".hrh", ".rss", ".rls", ".mmp", ".rs", ".toml", ".svg",
}
SKIP_NAMES = {"Cargo.lock", "bld.inf"}  # bld.inf counted separately below
SKIP_DIRS = {"build", "target", ".git"}


def is_comment(line: str) -> bool:
    s = line.strip()
    return s.startswith("//") or s.startswith("/*") or s.startswith("*")


def count(path):
    raw = code = 0
    with open(path, "rb") as f:
        for line in f:
            text = line.decode("utf-8", "replace")
            raw += 1
            if text.strip() and not is_comment(text):
                code += 1
    return raw, code


def walk(root):
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        for name in sorted(filenames):
            if name in SKIP_NAMES and name != "bld.inf":
                continue
            ext = os.path.splitext(name)[1]
            if name == "bld.inf" or ext in SOURCE_EXT:
                yield os.path.join(dirpath, name)


def main():
    paths = []
    for arg in sys.argv[1:]:
        if os.path.isdir(arg):
            paths.extend(walk(arg))
        else:
            paths.append(arg)
    files = raw_total = code_total = 0
    for p in sorted(set(paths)):
        raw, code = count(p)
        files += 1
        raw_total += raw
        code_total += code
        print(f"{raw:6d} {code:6d}  {p}")
    print(f"{raw_total:6d} {code_total:6d}  TOTAL ({files} files)")


if __name__ == "__main__":
    main()
