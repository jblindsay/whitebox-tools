/*
This code is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Last Modified: 13/09/2018
License: MIT
*/

use super::n_minimizer::NMinimizer;
use std::cmp::Ordering;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::f64;

/// DistanceMetric is a simple enum specifying a method for measuring distances.
pub enum DistanceMetric {
    Euclidean,
    SquaredEuclidean,
}

////////////////////////////
// FixedRadiusSearchKey3D //
////////////////////////////

#[derive(Clone, Copy)]
struct FixedRadiusSearchEntry2D<T: Copy> {
    x: f64,
    y: f64,
    value: T,
}

/// A simple 2D hash-based fixed radius search data structure.
///
/// ## Example
///     let mut frs = FixedRadiusSearch2D::new(5.0, DistanceMetric::SquaredEuclidean);
///     frs.insert(45.3, 32.5, 1i32);
///     frs.insert(25.3, 65.5, 2i32);
///     frs.insert(42.3, 35.5, 3i32);
///     frs.insert(40.3, 31.5, 4i32);
///     frs.insert(24.3, 68.5, 5i32);
///
///     let s1 = frs.search(41.4, 31.4);
///     println!("{:?}", s1);
///
///     let s2 = frs.knn_search(22.4, 69.4, 2);
///     println!("{:?}", s2);
pub struct FixedRadiusSearch2D<T: Copy> {
    inv_r: f64,
    r_sqr: f64,
    hm: HashMap<[i32; 2], Vec<FixedRadiusSearchEntry2D<T>>>,
    size: usize,
    is_distance_squared: bool,
    dx: [i32; 25],
    dy: [i32; 25],
}

impl<T: Copy> FixedRadiusSearch2D<T> {
    /// Creates a new 2-D fixed-radius search structure with the specified radius, containing data of type T.
    pub fn new(radius: f64, metric: DistanceMetric) -> FixedRadiusSearch2D<T> {
        let map = HashMap::new();
        let sqr_dist = match metric {
            DistanceMetric::Euclidean => false,
            DistanceMetric::SquaredEuclidean => true,
        };
        FixedRadiusSearch2D {
            // r: radius * 0.5f64,
            inv_r: 1f64 / (radius * 0.5f64),
            r_sqr: radius * radius,
            hm: map,
            size: 0usize,
            is_distance_squared: sqr_dist,
            dx: [
                -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2,
            ],
            dy: [
                -2, -2, -2, -2, -2, -1, -1, -1, -1, -1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2,
            ],
        }
    }

    /// Inserts a point (x, y, value).
    pub fn insert(&mut self, x: f64, y: f64, value: T) {
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
        self.size += 1;
    }

