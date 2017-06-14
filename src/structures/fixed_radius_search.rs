///////////////////////////////////////
// A 2D hash-based fixed radius search
///////////////////////////////////////

use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};

#[derive(Clone, PartialEq, Eq, Hash)]
struct FixedRadiusSearchKey2D {
    col: isize,
    row: isize,
}

#[derive(Clone, Copy)]
struct FixedRadiusSearchEntry2D {
    x: f64,
    y: f64,
    // dist: f64,
    index: usize,
}

pub struct FixedRadiusSearch2D<T: Copy> {
    r: f64,
    r_sqr: f64,
    hm: HashMap<FixedRadiusSearchKey2D, Vec<FixedRadiusSearchEntry2D>>,
    values: Vec<T>,
    length: usize,
}

impl<T: Copy> FixedRadiusSearch2D<T> {
    pub fn new(radius: f64) -> FixedRadiusSearch2D<T> {
        let map = HashMap::new();
        let values: Vec<T> = vec![];
        FixedRadiusSearch2D {
            r: radius,
            r_sqr: radius*radius,
            hm: map,
            values: values,
            length: 0usize,
        }
    }

    pub fn insert(&mut self, x: f64, y: f64, value: T) {
        let k = FixedRadiusSearchKey2D { col: (x / self.r).floor() as isize, row: (y / self.r).floor() as isize };
        let val = match self.hm.entry(k) {
           Vacant(entry) => entry.insert(vec![]),
           Occupied(entry) => entry.into_mut(),
        };
        val.push(FixedRadiusSearchEntry2D { x: x, y: y, index: self.length}); //, dist: -1f64});
        self.values.push(value);
        self.length += 1;
    }

    pub fn search(&self, x: f64, y: f64) -> Vec<(T, f64)> {
        let mut ret = vec![];
        let i = (x / self.r).floor() as isize;
        let j = (y / self.r).floor() as isize;

        for m in -1..2 {
            for n in -1..2 {
                if let Some(vals) = self.hm.get(&FixedRadiusSearchKey2D{ col: i+m, row: j+n }) {
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


////////////////////////////////////////
// A 3D hash-based fixed radius search
////////////////////////////////////////
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
    // dist: f64,
    index: usize,
}

pub struct FixedRadiusSearch3D<T: Copy> {
    r: f64,
    r_sqr: f64,
    hm: HashMap<FixedRadiusSearchKey3D, Vec<FixedRadiusSearchEntry3D>>,
    values: Vec<T>,
    length: usize,
}

impl<T: Copy> FixedRadiusSearch3D<T> {
    pub fn new(radius: f64) -> FixedRadiusSearch3D<T> {
        let map = HashMap::new();
        let values: Vec<T> = vec![];
        FixedRadiusSearch3D {
            r: radius,
            r_sqr: radius*radius,
            hm: map,
            values: values,
            length: 0usize,
        }
    }

    pub fn insert(&mut self, x: f64, y: f64, z: f64, value: T) {
        let k = FixedRadiusSearchKey3D { 
            col: (x / self.r).floor() as isize, 
            row: (y / self.r).floor() as isize,
            layer: (z / self.r).floor() as isize
        };
        let val = match self.hm.entry(k) {
           Vacant(entry) => entry.insert(vec![]),
           Occupied(entry) => entry.into_mut(),
        };
        val.push(FixedRadiusSearchEntry3D { x: x, y: y, z: z, index: self.length}); //, dist: -1f64});
        self.values.push(value);
        self.length += 1;
    }

    pub fn search(&self, x: f64, y: f64, z: f64) -> Vec<(T, f64)> {
        let mut ret = vec![];
        let i = (x / self.r).floor() as isize;
        let j = (y / self.r).floor() as isize;
        let k = (z / self.r).floor() as isize;

        for m in -1..2 {
            for n in -1..2 {
                for p in -1..2 {
                    if let Some(vals) = self.hm.get(&FixedRadiusSearchKey3D{ col: i+m, row: j+n, layer: k+p }) {
                        for val in vals {
                            // calculate the squared distance to (x,y, z)
                            let dist = (x - val.x)*(x - val.x) + (y - val.y)*(y - val.y) + (z - val.z)*(z - val.z);
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
