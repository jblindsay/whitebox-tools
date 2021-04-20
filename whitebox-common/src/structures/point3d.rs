/// A 3-D point, with x, y, and z fields.
#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct Point3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3D {
    /// Creates a new Point3D,
    pub fn new(x: f64, y: f64, z: f64) -> Point3D {
        Point3D {x: x, y: y, z: z}
    }
}