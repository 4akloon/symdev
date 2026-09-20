use super::*;

#[test]
fn meta_code_matches_spec_table() {
    let codes = E32Deflate::canonical(&E32Deflate::META_LENGTHS);
    let show = |m: usize| {
        let l = E32Deflate::META_LENGTHS[m];
        format!("{:0width$b}", codes[m], width = l as usize)
    };
    assert_eq!(show(0), "00");
    assert_eq!(show(1), "100");
    assert_eq!(show(2), "01");
    assert_eq!(show(14), "11111100");
    assert_eq!(show(21), "11111111111100");
    assert_eq!(show(28), "1111111111111111");
}

#[test]
fn length_and_distance_buckets_match_spec_tables() {
    assert_eq!(E32Deflate::bucket(0), (0, 0, 0));
    assert_eq!(E32Deflate::bucket(8), (8, 1, 0)); // length 11 → sym 264
    assert_eq!(E32Deflate::bucket(255), (27, 5, 31)); // length 258 → sym 283
    assert_eq!(E32Deflate::bucket(4095), (43, 9, 511)); // distance 4096 → sym 43
}

#[test]
fn bijective_runs_match_spec_examples() {
    let cases: [(u32, &[usize]); 7] = [
        (1, &[0]),
        (2, &[1]),
        (3, &[0, 0]),
        (4, &[0, 1]),
        (5, &[1, 0]),
        (6, &[1, 1]),
        (7, &[0, 0, 0]),
    ];
    for (run, want) in cases {
        let got = std::cell::RefCell::new(Vec::new());
        E32Deflate::write_run(&mut BitWriter::default(), run, &|_, m| {
            got.borrow_mut().push(m)
        });
        assert_eq!(got.into_inner(), want, "run {run}");
    }
}

#[test]
fn round_trips_edge_inputs() {
    let mut long = vec![7u8; 5000];
    long.extend((0..3000u32).map(|i| (i * 31 % 251) as u8));
    long.extend_from_within(100..4200);
    for body in [long, b"abcabcabcabcabcabcabcabc".repeat(40)] {
        let stream = E32Deflate::compress(&body).unwrap();
        assert_eq!(E32Deflate::decompress(&stream, body.len()).unwrap(), body);
    }
}

#[test]
fn tiny_body_is_an_error_like_the_reference() {
    assert!(E32Deflate::compress(b"ab").is_err());
}
