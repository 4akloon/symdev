//! `Request`: one JSON-RPC 2.0 request line, parameters by name (EKA2L1's control README).
use crate::json::quote;

/// A parameter value of the methods symdev calls.
#[derive(Debug, Clone, Copy)]
pub enum Param<'a> {
    Str(&'a str),
    /// A UID, written `"0x%08X"` (the README takes a number or `0x` and hex digits).
    Uid(u32),
    Names(&'a [&'a str]),
}

pub struct Request;

impl Request {
    pub fn line(id: u64, method: &str, params: &[(&str, Param)]) -> String {
        let fields: Vec<String> = params
            .iter()
            .map(|(name, value)| format!("{}:{}", quote(name), Self::value(value)))
            .collect();
        format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":{},\"params\":{{{}}}}}",
            quote(method),
            fields.join(",")
        )
    }

    fn value(value: &Param) -> String {
        match value {
            Param::Str(s) => quote(s),
            Param::Uid(uid) => format!("\"0x{uid:08X}\""),
            Param::Names(names) => {
                let names: Vec<String> = names.iter().map(|n| quote(n)).collect();
                format!("[{}]", names.join(","))
            }
        }
    }
}
