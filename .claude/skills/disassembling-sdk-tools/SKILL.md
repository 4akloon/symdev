---
name: disassembling-sdk-tools
description: Use when you need to know exactly what an SDK binary does (bmconv, mifconv, svgtbinencode, makesis, signsis, elf2e32, rcomp, uidcrc, makekeys) — the installed Ghidra headless decompiler, the cheap workflow, and the clean-room rule that goes with it.
---

# Disassembling an SDK tool

The repository owner approved disassembling SDK binaries when it makes a byte-exact reimplementation easier. Reach for it early instead of guessing from black-box probes alone — but keep the clean-room split below.

## Clean-room split (not optional)

The agent that reads the disassembly **writes a prose specification** into `docs/research/<tool>-spec.md` and nothing else: no code, no pseudo-code transcribed from the listing, no function or symbol names, no addresses, no string literals from the binary. Format constants that appear in output files are fine. A **different** agent implements from that spec plus golden bytes and never reads the disassembly. Record the pair in `docs/research/licensing.md`.

## What is installed

| Tool | Path | For |
|---|---|---|
| Ghidra 12.1.3 headless | `~/.local/opt/ghidra/support/analyzeHeadless` | decompiling to C-like pseudo-code |
| JDK 21 | `~/.local/opt/jdk21` (`export JAVA_HOME=~/.local/opt/jdk21`) | required by Ghidra |
| `DecompileTo.java` | `~/.local/share/ghidra-scripts/` | function index, or decompile chosen functions to a file |
| Python venv | `~/.local/revenv/bin/python` with `pefile`, `capstone`, `lief` | PE headers, imports/exports, scripted slices |
| Always there | `objdump -d -M intel`, `strings`, `nm`, `readelf` | quick looks, address slices |

## The cheap loop

1. **Import once per binary** (about 10–60 s; the project is reusable):

   ```bash
   export JAVA_HOME=~/.local/opt/jdk21
   ~/.local/opt/ghidra/support/analyzeHeadless /tmp/ghidra-proj <project> \
     -import /home/genius/sdk/S60_3rd_FP2/epoc32/tools/<tool>.exe >/dev/null 2>&1
   ```

2. **Get the function index** (address, size in bytes, name) — a few hundred lines, cheap to grep:

   ```bash
   ~/.local/opt/ghidra/support/analyzeHeadless /tmp/ghidra-proj <project> -process <tool>.exe \
     -noanalysis -scriptPath ~/.local/share/ghidra-scripts \
     -postScript DecompileTo.java /tmp/<tool>-index.txt >/dev/null 2>&1
   ```

3. **Find where to look** without reading anything: `strings -t x <tool>.exe | grep -i <message>` gives the address of a diagnostic, and `objdump -s -j .rdata` plus a search for that address finds the code that references it. Error messages are the best map of a tool's decision points.

4. **Decompile only the functions you need**, by address or by name substring, into a file:

   ```bash
   ~/.local/opt/ghidra/support/analyzeHeadless /tmp/ghidra-proj <project> -process <tool>.exe \
     -noanalysis -scriptPath ~/.local/share/ghidra-scripts \
     -postScript DecompileTo.java /tmp/fn.c 0x00401c08 0x00404349 >/dev/null 2>&1
   ```

   Then read `/tmp/fn.c` with `sed -n` ranges. A 1.8 KB function is ~400 lines of C; most are 30–80. Never dump a whole binary's disassembly into the conversation.

5. **Confirm black-box.** A hypothesis from the listing costs nothing to check: craft a minimal input, run the tool under Wine, diff the bytes. Byte evidence beats a reading of the code, and the spec needs those examples anyway.

## Token discipline

- Keep notes, listings and scripts in your own scratch directory (`/tmp/claude-1000/<tool>-spec-work/`); only conclusions go into the conversation or the spec.
- Slice, never dump: `sed -n '120,180p'`, `objdump -d --start-address=0x401c08 --stop-address=0x401d00`.
- Prefer the decompiler over raw assembly; prefer strings and cross-references over reading functions in order.
- Script the repetitive parts with the venv (`~/.local/revenv/bin/python -c 'import pefile …'`) and print only what you need.

## Wine safety

Running the tool is part of the loop, so: after a crash a `winedbg --auto` process lingers — find it with `pgrep -f '^winedbg'` (keep the `^`, an unanchored pattern also matches your own shell and kills it) and `kill -9` only those PIDs. Never `wineserver -k`, never `pkill wine`: the user may have their own Wine programs open.
