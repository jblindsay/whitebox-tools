////////////////////////////////////
// A hash-based fixed radius search
////////////////////////////////////
extern crate num_cpus;
extern crate rayon;

use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};
// use std::thread;
// use std::sync::mpsc;

#[derive(Clone, PartialEq, Eq, Hash)]
struct FixedRadiusSearchKey {
    col: isize,
    row: isize,
}

#[derive(Clone, Copy)]
struct FixedRadiusSearchEntry {
    x: f64,
    y: f64,
    dist: f64,
    index: usize,
}

pub struct FixedRadiusSearch<T: Copy> {
    r: f64,
    r_sqr: f64,
    hm: HashMap<FixedRadiusSearchKey, Vec<FixedRadiusSearchEntry>>,
    values: Vec<T>,
    length: usize,
    num_cpus: usize,
    run_concurrently: bool,
}

impl<T: Copy> FixedRadiusSearch<T> {
    pub fn new(radius: f64) -> FixedRadiusSearch<T> {
        let map = HashMap::new();
        let values: Vec<T> = vec![];
        let num_cpus = num_cpus::get();
        FixedRadiusSearch {
            r: radius,
            r_sqr: radius*radius,
            hm: map,
            values: values,
            length: 0usize,
            num_cpus: num_cpus,
            run_concurrently: false
        }
    }

    pub fn insert(&mut self, x: f64, y: f64, value: T) {
        let k = FixedRadiusSearchKey { col: (x / self.r).floor() as isize, row: (y / self.r).floor() as isize };
        let val = match self.hm.entry(k) {
           Vacant(entry) => entry.insert(vec![]),
           Occupied(entry) => entry.into_mut(),
        };
        val.push(FixedRadiusSearchEntry { x: x, y: y, index: self.length, dist: -1f64});
        self.values.push(value);
        self.length += 1;

        // let mut added_value = false;
        // if let Some(vals) = self.hm.get_mut(&k) {
        //     self.values.push(value);
        //     vals.push(FixedRadiusSearchEntry { x: x, y: y, index: self.length, dist: -1f64 });
        //     added_value = true;
        // }
        // if !added_value {
        //     self.values.push(value);
        //     self.hm.insert(k, vec![FixedRadiusSearchEntry { x: x, y: y, index: self.length, dist: -1f64 }]);
        // }
        // self.length += 1;
    }

    pub fn search(&mut self, x: f64, y: f64) -> Vec<(T, f64)> {
        let mut ret = vec![];
        let i = (x / self.r).floor() as isize;
        let j = (y / self.r).floor() as isize;

        if !self.run_concurrently {
            for m in -1..2 {
                for n in -1..2 {
                    if let Some(vals) = self.hm.get(&FixedRadiusSearchKey{ col: i+m, row: j+n }) {
                        for val in vals {
                            // calculate the squared distance to (x,y)
                            let dist = (x - val.x)*(x - val.x) + (y - val.y)*(y - val.y);
                            if dist <= self.r_sqr {
                                ret.push((self.values[val.index], dist.sqrt()));
                            }
                        }
                    }
                }
            }
            if ret.len() > 5000 && self.num_cpus > 1 {
                self.run_concurrently = true;
            }
        } else {
            // let (tx, rx) = mpsc::channel();
            // for m in -1..2 {
            //     for n in -1..2 {
            //         let tx = tx.clone();
            //         if let Some(vals) = self.hm.get_mut(&FixedRadiusSearchKey{ col: i+m, row: j+n }) {
            //             let vals = vals.clone();
            //             let x = x.clone();
            //             let y = y.clone();
            //             let r_sqr = self.r_sqr.clone();
            //             let tx = tx.clone();
            //             thread::spawn(move || {
            //                 for val in vals {
            //                     // calculate the squared distance to (x,y)
            //                     let dist = (x - val.x)*(x - val.x) + (y - val.y)*(y - val.y);
            //                     if dist <= r_sqr {
            //                         less_than_threshold.push((val.index, dist.sqrt()));
            //                     } else {
            //                         less_than_threshold.push((val.index, -1f64));
            //                     }
            //                 }
            //                 tx.send(less_than_threshold).unwrap();
            //             });
            //         }
            //     }
            // }
            //
            // for _ in 0..10 {
            //     let data = rx.recv().unwrap();
            //     for d in data {
            //         if d.1 >= 0f64 {
            //             ret.push((self.values[d.0], d.1));
            //         }
            //     }
            // }

            // let mut points = vec![];
            for m in -1..2 {
                for n in -1..2 {
                    if let Some(vals) = self.hm.get_mut(&FixedRadiusSearchKey{ col: i+m, row: j+n }) {
                        // points.extend_from_slice(&vals[..]);
                        // points.extend(vals.iter().cloned());
                        if vals.len() >= 5000 {
                            calc_dist(&mut vals[..], &self.r_sqr, &x, &y);
                            for val in vals {
                                if val.dist >= 0f64 {
                                    ret.push((self.values[val.index], val.dist));
                                }
                            }
                        } else {
                            for val in vals {
                                // calculate the squared distance to (x,y)
                                let dist = (x - val.x)*(x - val.x) + (y - val.y)*(y - val.y);
                                if dist <= self.r_sqr {
                                    ret.push((self.values[val.index], dist.sqrt()));
                                }
                            }
                        }
                    }
                }
            }
            // if points.len() >= 5000 {
            //     calc_dist(&mut points[..], &self.r_sqr, &x, &y);
            //     for val in points {
            //         if val.dist >= 0f64 {
            //             ret.push((self.values[val.index], val.dist));
            //         }
            //     }
            // } else {
            //     for val in points {
            //         // calculate the squared distance to (x,y)
            //         let dist = (x - val.x)*(x - val.x) + (y - val.y)*(y - val.y);
            //         if dist <= self.r_sqr {
            //             ret.push((self.values[val.index], dist.sqrt()));
            //         }
            //     }
            // }

            if ret.len() < 2500 {
                self.run_concurrently = false;
            }
        }

        ret
    }
}

#[inline(always)]
fn calc_dist(slice: &mut [FixedRadiusSearchEntry], threshold: &f64, x: &f64, y: &f64) {
    if slice.len() < 1000 {
        for val in slice {
            let dist = (x - val.x)*(x - val.x) + (y - val.y)*(y - val.y);
            if dist <= *threshold {
                val.dist = dist.sqrt();
            } else {
                val.dist = -1f64;
            }
        }
    } else {
        let mid_point = slice.len() / 2;
        let (left, right) = slice.split_at_mut(mid_point);
        rayon::join(|| calc_dist(left, threshold, x, y), || calc_dist(right, threshold, x, y));
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
