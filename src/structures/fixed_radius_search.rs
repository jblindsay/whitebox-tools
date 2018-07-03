use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;

// #[derive(Clone, PartialEq, Eq, Hash)]
// struct FixedRadiusSearchKey2D {
//     col: isize,
//     row: isize,
// }

// #[derive(Clone, Copy)]
// struct FixedRadiusSearchEntry2D {
//     x: f64,
//     y: f64,
//     index: usize,
// }

#[derive(Clone, Copy)]
struct FixedRadiusSearchEntry2D<T: Copy> {
    x: f32,
    y: f32,
    value: T,
}

/// A simple 2D hash-based fixed radius search data structure.
///
/// ## Example
///     let mut frs: FixedRadiusSearch2D<i32> = FixedRadiusSearch2D::new(5.0);
///     frs.insert(45.3, 32.5, 1i32);
///     frs.insert(25.3, 65.5, 2i32);
///     frs.insert(42.3, 35.5, 3i32);
///     frs.insert(40.3, 31.5, 4i32);
///     frs.insert(24.3, 68.5, 5i32);
///
///     let s1 = frs.search(41.4, 31.4);
///     println!("{:?}", s1);
///
///     let s2 = frs.search(22.4, 69.4);
///     println!("{:?}", s2);
pub struct FixedRadiusSearch2D<T: Copy> {
    inv_r: f32,
    r_sqr: f32,
    hm: HashMap<[i32; 2], Vec<FixedRadiusSearchEntry2D<T>>>,
    length: usize,
    is_distance_squared: bool,
    dx: [i32; 25],
    dy: [i32; 25],
}

impl<T: Copy> FixedRadiusSearch2D<T> {
    /// Creates a new 2-D fixed-radius search structure with the specified radius, containing data of type T.
    pub fn new(radius: f32) -> FixedRadiusSearch2D<T> {
        let map = HashMap::new();
        FixedRadiusSearch2D {
            // r: radius * 0.5f32,
            inv_r: 1f32 / (radius * 0.5f32),
            r_sqr: radius * radius,
            hm: map,
            length: 0usize,
            is_distance_squared: false,
            dx: [
                -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1,
                2,
            ],
            dy: [
                -2, -2, -2, -2, -2, -1, -1, -1, -1, -1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 2, 2, 2, 2,
                2,
            ],
        }
    }

    /// Inserts a point (x, y, value).
    pub fn insert(&mut self, x: f32, y: f32, value: T) {
        let val = match self.hm.entry([
            (x * self.inv_r).floor() as i32,
            (y * self.inv_r).floor() as i32,
        ]) {
            Vacant(entry) => entry.insert(vec![]),
            Occupied(entry) => entry.into_mut(),
        };
        val.push(FixedRadiusSearchEntry2D {
            x: x,
            y: y,
            value: value,
        });
        self.length += 1;
    }

    /// Performs a fixed-radius search operation on point (x, y), returning a vector of data (type T), distance tuples.
    pub fn search(&self, x: f32, y: f32) -> Vec<(T, f32)> {
        let i = (x * self.inv_r).floor() as i32;
        let j = (y * self.inv_r).floor() as i32;

        let mut num_points = 0usize;
        for k in 0..25 {
            if let Some(vals) = self.hm.get(&[i + self.dx[k], j + self.dy[k]]) {
                num_points += vals.len();
            }
        }

        let mut ret = Vec::with_capacity(num_points);
        let mut dist: f32;
        for k in 0..25 {
            if let Some(vals) = self.hm.get(&[i + self.dx[k], j + self.dy[k]]) {
                for val in vals {
                    // calculate the squared distance to (x,y)
                    dist = (x - val.x) * (x - val.x) + (y - val.y) * (y - val.y);
                    if dist <= self.r_sqr {
                        if self.is_distance_squared {
                            ret.push((val.value, dist));
                        } else {
                            ret.push((val.value, dist.sqrt()));
                        }
                    }
                }
            }
        }

        ret
    }

    pub fn is_distance_squared(&mut self, value: bool) {
        self.is_distance_squared = value;
    }

    pub fn len(&self) -> usize {
        self.length
    }
}

// pub struct FixedRadiusSearch2D<T: Copy> {
//     r: f64,
//     r_sqr: f64,
//     hm: HashMap<FixedRadiusSearchKey2D, Vec<FixedRadiusSearchEntry2D>>,
//     values: Vec<T>,
//     length: usize,
//     is_distance_squared: bool,
// }

