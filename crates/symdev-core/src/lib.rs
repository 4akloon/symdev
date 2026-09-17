mod error;
mod types;

pub use error::*;
pub use types::*;

#[cfg(test)]
extern crate self as symdev_core;

#[test]
fn not_implemented_error_displays_feature_and_milestone() {
    let err = symdev_core::Error::NotImplemented {
        feature: "symdev build",
        milestone: "M1",
    };
    assert_eq!(
        err.to_string(),
        "not implemented: 'symdev build' (unlocks at M1)"
    );
}

#[test]
fn remote_path_debug_and_display() {
    let p = symdev_core::RemotePath::new("/tmp/hello");
    assert!(format!("{p}").contains("/tmp/hello"));
    assert!(format!("{p:?}").contains("/tmp/hello"));
}
