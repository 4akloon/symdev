# runtime-menu

Task: move the Options menu out of `symdev.toml` into a runtime `fn menu(&self, m: &mut Menu)` API with closures, deleting `Command`/command ids from the application-facing API.

## Findings

## Decisions

## Dead ends

## Next step

Read the current `[[ui.menu]]` path end to end: manifest -> macro -> shim -> resource.
