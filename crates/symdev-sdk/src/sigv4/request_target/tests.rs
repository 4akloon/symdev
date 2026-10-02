use super::RequestTarget;

fn parse(url: &str) -> RequestTarget {
    RequestTarget::parse(url).unwrap()
}

#[test]
fn query_is_decoded_encoded_once_and_sorted_by_key_then_value() {
    let target = parse("https://h/?prefix=a%20b&delimiter=/&max-keys=2&a=2&a=1&flag");
    assert_eq!(
        target.canonical_query,
        "a=1&a=2&delimiter=%2F&flag=&max-keys=2&prefix=a%20b"
    );
}

#[test]
fn path_with_semicolon_or_space_is_percent_encoded_once() {
    assert_eq!(
        parse("https://h/sdk/a;b c/x.tar.gz").canonical_uri,
        "/sdk/a%3Bb%20c/x.tar.gz"
    );
    assert_eq!(
        parse("https://h/sdk/a%3Bb%20c/x.tar.gz").canonical_uri,
        "/sdk/a%3Bb%20c/x.tar.gz"
    );
    assert_eq!(parse("https://h/~a-b_c.d/%7E").canonical_uri, "/~a-b_c.d/~");
}

#[test]
fn an_empty_path_is_the_root_and_a_fragment_is_dropped() {
    assert_eq!(parse("https://h").canonical_uri, "/");
    let target = parse("https://h?x=1#frag");
    assert_eq!(
        (
            target.canonical_uri.as_str(),
            target.canonical_query.as_str()
        ),
        ("/", "x=1")
    );
}

#[test]
fn host_keeps_a_non_default_port_and_drops_a_default_one() {
    assert_eq!(parse("http://127.0.0.1:8080/x").host, "127.0.0.1:8080");
    assert_eq!(parse("https://h.example:443/x").host, "h.example");
    assert_eq!(parse("http://h.example:80/x").host, "h.example");
    assert_eq!(parse("https://h.example:8443/x").host, "h.example:8443");
    assert_eq!(parse("http://[::1]:80/x").host, "[::1]");
    assert_eq!(parse("http://[::1]/x").host, "[::1]");
}

#[test]
fn refuses_urls_it_cannot_sign_and_names_them() {
    for url in [
        "examplebucket/test.txt",
        "ftp://h/x",
        "https:///x",
        "https://user@h/x",
        "https://h/a%zzb",
        "https://h/a%+1b",
        "https://h/x?a=%4",
    ] {
        let Err(err) = RequestTarget::parse(url) else {
            panic!("{url} was accepted");
        };
        assert!(err.to_string().contains(url), "{url}: {err}");
    }
}
