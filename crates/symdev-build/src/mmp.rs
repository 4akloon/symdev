use crate::bld::ParseError;
use crate::model::{Mmp, MmpResource};

const PREPROCESSOR: &[&str] = &["if", "ifdef", "ifndef", "elif", "else", "endif", "include"];

fn preprocessor(tok: &str) -> bool {
    let Some(name) = tok.strip_prefix('#') else {
        return false;
    };
    PREPROCESSOR.iter().any(|p| name.eq_ignore_ascii_case(p))
}

fn parse_uid_token(tok: &str) -> Result<u32, ParseError> {
    let (radix, digits) =
        if let Some(hex) = tok.strip_prefix("0x").or_else(|| tok.strip_prefix("0X")) {
            (16, hex)
        } else {
            (10, tok)
        };
    u32::from_str_radix(digits, radix).map_err(|_| ParseError(format!("invalid UID: {tok}")))
}

fn rest(line: &str, tok: &str) -> String {
    line[tok.len()..].trim().to_string()
}

fn rest_tokens(line: &str, tok: &str) -> Vec<String> {
    rest(line, tok)
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

impl Mmp {
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let mut target = String::new();
        let mut target_type = String::new();
        let mut uid = Vec::new();
        let mut targetpath = None;
        let mut source = Vec::new();
        let mut source_sourcepath = Vec::new();
        let mut sourcepath = Vec::new();
        let mut systeminclude = Vec::new();
        let mut userinclude = Vec::new();
        let mut library = Vec::new();
        let mut staticlibrary = Vec::new();
        let mut capability = Vec::new();
        let mut epocstacksize = None;
        let mut epocheapsize = None;
        let mut epocallowdlldata = false;
        let mut resource = Vec::new();
        let mut resource_block: Option<MmpResource> = None;

        for raw in text.lines() {
            let line = match raw.find("//") {
                Some(i) => &raw[..i],
                None => raw,
            };
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with('#') {
                let Some(tok) = line.split_whitespace().next() else {
                    continue;
                };
                if preprocessor(tok) {
                    return Err(ParseError(format!("unsupported preprocessor: {tok}")));
                }
                continue;
            }

            let Some(tok) = line.split_whitespace().next() else {
                continue;
            };

            if let Some(mut block) = resource_block.take() {
                if tok.eq_ignore_ascii_case("END") {
                    resource.push(block);
                    continue;
                }
                if tok.eq_ignore_ascii_case("HEADER") {
                    block.header = true;
                } else if tok.eq_ignore_ascii_case("TARGETPATH") {
                    block.targetpath = Some(rest(line, tok));
                } else if tok.eq_ignore_ascii_case("LANG") {
                    block.lang.extend(rest_tokens(line, tok));
                } else {
                    return Err(ParseError(format!(
                        "TODO: START RESOURCE directive {tok} (not observed)"
                    )));
                }
                resource_block = Some(block);
                continue;
            }

            if tok.eq_ignore_ascii_case("START") {
                let mut words = line.split_whitespace().skip(1);
                let kind = words.next().unwrap_or("");
                if !kind.eq_ignore_ascii_case("RESOURCE") {
                    return Err(ParseError(format!("unknown directive: {line}")));
                }
                let file = words
                    .next()
                    .ok_or_else(|| ParseError("START RESOURCE without a file".into()))?;
                resource_block = Some(MmpResource {
                    file: file.to_string(),
                    sourcepath: sourcepath.last().cloned(),
                    targetpath: None,
                    header: false,
                    lang: Vec::new(),
                });
                continue;
            }

            if tok.eq_ignore_ascii_case("TARGET") {
                target = rest(line, tok);
            } else if tok.eq_ignore_ascii_case("TARGETTYPE") {
                target_type = rest(line, tok);
            } else if tok.eq_ignore_ascii_case("UID") {
                let vals = rest_tokens(line, tok);
                if vals.len() != 2 && vals.len() != 3 {
                    return Err(ParseError("UID requires two or three values".into()));
                }
                uid.clear();
                for v in vals {
                    uid.push(parse_uid_token(&v)?);
                }
            } else if tok.eq_ignore_ascii_case("TARGETPATH") {
                targetpath = Some(rest(line, tok));
            } else if tok.eq_ignore_ascii_case("SOURCE") {
                for file in rest_tokens(line, tok) {
                    source.push(file);
                    source_sourcepath.push(sourcepath.last().cloned());
                }
            } else if tok.eq_ignore_ascii_case("SOURCEPATH") {
                sourcepath.extend(rest_tokens(line, tok));
            } else if tok.eq_ignore_ascii_case("SYSTEMINCLUDE") {
                systeminclude.extend(rest_tokens(line, tok));
            } else if tok.eq_ignore_ascii_case("USERINCLUDE") {
                userinclude.extend(rest_tokens(line, tok));
            } else if tok.eq_ignore_ascii_case("LIBRARY") {
                library.extend(rest_tokens(line, tok));
            } else if tok.eq_ignore_ascii_case("STATICLIBRARY") {
                staticlibrary.extend(rest_tokens(line, tok));
            } else if tok.eq_ignore_ascii_case("CAPABILITY") {
                capability.extend(rest_tokens(line, tok));
            } else if tok.eq_ignore_ascii_case("EPOCSTACKSIZE") {
                epocstacksize = Some(rest(line, tok));
            } else if tok.eq_ignore_ascii_case("EPOCHEAPSIZE") {
                epocheapsize = Some(rest(line, tok));
            } else if tok.eq_ignore_ascii_case("EPOCALLOWDLLDATA") {
                epocallowdlldata = true;
            } else {
                return Err(ParseError(format!("unknown directive: {tok}")));
            }
        }

        if resource_block.is_some() {
            return Err(ParseError("unclosed START RESOURCE".into()));
        }

        if !target_type.eq_ignore_ascii_case("EXE") {
            return Err(ParseError(format!(
                "TARGETTYPE must be EXE, got {target_type}"
            )));
        }

        Ok(Self {
            target,
            target_type,
            uid,
            targetpath,
            source,
            source_sourcepath,
            sourcepath,
            systeminclude,
            userinclude,
            library,
            staticlibrary,
            capability,
            epocstacksize,
            epocheapsize,
            epocallowdlldata,
            resource,
        })
    }
}

