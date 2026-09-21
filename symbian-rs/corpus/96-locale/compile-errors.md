# 96 — every compile error the `locale!` design produces

Captured 2026-09-21 with `cargo build --release` for `arm-symbian-e32` on a throwaway
`examples/sizeprobe` holding the two-language table below. The probe is deleted; the
table it used is:

```rust
symbian_std::locale! {
    languages: english, french;

    GREETING = { english: "Hello from Rust SDK", french: "Bonjour du SDK Rust" },
}
```

## 1. A key the table does not have
```
error[E0425]: cannot find value `GREETNG` in this scope
  --> examples/sizeprobe/src/main.rs:16:20
   |
16 |     let greeting = GREETNG.get();
   |                    ^^^^^^^ not found in this scope
   |
note: similarly named constant `GREETING` defined here
  --> examples/sizeprobe/src/main.rs:8:1
   |
 8 | / symbian_std::locale! {
 9 | |     languages: english, french;
10 | |
11 | |     GREETING = { english: "Hello from Rust SDK", french: "Bonjour du SDK Rust" },
12 | | }
   | |_^
   = note: this error originates in the macro `symbian_std::locale` (in Nightly builds, run with -Z macro-backtrace for more info)
help: a constant with a similar name exists
   |
16 |     let greeting = GREETING.get();
   |                         +
```

## 2. A key present in one language and missing from the other
```
error[E0063]: missing field `french` in initializer of `Text`
  --> examples/sizeprobe/src/main.rs:8:1
   |
 8 | / symbian_std::locale! {
 9 | |     languages: english, french;
10 | |
11 | |     GREETING = { english: "Hello from Rust SDK" },
12 | | }
   | |_^ missing `french`
   |
   = note: this error originates in the macro `symbian_std::locale` (in Nightly builds, run with -Z macro-backtrace for more info)
```

## 3. A row naming a language the declaration does not have
```
error[E0560]: struct `Text` has no field named `german`
  --> examples/sizeprobe/src/main.rs:11:50
   |
11 |     GREETING = { english: "Hello from Rust SDK", german: "Hallo vom Rust SDK" },
   |                                                  ^^^^^^ `Text` does not have this field
   |
   = note: all struct fields are already assigned
```

## 4. A word in `languages:` that is not a TLanguage
```
error[E0425]: cannot find value `frnech` in module `$crate::locale::lang`
  --> examples/sizeprobe/src/main.rs:9:25
   |
 9 |     languages: english, frnech;
   |                         ^^^^^^ not found in `$crate::locale::lang`
   |
note: similarly named constant `french` defined here
  --> crates/symbian-core/src/locale/lang.rs:23:1
   |
23 | pub const french: Language = Language::from_code(2); // ELangFrench
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^
help: a constant with a similar name exists
   |
 9 -     languages: english, frnech;
 9 +     languages: english, french;
   |
error[E0560]: struct `Text` has no field named `french`
  --> examples/sizeprobe/src/main.rs:11:50
   |
11 |     GREETING = { english: "Hello from Rust SDK", french: "Bonjour du SDK Rust" },
```

## 5. Two rows with the same key
```
error[E0428]: the name `GREETING` is defined multiple times
  --> examples/sizeprobe/src/main.rs:8:1
   |
 8 | / symbian_std::locale! {
 9 | |     languages: english, french;
10 | |
11 | |     GREETING = { english: "a", french: "b" },
12 | |     GREETING = { english: "c", french: "d" },
13 | | }
   | | ^
   | | |
   | |_`GREETING` redefined here
   |   previous definition of the value `GREETING` here
   |
   = note: `GREETING` must be defined only once in the value namespace of this module
   = note: this error originates in the macro `symbian_std::locale` (in Nightly builds, run with -Z macro-backtrace for more info)
```

## 6. A translation that is not a string
```
error[E0308]: mismatched types
  --> examples/sizeprobe/src/main.rs:11:58
   |
11 |     GREETING = { english: "Hello from Rust SDK", french: 42 },
   |                                                          ^^ expected `&str`, found integer
```

## 1b. The same mistake when the table is a module, which is how it is meant to be used

`examples/locale` keeps its table in `src/strings.rs` and says `mod strings;`, so rustc
names the module rather than "this scope":

```
error[E0425]: cannot find value `GREETNG` in module `strings`
  --> examples/locale/src/main.rs:59:58
   |
59 |     let _ = writeln!(notes, "greeting_here={}", strings::GREETNG.get());
   |                                                          ^^^^^^^ not found in `strings`
   |
note: similarly named constant `GREETING` defined here
  --> examples/locale/src/strings.rs:14:1
   |
14 | / symbian_std::locale! {
15 | |     languages: english, ukrainian, french;
...
```
