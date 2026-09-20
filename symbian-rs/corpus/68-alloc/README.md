# 68 — the Rust heap on the Symbian heap (2026-09-20)

`allocdemo.exe`, 11 499 bytes, is `symbian-rs/examples/alloc` built by `symdev build`
(cargo → the recorded link line plus `-u _Z7E32Mainv` and `--gc-sections` → the native
post-linker). It grows a `Vec<u16>` and a `String` through `User::ReAlloc`, builds an
`HBuf16` and shows it as a `const TDesC16&`, allocates a 32-aligned `Box` through the
allocator's padding path, drops everything and prints numbers taken from the data.

In EKA2L1:

```
[Service.Notifier]: Trying to display: heap 0,1,4,9,16,
[Service.Notifier]: Trying to display: alloc sum=85344 cap=64 a8=0 a32=0 byte=a5 heap=16
```

Experiment record: `docs/research/experiment-backlog.md` §68.
