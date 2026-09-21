# 95 — the Options menu declared in Rust (2026-09-21)

`uidemo.exe`, 13 714 bytes, is `symbian-rs/examples/ui` with its Options menu moved out
of `symdev.toml` and into `App::menu`. `uidemo.rss` is the resource text symdev
generated: a `MENU_BAR` with one title, and a **`MENU_PANE` with no items at all**.
Every line is added at runtime through `CEikMenuPane::AddMenuItemL` from
`CShimAppUi::DynInitMenuPaneL`, each time the menu opens.

What the application writes, in full:

```rust
fn menu(&self, m: &mut Menu<Self>) {
    m.item("More bars",  |app| { … });
    m.item("Fewer bars", |app| { … });
    m.item("Reset",      |app| { … });
    m.exit("Exit");
}
```

No id, no constant, no `Command`. The number `HandleCommandL` carries is the line's
position (`0x4000 + n`) and never reaches the application. The action is a
non-capturing closure, so it is a `fn(&mut Bars)` the crate copies out of a second
`&self` call and runs once that borrow has ended — see
[`runtime-menu-design.md`](../../../docs/research/runtime-menu-design.md).

## The screenshots

| File | What it shows |
|---|---|
| `00-start.png` | the application as it launches: three bars, `bars=3 keys=0 cmd=0`, `Options`/`Exit` on the softkeys |
| `01-menu.png` | F1. The pane holds Avkon's own **"Show open apps."** — the framework inserts it — above the four lines Rust added into an empty compiled pane |
| `02-fewer.png` | `Down` `Return` on "Fewer bars": **`bars=2 keys=0 cmd=1`**. Experiment 91's proof, through a closure |
| `04-long-label.png` | one item temporarily given a 47-character label. Avkon ellipsises it; the application does **not** panic, which is what a `TBuf<40>` overflow would have done |
| `05-long-fired.png` | that truncated item still fires the right action: `bars=3 keys=0 cmd=1` |

Emulator only; no device. EKA2L1 from `~/src/EKA2L1-build`, N00 (RM-469), 900×600
window, keys through `docs/research/acceptance/emukey.py`.
