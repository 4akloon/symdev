use super::*;

const PASSING: &str = r#"{"schema":1,"app":"files","uid3":"0xe0000685","passed":2,"failed":0,
    "cases":[{"name":"write","ok":true},{"name":"read back","ok":true}]}"#;

const FAILING: &str = r#"{"schema":1,"app":"files","uid3":"0xe0000685","passed":1,"failed":1,
    "cases":[{"name":"write","ok":true},
             {"name":"read back","ok":false,"detail":"UnexpectedEof (KErrEof (-25))"}]}"#;

#[test]
fn a_passing_report_passes() {
    let report = TestReport::parse(PASSING).unwrap();
    assert_eq!(report.app, "files");
    assert_eq!((report.passed(), report.failed()), (2, 0));
    assert!(report.is_pass());
}

#[test]
fn a_failing_report_fails_and_keeps_the_detail() {
    let report = TestReport::parse(FAILING).unwrap();
    assert_eq!((report.passed(), report.failed()), (1, 1));
    assert!(!report.is_pass());
    assert_eq!(report.cases[1].detail, "UnexpectedEof (KErrEof (-25))");
}

#[test]
fn a_report_with_no_cases_is_not_a_pass() {
    let report = TestReport::parse(
        r#"{"schema":1,"app":"x","uid3":"0x1","passed":0,"failed":0,"cases":[]}"#,
    )
    .unwrap();
    assert!(
        !report.is_pass(),
        "an empty report must not count as a pass"
    );
}

#[test]
fn a_file_this_symdev_does_not_understand_is_an_error() {
    for text in [
        r#"{"schema":2,"app":"x","uid3":"0x1","cases":[]}"#,
        r#"{"app":"x","uid3":"0x1","cases":[]}"#,
        r#"{"schema":1,"uid3":"0x1","cases":[]}"#,
        r#"{"schema":1,"app":"x","uid3":"0x1"}"#,
        r#"{"schema":1,"app":"x","uid3":"0x1","cases":[{"name":"a"}]}"#,
    ] {
        assert!(TestReport::parse(text).is_err(), "accepted {text}");
    }
}

#[test]
fn the_result_file_is_where_the_device_put_it() {
    let data = EmulatorData::at(Path::new("/home/u/.local/share/EKA2L1"));
    assert_eq!(
        data.result_file(0xe000_0685),
        Path::new("/home/u/.local/share/EKA2L1/data/drives/e/symdev/results/e0000685.json")
    );
}

#[test]
fn waiting_for_a_report_that_never_comes_is_an_error() {
    let dir = std::env::temp_dir().join(format!("symdev-results-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let missing = dir.join("nothing.json");
    let err = await_report(&missing, Duration::from_millis(300))
        .unwrap_err()
        .to_string();
    assert!(err.contains("never wrote one"), "{err}");
}
