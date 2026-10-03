//! `symdev test --emulator`: what `cargo test` does, for the project's own package (design
//! spec §7): install `build/<name>.sisx` on a device chosen as for `cargo run`, run it, read
//! the report it wrote to `E:\symdev\results\<uid3>.json` and print it as `libtest` does.
use std::io::IsTerminal;
use std::process::ExitCode;

use symdev_core::{Error, Result};
use symdev_emulator::control::ControlClient;

use crate::ld::LinkKind;
use crate::provision::Provision;
use crate::run::{ExeTarget, Interrupt, Runner, pick_device};

pub fn test_project(
    m: symdev_manifest::Manifest,
    emulator: bool,
    provision: &Provision,
) -> Result<ExitCode> {
    if !emulator {
        return Err(Error::Other(
            "symdev test needs --emulator: the host-side tests of the SDK crates are \
             `cargo test` in symbian-rs, and there is no other test backend yet"
                .into(),
        ));
    }
    let uid3 = m
        .symbian
        .uid3
        .ok_or_else(|| Error::Other("uid3 required for test (set symbian.uid3)".into()))?;
    let cwd = std::env::current_dir().map_err(|e| Error::Other(e.to_string()))?;
    let sisx = cwd.join("build").join(format!("{}.sisx", m.package.name));
    let mut target = ExeTarget::installed(sisx, uid3)?;
    target.kind = LinkKind::Test {
        name: m.package.name.clone(),
    };
    let device = pick_device(std::io::stdin().is_terminal(), provision)?;
    let client = ControlClient::connect(&device.socket)?;
    let interrupt = Interrupt::install()?;
    let exit = Runner::new(target, device, client).run(&interrupt, &mut std::io::stdout())?;
    if let Some(message) = &exit.message {
        eprintln!("{message}");
    }
    Ok(ExitCode::from(exit.code))
}
