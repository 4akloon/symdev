# DSO `.hash` section: behavioural spec

> **Provenance.** This is a clean-room behavioural specification. It was written
> on 2026-09-19 after reading the C++ source of elf2e32_next (EPL-1.0) and
> describes only the observable output. It contains no source code, no copied
> comments and no identifiers from that project. The implementer should work
> from this document alone. It was checked against 11 reference `.dso` files,
> reproducing each byte for byte (see "Verification").

## Scope

This spec covers the SysV hash table (`.hash`, section type `SHT_HASH` = 5) in
the import stub `.dso` that elf2e32_next writes for a DLL (`--dso=...`). All
words are 32-bit little-endian.

## Definitions

- **E**: the ordered list of exported symbols given to the DSO writer. It is
  sorted by ordinal, ascending. In normal builds the ordinals run 1, 2, ..., n
  with no gaps. Absent exports (those marked ABSENT in a `.def`) stay in the
  list at their ordinal.
- **n** = |E|, the number of exports. The writer refuses to produce a DSO when
  n = 0.
- **N** = n + 1, the number of `.dynsym` entries including the mandatory null
  symbol at index 0.
- The export at position k in E (k = 1..n) goes into `.dynsym` index k. With
  contiguous ordinals, its ordinal is also k.

## 1. Number of buckets

    nbucket = floor(N / 3) + (N mod 3)          where N = n + 1
    nchain  = N

The formula uses N (exports plus the null symbol), not n. It is not a prime
table and it is not monotonic in n. The next table covers every observed value:

| n (exports) | N  | floor(N/3) | N mod 3 | nbucket | nchain |
|------------:|---:|-----------:|--------:|--------:|-------:|
| 1           | 2  | 0          | 2       | **2**   | 2      |
| 2           | 3  | 1          | 0       | **1**   | 3      |
| 3           | 4  | 1          | 1       | **2**   | 4      |
| 4           | 5  | 1          | 2       | **3**   | 5      |
| 5           | 6  | 2          | 0       | **2**   | 6      |
| 6           | 7  | 2          | 1       | **3**   | 7      |
| 8           | 9  | 3          | 0       | **3**   | 9      |
| 10          | 11 | 3          | 2       | **5**   | 11     |
| 13          | 14 | 4          | 2       | **6**   | 14     |
| 17          | 18 | 6          | 0       | **6**   | 18     |
| 25          | 26 | 8          | 2       | **10**  | 26     |
| 40          | 41 | 13         | 2       | **15**  | 41     |

Section size in bytes = 4 × (2 + nbucket + nchain).

## 2. Hash function and hashed string

The writer uses the standard SysV / ELF `elf_hash` from the System V ABI. For
each byte c of the string, treated as unsigned:

1. h = (h × 16 + c), truncated to 32 bits.
2. g = h AND 0xF0000000.
3. If g is non-zero, h = h XOR (g >> 24).
4. h = h AND (NOT g).

h starts at 0. The result is the final h.

The string hashed is the symbol's name exactly as it is written into
`.dynstr`/`.strtab` for that `.dynsym` entry:

- The plain export name, such as a mangled C++ name like `_Z4F000i`. There is no
  `@version` or `@@...` suffix and no trailing NUL.
- If the `.def` gives the symbol an alias (export name different from the
  internal name), the alias is both written and hashed.
- For an absent export, the name written and hashed is
  `_._.absent_export_<ordinal>`, where the ordinal is in plain decimal with no
  padding.

## 3. Filling the bucket and chain arrays

Layout: word 0 = nbucket, word 1 = nchain, then nbucket bucket words, then
nchain chain words. Every bucket and chain word starts at 0 (STN_UNDEF). The
null symbol at index 0 is never inserted, so chain[0] stays 0.

Symbols are inserted one at a time, in E order (ascending ordinal, which is
ascending `.dynsym` index). The **symbol index** stored in the table is the
export's ordinal. With contiguous ordinals, the ordinal equals the `.dynsym`
index. To insert a symbol with index i and name s:

1. b = elf_hash(s) mod nbucket.
2. If bucket[b] is 0, set bucket[b] = i and stop.
3. Otherwise, walk the chain starting at c = bucket[b]. While chain[c] is not
   0:
   - If chain[c] equals i, the symbol is already present. Stop and change
     nothing.
   - Otherwise, set c = chain[c].
4. At the end of the chain (chain[c] = 0), set chain[c] = i. The new symbol is
   appended at the **tail**.

The result:

- Each bucket holds the **first** (lowest-index) symbol that hashed to it.
- Each chain runs in ascending index order. chain[i] is the next higher index
  in the same bucket, or 0 if i is the last one.
- The duplicate check in step 3 never fires when indices are unique, which is
  the normal case. It does not compare against the bucket head itself.

Equivalently: for each bucket, the members sorted ascending are i1 < i2 < ... <
ik. Then bucket = i1, chain[i1] = i2, ..., chain[ik] = 0.

## 4. Other places that depend on the hash

- The `.hash` section header has sh_type = 5 (`SHT_HASH`), sh_flags = 0,
  sh_entsize = 0, sh_addralign = 4, sh_info = 0, and sh_link = the section index
  of `.dynsym`.
- The `.dynamic` entry `DT_HASH` (tag 4) has d_val = the **file offset** of the
  `.hash` section. The writer's DSO is not loaded, so this is not a virtual
  address.
- Nothing else in the DSO depends on the hash table's contents. The same
  `elf_hash` function computes the two `vd_hash` fields in `.version_d`, over
  the DSO file name (basename of the `--dso` path) and the `--linkas` string.
  Those fields are independent of `.hash`.
- The hash section's size depends only on n. It shifts the file offsets of
  every section placed after it.

## Edge cases and caveats

- Ordinal vs index: the table stores ordinals. If E ever had non-contiguous
  ordinals, or ordinals greater than n, the stored values would not match
  `.dynsym` indices. An ordinal ≥ N would also fall outside the chain array. In
  the reference tool that is memory-unsafe, not a defined behaviour. A
  compatible writer should require ordinals to be exactly 1..n in order, which
  holds for every normal build. Otherwise it should treat the ordinal as the
  index and reject values ≥ N.
- n = 0 is rejected (no DSO written).

## Verification

A throwaway Python script, written only from this document, was checked
against 11 reference files. For each file it read `.dynsym` and `.strtab`,
rebuilt the table from the names, and compared the result with the file's
`.hash` bytes. All 11 were byte-identical:
`symdev-experiment-52/hash/f{1,4,5,6,8,10,13,17,25,40}.dso` and
`symdev-experiment-52/mathlib.dso` (n = 3). It also checked that the `DT_HASH`
value equals the `.hash` file offset and that `.hash` sh_link points at
`.dynsym`.
