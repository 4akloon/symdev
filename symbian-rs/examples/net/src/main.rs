//! Smoke test for step 74, first cut: does the emulator's socket server answer at all?
#![no_std]

use symbian_core::net::{HostResolver, Socket, with_session};
use symbian_core::{Buf16, Result, user};

/// The host's loopback, where the test server runs.
const HOST: u32 = 0x7f00_0001;
const PORT: u16 = 18974;

fn code(result: Result<()>) -> i64 {
    match result {
        Ok(()) => 0,
        Err(e) => i64::from(e.code()),
    }
}

#[symbian_std::main]
fn main() -> Result<()> {
    let mut note = Buf16::<200>::new();
    note.push_str("net74")?;

    let outcome = with_session(|server| {
        note.push_str(" session=0")?;

        let mut resolver = HostResolver::open(server)?;
        note.push_str(" resolver=0")?;
        match resolver.lookup("localhost", PORT) {
            Ok(mut addr) => {
                note.push_str(" lookup=")?;
                note.append_num(i64::from(addr.to_v4()?))?;
            }
            Err(e) => {
                note.push_str(" lookup_err=")?;
                note.append_num(i64::from(e.code()))?;
            }
        }

        let mut socket = Socket::tcp(server)?;
        note.push_str(" tcp=0")?;
        let mut addr = symbian_core::net::InetAddr::v4(HOST, PORT);
        note.push_str(" connect=")?;
        note.append_num(code(socket.connect(&mut addr)))?;

        note.push_str(" send=")?;
        note.append_num(code(socket.send(b"ping\n")))?;

        let mut buf = [0u8; 32];
        match socket.recv(&mut buf) {
            Ok(n) => {
                note.push_str(" recv=")?;
                note.append_num(n as i64)?;
                note.push_str(" b0=")?;
                note.append_num(i64::from(buf[0]))?;
            }
            Err(e) => {
                note.push_str(" recv_err=")?;
                note.append_num(i64::from(e.code()))?;
            }
        }
        Ok(())
    });

    if let Err(e) = outcome {
        note.push_str(" ERR=")?;
        note.append_num(i64::from(e.code()))?;
    }
    note.push_str(" alive")?;
    user::info_print(&note)?;
    user::after(3_000_000);
    Ok(())
}
