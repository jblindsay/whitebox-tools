#[derive(Clone, Copy, Debug)]
pub struct BoundingBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64
}

impl BoundingBox {
    pub fn new(min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> BoundingBox {
        let (x1, x2) = if min_x < max_x {
            (min_x, max_x)
        } else {
            (max_x, min_x)
        };
        let (y1, y2) = if min_y < max_y {
            (min_y, max_y)
        } else {
            (max_y, min_y)
        };
        BoundingBox {
            min_x: x1,
            min_y: y1,
            max_x: x2,
            max_y: y2
        }
    }
    pub fn get_height(&self) -> f64 {
        self.max_y - self.min_y
    }

    pub fn get_width(&self) -> f64 {
        self.max_x - self.min_x
    }

    pub fn overlaps(&self, other: BoundingBox) -> bool {
        if self.max_y < other.min_y || self.max_x < other.min_x
                || self.min_y > other.max_y || self.min_x > other.max_x {
            return false;
        }
        true
    }

    pub fn intersects_edge_of(&self, other: BoundingBox) -> bool {
        let mut one_inside_found = false;
        let mut one_outside_found = false;
        // at least one of the coordinates has to be within and at least 
        // one of them has to be outside
        for a in 0..4 {
            let (x, y) = match a {
            0 => (self.min_x, self.max_y),
            1 => (self.min_x, self.min_y),
            2 => (self.max_x, self.max_y),
            _ => (self.max_x, self.min_y),
            };
            if !one_inside_found {
                if y <= other.max_y && y >= other.min_y
                        && x <= other.max_x && x >= other.min_x {
                    one_inside_found = true;
                }
            }
            if !one_outside_found {
                if !(y <= other.max_y && y >= other.min_y)
                        || !(x <= other.max_x && x >= other.min_x) {
                    one_outside_found = true;
                }
            }
            if one_inside_found && one_outside_found {
                return true;
            }
        }
        false
    }
    
    pub fn entirely_contained_within(&self, other: BoundingBox) -> bool {
        if self.max_y < other.max_y && self.max_x < other.max_x
                && self.min_y > other.min_y && self.min_x > other.min_x {
            return true;
        }
        false
    }
    
    
    pub fn within(&self, other: BoundingBox) -> bool {
        if self.max_y <= other.max_y && self.max_x <= other.max_x
                && self.min_y >= other.min_y  && self.min_x >= other.min_x {
            return true;
        }
        false
    }
    
    pub fn entirely_contains(&self, other: BoundingBox) -> bool {
        if other.max_y < self.max_y && other.max_x < self.max_x
                && other.min_y > self.min_y && other.min_x > self.min_x {
            return true;
        }
        false
    }
    
    pub fn contains(&self, other: BoundingBox) -> bool {
        other.max_y <= self.max_y && other.max_x <= self.max_x
            && other.min_y >= self.min_y && other.min_x >= self.min_x
    }
    
    pub fn intersect(&self, other: BoundingBox) -> BoundingBox {
        let max_y = if self.max_y <= other.max_y { self.max_y } else { other.max_y };
        let max_x = if self.max_x <= other.max_x { self.max_x } else { other.max_x };
        let min_y = if self.min_y >= other.min_y { self.min_y } else { other.min_y };
        let min_x = if self.min_x >= other.min_x { self.min_x } else { other.min_x };
        BoundingBox { min_x: min_x, max_x: max_x, min_y: min_y, max_y: max_y}
    }
    
    pub fn is_point_in_box(&self, x: f64, y: f64) -> bool {
        !(self.max_y < y || self.max_x < x || self.min_y > y || self.min_x > x)
    }
    
    pub fn expand_to(&mut self, other: BoundingBox) {
        self.max_y = if self.max_y >= other.max_y { self.max_y } else { other.max_y }; 
        self.max_x = if self.max_x >= other.max_x { self.max_x } else { other.max_x }; 
        self.min_y = if self.min_y <= other.min_y { self.min_y } else { other.min_y }; 
        self.min_x = if self.min_x <= other.min_x { self.min_x } else { other.min_x }; 
    }
    
    pub fn contract_to(&mut self, other: BoundingBox) {
        self.max_y = if self.max_y <= other.max_y { self.max_y } else { other.max_y }; 
        self.max_x = if self.max_x <= other.max_x { self.max_x } else { other.max_x }; 
        self.min_y = if self.min_y >= other.min_y { self.min_y } else { other.min_y }; 
        self.min_x = if self.min_x >= other.min_x { self.min_x } else { other.min_x }; 
    }

    pub fn expand_by(&mut self, value: f64) {
        self.max_y += value; 
        self.max_x += value; 
        self.min_y -= value; 
        self.min_x -= value; 
    }

    pub fn contract_by(&mut self, value: f64) {
        self.max_y -= value; 
        self.max_x -= value; 
        self.min_y += value; 
        self.min_x += value; 
    }
    
}