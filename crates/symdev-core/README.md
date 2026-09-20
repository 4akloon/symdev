# symdev-core

Shared vocabulary for the symdev workspace: the error type, small value types, and the traits
that separate *what* is done (build, package, deliver, emulate) from *where* it runs.

Depends only on `thiserror`. Every other crate except `symdev-manifest` and `symdev-uidcrc`
depends on this one.

## Contents

- **`Error` / `Result`**: one error enum for library paths, including
  `Error::NotImplemented { feature, milestone }` and `Error::Other(String)`.
- **Value types**: `Project` (root directory), `Artifact` (a file plus an optional `!:\…`
  install destination; `Artifact::exe` and `Artifact::installed`), `Package` (primary `.sisx`
  and companions), `RemotePath`, `PathStyle`, `Output`, `Capabilities`.
- **Traits**: `BuildBackend`, `PackageBackend`, `DeviceTransport`, `EmulatorBackend`,
  `DebuggerBackend`, `ExecutionEnvironment` (async `run` / `push` / `pull`), plus empty marker
  traits for platform, language, SDK, toolchain, test and runtime backends.
- **`LocalEnv`**: the `ExecutionEnvironment` that runs commands on the host.

## Status

`symdev-build` implements `BuildBackend` (`GcceBuild`) and `PackageBackend` (`SisPackage`);
`symdev-emulator` provides the EKA2L1 launcher. `DeviceTransport` and `DebuggerBackend` have no
implementation yet, and `symdev-emulator` does not yet implement `EmulatorBackend`.

## Testing

```bash
cargo test -p symdev-core --offline
```
