#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum MinutiaKind {
    RidgeEnding = 1, // 1 = ridge ending
    Bifurcation = 2, // 0 = bifurcation in mindtct C; mapped here to 2
}

#[derive(uniffi::Record)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

/// Plain minutia snapshot for FFI (no Object handles / clone_pointer).
/// Prefer this over `Minutiae.get()` → `Minutia` Object accessors in Python.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct MinutiaView {
    pub x: i32,
    pub y: i32,
    /// Degrees from +X axis (east), same as `Minutia::angle()`.
    pub angle: f64,
    pub reliability: f64,
    pub kind: MinutiaKind,
}

/// A lightweight, fully-safe copy of one minutia.
/// (Add more fields if you need them.)
#[derive(Debug, Clone, uniffi::Object)]
pub struct Minutia {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) direction: i32,
    pub(crate) reliability: f64, // 0.0 to 1.0
    pub(crate) kind: MinutiaKind,
}

impl Minutia {
    pub(crate) fn to_view(&self) -> MinutiaView {
        MinutiaView {
            x: self.x,
            y: self.y,
            angle: self.angle(),
            reliability: self.reliability,
            kind: self.kind.clone(),
        }
    }
}

#[uniffi::export]
impl Minutia {
    /// Get the angle in degrees (0-360) for this minutia starting from the X axis (east).
    pub fn angle(&self) -> f64 {
        let angle = 90.0 - self.direction as f64 * 11.25;
        (angle % 360.0 + 360.0) % 360.0 // Ensures result is in [0, 360)
    }

    pub fn x(&self) -> i32 {
        self.x
    }

    pub fn y(&self) -> i32 {
        self.y
    }

    /// Get the position as a tuple (x, y).
    pub fn position(&self) -> Position {
        Position {
            x: self.x,
            y: self.y,
        }
    }

    /// Get the reliability score (0.0 to 1.0).
    pub fn reliability(&self) -> f64 {
        self.reliability
    }

    /// Get the kind of minutia (ridge ending or bifurcation).
    pub fn kind(&self) -> MinutiaKind {
        self.kind.clone()
    }
}
