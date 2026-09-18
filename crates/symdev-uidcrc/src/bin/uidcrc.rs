use std::process::ExitCode;

use symdev_uidcrc::UidCrc;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let tokens = &args[1..];
    match tokens {
        [a, b, c] | [a, b, c, ..] => {
            let uid1 = UidCrc::parse_uid(a).ok_or_else(|| format!("invalid uid1: {a}"))?;
            let uid2 = UidCrc::parse_uid(b).ok_or_else(|| format!("invalid uid2: {b}"))?;
            let uid3 = UidCrc::parse_uid(c).ok_or_else(|| format!("invalid uid3: {c}"))?;
            let crc = UidCrc::new(uid1, uid2, uid3);
            if let Some(out) = tokens.get(3) {
                std::fs::write(out, crc.bytes()).map_err(|e| format!("write {out}: {e}"))?;
            } else {
                println!("{}", crc.line());
            }
            if tokens.len() > 4 {
                return Err("Usage: uidcrc <uid1> <uid2> <uid3> [<outputfile>]".into());
            }
            Ok(())
        }
        _ => Err("Usage: uidcrc <uid1> <uid2> <uid3> [<outputfile>]".into()),
    }
}