#[test]
fn parse_exe_with_source() {
    let m = Mmp::parse("TARGET hello.exe\nTARGETTYPE EXE\nSOURCE hello.cpp\n").unwrap();
    assert_eq!(m.target, "hello.exe");
    assert_eq!(m.source, ["hello.cpp"]);
}

#[test]
fn dll_rejected() {
    assert!(Mmp::parse("TARGET x.dll\nTARGETTYPE DLL\n").is_err());
}

#[test]
fn option_gcce_rejected() {
    assert!(Mmp::parse("TARGET x.exe\nTARGETTYPE EXE\nOPTION_GCCE -O3\n").is_err());
}

#[test]
fn uid_two_hex_values() {
    let m = Mmp::parse("TARGET x.exe\nTARGETTYPE EXE\nUID 0x100039CE 0x1000008d\nSOURCE a.cpp\n")
        .unwrap();
    assert_eq!(m.uid, [0x1000_39CE, 0x1000_008d]);
}

#[test]
fn uid_three_values_hex_and_decimal() {
    let m =
        Mmp::parse("TARGET x.exe\nTARGETTYPE EXE\nUID 0x1000007a 100 0xE0000001\nSOURCE a.cpp\n")
            .unwrap();
    assert_eq!(m.uid, [0x1000_007a, 100, 0xE000_0001]);
}

#[test]
fn uid_one_value_rejected() {
    assert!(Mmp::parse("TARGET x.exe\nTARGETTYPE EXE\nUID 0x1000007a\nSOURCE a.cpp\n").is_err());
}

#[test]
fn uid_omitted() {
    let m = Mmp::parse("TARGET hello.exe\nTARGETTYPE EXE\nSOURCE hello.cpp\n").unwrap();
    assert!(m.uid.is_empty());
}

#[test]
fn start_resource_block_is_typed() {
    let m = Mmp::parse(
        "TARGET x.exe\nTARGETTYPE EXE\nSOURCEPATH ..\\src\nSOURCE a.cpp\nSOURCEPATH ..\\data\nSTART RESOURCE hello.rss\nHEADER\nTARGETPATH \\resource\\apps\nEND\n",
    )
    .unwrap();
    assert_eq!(
        m.resource,
        [MmpResource {
            file: "hello.rss".into(),
            sourcepath: Some("..\\data".into()),
            targetpath: Some("\\resource\\apps".into()),
            header: true,
            lang: Vec::new(),
        }]
    );
    assert!(m.targetpath.is_none());
}

#[test]
fn start_resource_rejects_unobserved_directive() {
    assert!(
        Mmp::parse(
            "TARGET x.exe\nTARGETTYPE EXE\nSOURCE a.cpp\nSTART RESOURCE a.rss\nTARGET b.rsc\nEND\n"
        )
        .is_err()
    );
}

#[test]
fn each_source_keeps_the_sourcepath_in_effect() {
    let m = Mmp::parse(
        "TARGET x.exe\nTARGETTYPE EXE\nSOURCEPATH ..\\src\nSOURCE a.cpp\nSOURCEPATH ..\\data\nSTART RESOURCE a.rss\nEND\nSOURCE b.cpp\n",
    )
    .unwrap();
    assert_eq!(m.source, ["a.cpp", "b.cpp"]);
    assert_eq!(
        m.source_sourcepath,
        [Some("..\\src".to_string()), Some("..\\data".to_string())]
    );
}