// impl<T: Copy> FixedRadiusSearch2D<T> {
//     /// Creates a new 2-D fixed-radius search structure with the specified radius, containing data of type T.
//     pub fn new(radius: f64) -> FixedRadiusSearch2D<T> {
//         let map = HashMap::new();
//         let values: Vec<T> = vec![];
//         FixedRadiusSearch2D {
//             r: radius,
//             r_sqr: radius * radius,
//             hm: map,
//             values: values,
//             length: 0usize,
//             is_distance_squared: false,
//         }
//     }

//     /// Inserts a point (x, y, value).
//     pub fn insert(&mut self, x: f64, y: f64, value: T) {
//         let k = FixedRadiusSearchKey2D {
//             col: (x / self.r).floor() as isize,
//             row: (y / self.r).floor() as isize,
//         };
//         let val = match self.hm.entry(k) {
//             Vacant(entry) => entry.insert(vec![]),
//             Occupied(entry) => entry.into_mut(),
//         };
//         val.push(FixedRadiusSearchEntry2D {
//             x: x,
//             y: y,
//             index: self.length,
//         });
//         self.values.push(value);
//         self.length += 1;
//     }

//     /// Performs a fixed-radius search operation on point (x, y), returning a vector of data (type T), distance tuples.
//     pub fn search(&self, x: f64, y: f64) -> Vec<(T, f64)> {
//         // let mut ret = vec![];
//         let i = (x / self.r).floor() as isize;
//         let j = (y / self.r).floor() as isize;

//         let mut num_points = 0usize;

//         for m in -1..2 {
//             for n in -1..2 {
//                 if let Some(vals) = self.hm.get(&FixedRadiusSearchKey2D {
//                     col: i + m,
//                     row: j + n,
//                 }) {
//                     num_points += vals.len();
//                 }
//             }
//         }

//         let mut ret = Vec::with_capacity(num_points);

//         for m in -1..2 {
//             for n in -1..2 {
//                 if let Some(vals) = self.hm.get(&FixedRadiusSearchKey2D {
//                     col: i + m,
//                     row: j + n,
//                 }) {
//                     for val in vals {
//                         // calculate the squared distance to (x,y)
//                         let dist = (x - val.x) * (x - val.x) + (y - val.y) * (y - val.y);
//                         if dist <= self.r_sqr {
//                             if self.is_distance_squared {
//                                 ret.push((self.values[val.index], dist));
//                             } else {
//                                 ret.push((self.values[val.index], dist.sqrt()));
//                             }
//                         }
//                     }
//                 }
//             }
//         }

//         ret
//     }

//     pub fn is_distance_squared(&mut self, value: bool) {
//         self.is_distance_squared = value;
//     }

//     pub fn len(&self) -> usize {
//         self.length
//     }
// }

#[derive(Clone, PartialEq, Eq, Hash)]
struct FixedRadiusSearchKey3D {
    col: isize,
    row: isize,
    layer: isize,
}

#[derive(Clone, Copy)]
struct FixedRadiusSearchEntry3D {
    x: f64,
    y: f64,
    z: f64,
    index: usize,
}

/// A simple 3D hash-based fixed radius search data structure.
pub struct FixedRadiusSearch3D<T: Copy> {
    r: f64,
    r_sqr: f64,
    hm: HashMap<FixedRadiusSearchKey3D, Vec<FixedRadiusSearchEntry3D>>,
    values: Vec<T>,
    length: usize,
}

impl<T: Copy> FixedRadiusSearch3D<T> {
    /// Creates a new 3-D fixed-radius search structure with the specified radius, containing data of type T.
    pub fn new(radius: f64) -> FixedRadiusSearch3D<T> {
        let map = HashMap::new();
        let values: Vec<T> = vec![];
        FixedRadiusSearch3D {
            r: radius,
            r_sqr: radius * radius,
            hm: map,
            values: values,
            length: 0usize,
        }
    }

    /// Inserts a point (x, y, z, value).
    pub fn insert(&mut self, x: f64, y: f64, z: f64, value: T) {
        let k = FixedRadiusSearchKey3D {
            col: (x / self.r).floor() as isize,
            row: (y / self.r).floor() as isize,
            layer: (z / self.r).floor() as isize,
        };
        let val = match self.hm.entry(k) {
            Vacant(entry) => entry.insert(vec![]),
            Occupied(entry) => entry.into_mut(),
        };
        val.push(FixedRadiusSearchEntry3D {
            x: x,
            y: y,
            z: z,
            index: self.length,
        }); //, dist: -1f64});
        self.values.push(value);
        self.length += 1;
    }

