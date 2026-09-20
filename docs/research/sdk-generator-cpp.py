#!/usr/bin/env python3
"""Stand-in for the SDK's Cygwin-built cpp.exe (GNU cpp 2.9x).

The generator drives cpp for two jobs: preprocessing bld.inf/.mmp project
files, and scanning sources for dependencies with -M -MG.  Three
translations are applied, all of them at the Windows-tool / Linux-tool
boundary:

  * GNU cpp 2.x's "-+" (C++ mode) is spelled "-x c++" by modern cpp.
  * Backslash separators become forward slashes.  gcc composes search-path
    prefixes itself before it ever calls open(), so winpath.so cannot fix a
    drive-less backslash path for it.  prepfile.pm already normalises
    forward slashes back out of the line markers it reads.
  * For project files only (.mmp / .inf): a statement continued with a
    trailing backslash is joined onto one output line and padded with blank
    lines, which is what cpp 2.9x emitted.  Modern cpp keeps the physical
    line break, and the generator's project-file parser is line-oriented, so
    without this the tail of a continued statement is read as a new
    statement.  The temporary file this needs is substituted back out of the
    line markers so the output is identical to a run on the original file.
"""
import os
import re
import subprocess
import sys
import tempfile

PROJECT_EXT = (".mmp", ".inf")


def join_continuations(text):
    lines = text.split("\n")
    out = []
    i = 0
    while i < len(lines):
        cur = lines[i].rstrip("\r")
        pad = 0
        while cur.endswith("\\") and i + 1 < len(lines):
            cur = cur[:-1] + " " + lines[i + 1].rstrip("\r")
            i += 1
            pad += 1
        out.append(cur)
        out.extend([""] * pad)
        i += 1
    return "\n".join(out)


def main():
    args = []
    for a in sys.argv[1:]:
        if a == "-+":
            continue
        args.append(a.replace("\\", "/"))

    src_idx = None
    for i, a in enumerate(args):
        if a.lower().endswith(PROJECT_EXT) and os.path.isfile(a):
            src_idx = i
    tmp = None
    orig = None
    if src_idx is not None:
        orig = args[src_idx]
        with open(orig, "rb") as f:
            raw = f.read().decode("latin-1")
        if re.search(r"\\\r?\n", raw):
            fd, tmp = tempfile.mkstemp(suffix=os.path.basename(orig))
            with os.fdopen(fd, "w", encoding="latin-1") as f:
                f.write(join_continuations(raw))
            args[src_idx] = tmp

    # GNU cpp 2.9x inserted a space of its own after an object-like macro
    # expansion; modern cpp keeps only the separator that was already there.
    # The generator's un-expansion step eats one space after each expanded
    # name, so without the extra space the following argument is glued onto
    # the directive keyword's first argument.  Put the space back.
    marks = []
    for i, a in enumerate(args):
        if a == "-D" and i + 1 < len(args):
            a = args[i + 1]
        if a.startswith("-D"):
            a = a[2:]
        if "=_____" in a:
            marks.append(a.split("=", 1)[1])

    cmd = ["g++", "-E", "-x", "c++"] + args
    try:
        p = subprocess.run(cmd, capture_output=True)
        out = p.stdout.decode("latin-1")
        for m in marks:
            out = out.replace(m, m + " ")
        if tmp:
            out = out.replace(tmp, orig)
        sys.stdout.write(out)
        sys.stderr.write(p.stderr.decode("latin-1"))
        return p.returncode
    finally:
        if tmp:
            os.unlink(tmp)


sys.exit(main())
