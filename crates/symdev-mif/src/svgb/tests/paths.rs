//! Path data and transforms, checked against `svgtbinencode -v 3`
//! (experiment 59).

use super::{encode, hex};
use crate::{SvgElement, Svgb};

fn path(d: &str) -> Vec<u8> {
    encode(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\"><path d=\"{d}\"/></svg>"
    ))
}

fn body(bytes: &[u8]) -> Vec<u8> {
    // Strip the header, `<svg>`, its `e8 03`, the `<path>` token and the id.
    bytes[10..bytes.len() - 5].to_vec()
}

/// svgb-mif-spec.md §4.10: command count, commands, value count, values.
#[test]
fn commands_are_reduced_to_five() {
    assert_eq!(
        body(&path("M 1 2 L 3 4 Z")),
        hex("03 00 00 01 04 04 00 00 00 01 00 00 00 02 00 00 00 03 00 00 00 04 00")
    );
    // `H` and `V` become lines; relative commands become absolute.
    assert_eq!(
        body(&path("M0 0H5V6h1v2")),
        hex("05 00 00 01 01 01 01 0a 00 \
             00 00 00 00 00 00 00 00 00 00 05 00 00 00 00 00 \
             00 00 05 00 00 00 06 00 00 00 06 00 00 00 06 00 \
             00 00 06 00 00 00 08 00")
    );
}

/// A second coordinate pair after `M` stays a move, where SVG would make it
/// a line.
#[test]
fn a_repeated_move_stays_a_move() {
    assert_eq!(
        body(&path("M 1 2 3 4")),
        hex("02 00 00 00 04 00 00 00 01 00 00 00 02 00 00 00 03 00 00 00 04 00")
    );
}

/// One reflection point, updated by every command except `M` and `Z`, is
/// what `S` and `T` both use.
#[test]
fn smooth_curves_share_one_reflection_point() {
    // `S` after `C`: 2·(5,6) − (3,4) = (7,8).
    assert_eq!(
        body(&path("M0 0 C1 2 3 4 5 6 S 7 8 9 10")),
        hex("03 00 00 03 03 0e 00 \
             00 00 00 00 00 00 00 00 00 00 01 00 00 00 02 00 \
             00 00 03 00 00 00 04 00 00 00 05 00 00 00 06 00 \
             00 00 07 00 00 00 08 00 00 00 07 00 00 00 08 00 \
             00 00 09 00 00 00 0a 00")
    );
    // `S` right after `M` reflects the origin, not the current point.
    assert_eq!(
        body(&path("M5 5 S 1 2 3 4")),
        hex("02 00 00 03 08 00 00 00 05 00 00 00 05 00 \
             00 00 00 00 00 00 00 00 00 00 01 00 00 00 02 00 \
             00 00 03 00 00 00 04 00")
    );
    // A plain `L` leaves 2·new − old behind for the next `T`.
    assert_eq!(
        body(&path("M5 5 L 6 7 T 1 2")),
        hex("03 00 00 01 02 08 00 00 00 05 00 00 00 05 00 \
             00 00 06 00 00 00 07 00 00 00 07 00 00 00 09 00 \
             00 00 01 00 00 00 02 00")
    );
}

/// The arithmetic is 16.16 integer arithmetic: `84` then `-8.059` is
/// 4976870, one more than the exact 75.941 would truncate to.
#[test]
fn relative_coordinates_are_added_in_fixed_point() {
    assert_eq!(
        body(&path("M84 0 l-8.059 0")),
        hex("02 00 00 01 04 00 00 00 54 00 00 00 00 00 e6 f0 4b 00 00 00 00 00")
    );
}

/// Out-of-range path coordinates saturate instead of dropping the attribute.
#[test]
fn path_coordinates_saturate() {
    assert_eq!(
        body(&path("M 0 0 L 40000 1")),
        hex("02 00 00 01 04 00 00 00 00 00 00 00 00 00 ff ff ff 7f 00 00 01 00")
    );
}

/// `points` shares the payload; a polygon adds the close command.
#[test]
fn points_becomes_a_move_and_lines() {
    let poly = encode("<svg><polygon points=\"1,2 3,4 5,6\"/></svg>");
    assert_eq!(
        body(&poly),
        hex("04 00 00 01 01 04 06 00 00 00 01 00 00 00 02 00 \
             00 00 03 00 00 00 04 00 00 00 05 00 00 00 06 00")
    );
    assert_eq!(
        body(&encode("<svg><polyline points=\"\"/></svg>")),
        hex("00 00 00 00")
    );
}

/// svgb-mif-spec.md §4.11, refined by experiment 59: `matrix()` takes its
/// kind word from its values, the other functions from their own type.
#[test]
fn transform_matrix_and_kind() {
    let transform = |t: &str| body(&encode(&format!("<svg><rect transform=\"{t}\"/></svg>")));
    assert_eq!(
        transform("translate(1,2)"),
        hex("00 00 01 00 00 00 00 00 00 00 01 00 \
             00 00 00 00 00 00 01 00 00 00 02 00 01 00 00 00")
    );
    assert_eq!(
        transform("matrix(1,0,0,1,0,0)"),
        hex("00 00 01 00 00 00 00 00 00 00 00 00 \
             00 00 00 00 00 00 01 00 00 00 00 00 00 00 00 00")
    );
    assert_eq!(
        transform("matrix(1,0,0,1,5,0)"),
        hex("00 00 01 00 00 00 00 00 00 00 05 00 \
             00 00 00 00 00 00 01 00 00 00 00 00 03 00 00 00")
    );
    // The functions multiply left to right and their kinds are OR-ed.
    assert_eq!(
        transform("translate(1,2) translate(3,4) scale(2,2)"),
        hex("00 00 02 00 00 00 00 00 00 00 04 00 \
             00 00 00 00 00 00 02 00 00 00 06 00 03 00 00 00")
    );
}

/// Arcs, unknown commands and unseparated coordinates are refused rather
/// than reproduced: the tool empties the whole path for each of them.
#[test]
fn refuses_what_empties_the_path() {
    let bad = |d: &str| {
        Svgb::new(3)
            .unwrap()
            .encode(&SvgElement::parse(&format!("<svg><path d=\"{d}\"/></svg>")).unwrap())
            .is_err()
    };
    assert!(bad("M0 0 L1 1 A 1 2 3 0 1 4 5"));
    assert!(bad("M0 0 X 1 1"));
    assert!(bad("M0 0L0.5.5"));
    assert!(bad("M0 0L1e1 2"));
    assert!(bad("M0 0 Z 1 1"));
}