    /// Performs a fixed-radius search operation on point (x, y, z), returning a vector of data (type T), distance tuples.
    pub fn search(&self, x: f64, y: f64, z: f64) -> Vec<(T, f64)> {
        let mut ret = vec![];
        let i = (x / self.r).floor() as isize;
        let j = (y / self.r).floor() as isize;
        let k = (z / self.r).floor() as isize;

        for m in -1..2 {
            for n in -1..2 {
                for p in -1..2 {
                    if let Some(vals) = self.hm.get(&FixedRadiusSearchKey3D {
                        col: i + m,
                        row: j + n,
                        layer: k + p,
                    }) {
                        for val in vals {
                            // calculate the squared distance to (x,y, z)
                            let dist = (x - val.x) * (x - val.x)
                                + (y - val.y) * (y - val.y)
                                + (z - val.z) * (z - val.z);
                            if dist <= self.r_sqr {
                                ret.push((self.values[val.index], dist.sqrt()));
                            }
                        }
                    }
                }
            }
        }

        ret
    }
}

// #[test]
// pub fn main() {
//     let mut frs: FixedRadiusSearch<i32> = FixedRadiusSearch::new(5.0);
//     frs.insert(45.3, 32.5, 1i32);
//     frs.insert(25.3, 65.5, 2i32);
//     frs.insert(42.3, 35.5, 3i32);
//     frs.insert(40.3, 31.5, 4i32);
//     frs.insert(24.3, 68.5, 5i32);
//
//     let s1 = frs.search(41.4, 31.4);
//     println!("{:?}", s1);
//
//     let s2 = frs.search(22.4, 69.4);
//     println!("{:?}", s2);
// }

// #[derive(Clone, Copy)]
// struct ExperimentalFixedRadiusSearchEntry2D<T: Copy> {
//     x: f32,
//     y: f32,
//     value: T,
// }

// pub struct ExperimentalFixedRadiusSearch2D<T: Copy> {
//     inv_r: f32,
//     r_sqr: f32,
//     hm: HashMap<[i32; 2], Vec<ExperimentalFixedRadiusSearchEntry2D<T>>>,
//     length: usize,
//     is_distance_squared: bool,
//     dx: [i32; 25],
//     dy: [i32; 25],
// }

// impl<T: Copy> ExperimentalFixedRadiusSearch2D<T> {
//     /// Creates a new 2-D fixed-radius search structure with the specified radius, containing data of type T.
//     pub fn new(radius: f32) -> ExperimentalFixedRadiusSearch2D<T> {
//         let map = HashMap::new();
//         ExperimentalFixedRadiusSearch2D {
//             // r: radius * 0.5f32,
//             inv_r: 1f32 / (radius * 0.5f32),
//             r_sqr: radius * radius,
//             hm: map,
//             length: 0usize,
//             is_distance_squared: false,
//             dx: [
//                 -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1,
//                 2,
//             ],
//             dy: [
//                 -2, -2, -2, -2, -2, -1, -1, -1, -1, -1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 2, 2, 2, 2,
//                 2,
//             ],
//         }
//     }

//     /// Inserts a point (x, y, value).
//     pub fn insert(&mut self, x: f32, y: f32, value: T) {
//         let val = match self.hm.entry([
//             (x * self.inv_r).floor() as i32,
//             (y * self.inv_r).floor() as i32,
//         ]) {
//             Vacant(entry) => entry.insert(vec![]),
//             Occupied(entry) => entry.into_mut(),
//         };
//         val.push(ExperimentalFixedRadiusSearchEntry2D {
//             x: x,
//             y: y,
//             value: value,
//         });
//         self.length += 1;
//     }

//     /// Performs a fixed-radius search operation on point (x, y), returning a vector of data (type T), distance tuples.
//     pub fn search(&self, x: f32, y: f32) -> Vec<(T, f32)> {
//         let i = (x * self.inv_r).floor() as i32;
//         let j = (y * self.inv_r).floor() as i32;

//         let mut num_points = 0usize;
//         for k in 0..25 {
//             if let Some(vals) = self.hm.get(&[i + self.dx[k], j + self.dy[k]]) {
//                 num_points += vals.len();
//             }
//         }

//         let mut ret = Vec::with_capacity(num_points);
//         let mut dist: f32;
//         for k in 0..25 {
//             if let Some(vals) = self.hm.get(&[i + self.dx[k], j + self.dy[k]]) {
//                 for val in vals {
//                     // calculate the squared distance to (x,y)
//                     dist = (x - val.x) * (x - val.x) + (y - val.y) * (y - val.y);
//                     if dist <= self.r_sqr {
//                         if self.is_distance_squared {
//                             ret.push((val.value, dist));
//                         } else {
//                             ret.push((val.value, dist.sqrt()));
//                         }
//                     }
//                 }
//             }
//         }

//         ret
//     }

//     pub fn is_distance_squared(&mut self, value: bool) {
//         self.is_distance_squared = value;
//     }

//     pub fn len(&self) -> usize {
//         self.length
//     }
// }
