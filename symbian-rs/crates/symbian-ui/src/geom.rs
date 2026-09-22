//! `Point`, `Rect` and `Rgb`: the value types a `draw` writes in.
//!
//! Coordinates are **window-relative pixels**: the view's own area always starts at
//! `(0, 0)` and is `Rect::size(w, h)`, because drawing through a `CWindowGc` is
//! window-relative and the shim hands the area over with its origin already zeroed.
//! There is no second coordinate system to get wrong.
#![forbid(unsafe_code)]

use crate::abi::RawRect;

/// A pixel position inside the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// A rectangle as a position and a size, which is what `TRect(TPoint, TSize)` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// A rectangle at the view's origin.
    pub const fn size(width: i32, height: i32) -> Self {
        Self::new(0, 0, width, height)
    }

    pub const fn top_left(&self) -> Point {
        Point::new(self.x, self.y)
    }

    pub(crate) const fn raw(&self) -> RawRect {
        RawRect {
            x: self.x,
            y: self.y,
            w: self.width,
            h: self.height,
        }
    }
}

/// A colour as 8 bits each of red, green and blue.
///
/// It crosses to C++ as `0x00RRGGBB` and the shim unpacks it into
/// `TRgb(red, green, blue)`, so nothing here depends on `TRgb`'s internal word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(u32);

impl Rgb {
    pub const BLACK: Self = Self::new(0, 0, 0);
    pub const WHITE: Self = Self::new(255, 255, 255);
    pub const RED: Self = Self::new(255, 0, 0);
    pub const GREEN: Self = Self::new(0, 255, 0);
    pub const BLUE: Self = Self::new(0, 0, 255);
    pub const GRAY: Self = Self::new(128, 128, 128);

    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self(((red as u32) << 16) | ((green as u32) << 8) | blue as u32)
    }

    pub const fn red(self) -> u8 {
        (self.0 >> 16) as u8
    }

    pub const fn green(self) -> u8 {
        (self.0 >> 8) as u8
    }

    pub const fn blue(self) -> u8 {
        self.0 as u8
    }

    pub(crate) const fn bits(self) -> u32 {
        self.0
    }
}
