# Task 3 of native localisation: the run-time strings reader

Plan: `docs/superpowers/plans/2026-09-22-native-localisation.md`, Task 3. Shim
`symrs_rsc_open`/`symrs_rsc_read`, `symbian_core::locale::{Str, Text}`, proved on the
emulator with a scratch console example against the C++ heap target (open +4/+208 B,
string +1/+36 B, all returned on free).

## Findings

## Decisions

## Dead ends

## Next step

Read `symrs_shim.h`, `symrs_f32.cpp`, `fs/session.rs`, `symrs_process`; write the shim.