    /// Performs a fixed-radius search operation on point (x, y), returning a vector of data (type T), distance tuples.
    pub fn search(&self, x: f64, y: f64) -> Vec<(T, f64)> {
        let i = (x * self.inv_r).floor() as i32;
        let j = (y * self.inv_r).floor() as i32;

        let mut num_points = 0usize;
        for k in 0..25 {
            if let Some(vals) = self.hm.get(&[i + self.dx[k], j + self.dy[k]]) {
                num_points += vals.len();
            }
        }

        let mut ret = Vec::with_capacity(num_points);
        let mut dist: f64;
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

    /// Performs an approximate k-nearest neighbours search on point (x, y, z), returning a vector of
    /// data (type T), distance tuples.
    pub fn knn_search(&self, x: f64, y: f64, num_neighbours: usize) -> Vec<(T, f64)> {
        if num_neighbours == 0 {
            // do a regular radius search
            return self.search(x, y);
        }
        let neighbours = if num_neighbours < self.size {
            num_neighbours
        } else {
            self.size
        };

        let i = (x * self.inv_r).floor() as i32;
        let j = (y * self.inv_r).floor() as i32;

        let mut dist: f64;
        let mut lows = NMinimizer::new(neighbours);

        // first start with the centre bin
        for m in -1i32..=1i32 {
            for n in -1i32..=1i32 {
                if let Some(vals) = self.hm.get(&[i + m, j + n]) {
                    for val in vals {
                        // calculate the squared distance to (x, y)
                        dist = (x - val.x) * (x - val.x) + (y - val.y) * (y - val.y);

                        lows.insert(MinDistValue {
                            value: val.value,
                            dist: dist,
                        });
                    }
                }
            }
        }

        // for k in 0..25 {
        //     if let Some(vals) = self.hm.get(&[i + self.dx[k], j + self.dy[k]]) {
        //         for val in vals {
        //             // calculate the squared distance to (x, y)
        //             dist = (x - val.x) * (x - val.x) + (y - val.y) * (y - val.y);

        //             lows.insert(MinDistValue {
        //                 value: val.value,
        //                 dist: dist,
        //             });
        //         }
        //     }
        // }

        let mut shell = 2i32;
        while lows.size() < neighbours {
            for m in -shell..=shell {
                for n in -shell..=shell {
                    if m.abs() == shell || n.abs() == shell {
                        // The line above is hacky. I need a better method for this.
                        if let Some(vals) = self.hm.get(&[i + m, j + n]) {
                            for val in vals {
                                // calculate the squared distance to (x, y)
                                dist = (x - val.x) * (x - val.x) + (y - val.y) * (y - val.y);

                                lows.insert(MinDistValue {
                                    value: val.value,
                                    dist: dist,
                                });
                            }
                        }
                    }
                }
            }

            shell += 1;
        }

        let mut ret = Vec::with_capacity(neighbours);
        let mut minima: MinDistValue<T>;
        for i in 0..lows.size() {
            minima = lows.get(i).unwrap();
            if self.is_distance_squared {
                ret.push((minima.value, minima.dist));
            } else {
                ret.push((minima.value, minima.dist.sqrt()));
            }
        }

        ret
    }

    pub fn set_distance_metric(&mut self, metric: DistanceMetric) {
        match metric {
            DistanceMetric::Euclidean => self.is_distance_squared = false,
            DistanceMetric::SquaredEuclidean => self.is_distance_squared = true,
        }
    }

    pub fn get_distance_metric(&mut self) -> DistanceMetric {
        match self.is_distance_squared {
            false => DistanceMetric::Euclidean,
            true => DistanceMetric::SquaredEuclidean,
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

////////////////////////////
// FixedRadiusSearchKey3D //
////////////////////////////

#[derive(Clone, Copy)]
struct FixedRadiusSearchEntry3D<T: Copy> {
    x: f64,
    y: f64,
    z: f64,
    value: T,
}

/// A simple 3D hash-based fixed radius search data structure.
///
/// ## Example
///     let mut frs = FixedRadiusSearch3D::new(5.0, DistanceMetric::SquaredEuclidean);
///     frs.insert(45.3, 32.5, 6.1, 1i32);
///     frs.insert(25.3, 65.5, 21.5, 2i32);
///     frs.insert(42.3, 35.5, 43.9, 3i32);
///     frs.insert(40.3, 31.5, 3.6, 4i32);
///     frs.insert(24.3, 68.5, 12.4, 5i32);
///
///     let s1 = frs.search(41.4, 31.4, 12.3);
///     println!("{:?}", s1);
///
///     let s2 = frs.knn_search(22.4, 69.4, 10.5, 2);
///     println!("{:?}", s2);
pub struct FixedRadiusSearch3D<T: Copy> {
    inv_r: f64,
    r_sqr: f64,
    hm: HashMap<[i32; 3], Vec<FixedRadiusSearchEntry3D<T>>>,
    size: usize,
    is_distance_squared: bool,
}

impl<T: Copy> FixedRadiusSearch3D<T> {
    /// Creates a new 3-D fixed-radius search structure with the specified radius, containing data of type T.
    pub fn new(radius: f64, metric: DistanceMetric) -> FixedRadiusSearch3D<T> {
        let map = HashMap::new();
        let sqr_dist = match metric {
            DistanceMetric::Euclidean => false,
            DistanceMetric::SquaredEuclidean => true,
        };
        FixedRadiusSearch3D {
            // r: radius,
            inv_r: 1f64 / radius,
            r_sqr: radius * radius,
            hm: map,
            size: 0usize,
            is_distance_squared: sqr_dist,
        }
    }

    /// Inserts a point (x, y, z, value).
    pub fn insert(&mut self, x: f64, y: f64, z: f64, value: T) {
        let val = match self.hm.entry([
            (x * self.inv_r).floor() as i32,
            (y * self.inv_r).floor() as i32,
            (z * self.inv_r).floor() as i32,
        ]) {
            Vacant(entry) => entry.insert(vec![]),
            Occupied(entry) => entry.into_mut(),
        };
        val.push(FixedRadiusSearchEntry3D {
            x: x,
            y: y,
            z: z,
            value: value,
        });

        self.size += 1;
    }

    /// Performs a fixed-radius search operation on point (x, y, z), returning a vector of
    /// data (type T), distance tuples.
    pub fn search(&self, x: f64, y: f64, z: f64) -> Vec<(T, f64)> {
        let mut ret = vec![];
        let i = (x * self.inv_r).floor() as i32;
        let j = (y * self.inv_r).floor() as i32;
        let k = (z * self.inv_r).floor() as i32;

        for m in -1..2 {
            for n in -1..2 {
                for p in -1..2 {
                    if let Some(vals) = self.hm.get(&[i + m, j + n, k + p]) {
                        for val in vals {
                            // calculate the squared distance to (x,y, z)
                            let dist = (x - val.x) * (x - val.x)
                                + (y - val.y) * (y - val.y)
                                + (z - val.z) * (z - val.z);
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
            }
        }

        ret
    }

    /// Performs an approximate k-nearest neighbours search on point (x, y, z), returning a vector of
    /// data (type T), distance tuples.
    pub fn knn_search(&self, x: f64, y: f64, z: f64, num_neighbours: usize) -> Vec<(T, f64)> {
        if num_neighbours == 0 {
            // do a regular radius search
            return self.search(x, y, z);
        }
        let neighbours = if num_neighbours < self.size {
            num_neighbours
        } else {
            self.size
        };

        let i = (x * self.inv_r).floor() as i32;
        let j = (y * self.inv_r).floor() as i32;
        let k = (z * self.inv_r).floor() as i32;

        let mut dist: f64;
        let mut lows = NMinimizer::new(neighbours);

        // first start with the centre bin
        for m in -1i32..=1i32 {
            for n in -1i32..=1i32 {
                for p in -1i32..=1i32 {
                    if let Some(vals) = self.hm.get(&[i + m, j + n, k + p]) {
                        for val in vals {
                            // calculate the squared distance to (x, y, z)
                            dist = (x - val.x) * (x - val.x)
                                + (y - val.y) * (y - val.y)
                                + (z - val.z) * (z - val.z);

                            lows.insert(MinDistValue {
                                value: val.value,
                                dist: dist,
                            });
                        }
                    }
                }
            }
        }

        let mut shell = 2i32;
        while lows.size() < neighbours {
            for m in -shell..=shell {
                for n in -shell..=shell {
                    for p in -shell..=shell {
                        if m.abs() == shell || n.abs() == shell || p.abs() == shell {
                            // The line above is hacky. I need a better method for this.
                            if let Some(vals) = self.hm.get(&[i + m, j + n, k + p]) {
                                for val in vals {
                                    // calculate the squared distance to (x, y, z)
                                    dist = (x - val.x) * (x - val.x)
                                        + (y - val.y) * (y - val.y)
                                        + (z - val.z) * (z - val.z);

                                    lows.insert(MinDistValue {
                                        value: val.value,
                                        dist: dist,
                                    });
                                }
                            }
                        }
                    }
                }
            }

            shell += 1;
        }

        let mut ret = Vec::with_capacity(neighbours);
        let mut minima: MinDistValue<T>;
        for i in 0..lows.size() {
            minima = lows.get(i).unwrap();
            if self.is_distance_squared {
                ret.push((minima.value, minima.dist));
            } else {
                ret.push((minima.value, minima.dist.sqrt()));
            }
        }

        ret
    }

    pub fn set_distance_metric(&mut self, metric: DistanceMetric) {
        match metric {
            DistanceMetric::Euclidean => self.is_distance_squared = false,
            DistanceMetric::SquaredEuclidean => self.is_distance_squared = true,
        }
    }

    pub fn get_distance_metric(&mut self) -> DistanceMetric {
        match self.is_distance_squared {
            false => DistanceMetric::Euclidean,
            true => DistanceMetric::SquaredEuclidean,
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

/// MinDistValue is used for the NMinimizer in KNN searching.
#[derive(Clone, Copy)]
struct MinDistValue<T: Copy> {
    value: T,
    dist: f64,
}

impl<T: Copy> PartialEq for MinDistValue<T> {
    fn eq(&self, other: &MinDistValue<T>) -> bool {
        self.dist == other.dist
    }
}

impl<T: Copy> Eq for MinDistValue<T> {}

impl<T: Copy> PartialOrd for MinDistValue<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.dist.partial_cmp(&other.dist)
    }
}

#[cfg(test)]
mod test {
    use super::{DistanceMetric, FixedRadiusSearch2D, FixedRadiusSearch3D};

    #[test]
    fn test_frs2d_insert_and_search() {
        let mut frs = FixedRadiusSearch2D::new(0.75, DistanceMetric::Euclidean);
        frs.insert(40f64, 32f64, 1i32);
        frs.insert(25f64, 65f64, 2i32);
        frs.insert(42f64, 35f64, 3i32);
        frs.insert(40f64, 31f64, 4i32);
        frs.insert(24f64, 68f64, 5i32);

        let s1 = frs.search(40.5, 31.5);
        assert_eq!(
            s1,
            vec![(4i32, 0.7071067811865476f64), (1i32, 0.7071067811865476f64)]
        );

        frs.set_distance_metric(DistanceMetric::SquaredEuclidean);
        let s2 = frs.search(40.5, 31.5);
        assert_eq!(s2, vec![(4i32, 0.5f64), (1i32, 0.5f64)]);
    }

    #[test]
    fn test_frs2d_insert_and_knn_search() {
        let mut frs = FixedRadiusSearch2D::new(0.75, DistanceMetric::Euclidean);
        frs.insert(40f64, 32f64, 1i32);
        frs.insert(25f64, 65f64, 2i32);
        frs.insert(42f64, 35f64, 3i32);
        frs.insert(40f64, 31f64, 4i32);
        frs.insert(24f64, 68f64, 5i32);

        let s1 = frs.knn_search(40.5, 31.5, 3);
        assert_eq!(
            s1,
            vec![
                (4i32, 0.7071067811865476f64),
                (1i32, 0.7071067811865476f64),
                (3i32, 3.8078865529319543f64),
            ]
        );

        frs.set_distance_metric(DistanceMetric::SquaredEuclidean);
        let s2 = frs.knn_search(40.5, 31.5, 3);
        assert_eq!(s2, vec![(4i32, 0.5f64), (1i32, 0.5f64), (3i32, 14.5f64)]);
    }

    #[test]
    fn test_frs3d_insert_and_search() {
        let mut frs = FixedRadiusSearch3D::new(0.75, DistanceMetric::Euclidean);
        frs.insert(40f64, 32f64, 1f64, 1i32);
        frs.insert(25f64, 65f64, 5f64, 2i32);
        frs.insert(42f64, 35f64, 1f64, 3i32);
        frs.insert(40f64, 31f64, 1f64, 4i32);
        frs.insert(24f64, 68f64, 5f64, 5i32);

        let s1 = frs.search(40.5, 31.5, 1f64);
        assert_eq!(
            s1,
            vec![(4i32, 0.7071067811865476f64), (1i32, 0.7071067811865476f64)]
        );

        frs.set_distance_metric(DistanceMetric::SquaredEuclidean);
        let s2 = frs.search(40.5, 31.5, 1f64);
        assert_eq!(s2, vec![(4i32, 0.5f64), (1i32, 0.5f64)]);
    }

    #[test]
    fn test_frs3d_insert_and_knn_search() {
        let mut frs = FixedRadiusSearch3D::new(0.75, DistanceMetric::Euclidean);
        frs.insert(40f64, 32f64, 1f64, 1i32);
        frs.insert(25f64, 65f64, 5f64, 2i32);
        frs.insert(42f64, 35f64, 1f64, 3i32);
        frs.insert(40f64, 31f64, 1f64, 4i32);
        frs.insert(24f64, 68f64, 5f64, 5i32);

        let s1 = frs.knn_search(40.5, 31.5, 1f64, 3);
        assert_eq!(
            s1,
            vec![
                (4i32, 0.7071067811865476f64),
                (1i32, 0.7071067811865476f64),
                (3i32, 3.8078865529319543f64),
            ]
        );

        frs.set_distance_metric(DistanceMetric::SquaredEuclidean);
        let s2 = frs.knn_search(40.5, 31.5, 1f64, 3);
        assert_eq!(s2, vec![(4i32, 0.5f64), (1i32, 0.5f64), (3i32, 14.5f64)]);
    }
}
