/*!
The following code has been modified from the original delaunator-rs project:

https://github.com/mourner/delaunator-rs

For a description of the data structure, including the halfedge connectivity, see:

https://mapbox.github.io/delaunator/


# Description
A very fast 2D [Delaunay Triangulation](https://en.wikipedia.org/wiki/Delaunay_triangulation) library for Rust.
A port of [Delaunator](https://github.com/mapbox/delaunator).


A triangle edge may be shared with another triangle. Instead of thinking about each edge A↔︎B, we will use two half-edges A→B and B→A. Having two half-edges is the key to everything this library provides.

Half-edges e are the indices into both of delaunator’s outputs:

    delaunay.triangles[e] returns the point id where the half-edge starts
    delaunay.halfedges[e] returns the opposite half-edge in the adjacent triangle, or -1 if there is no adjacent triangle

Triangle ids and half-edge ids are related.

    The half-edges of triangle t are 3*t, 3*t + 1, and 3*t + 2.
    The triangle of half-edge id e is floor(e/3

# Example

```rust
use delaunator::triangulate;
use structures::Point2D

let points = vec![
    Point2D { x: 0., y: 0. },
    Point2D { x: 1., y: 0. },
    Point2D { x: 1., y: 1. },
    Point2D { x: 0., y: 1. },
];

let result = triangulate(&points).expect("No triangulation exists.");
println!("{:?}", result.triangles); // [0, 2, 1, 0, 3, 2]
```
*/

use crate::structures::Point2D;
use std::collections::HashSet;
use std::f64;

/// Represents the area outside of the triangulation.
/// Halfedges on the convex hull (which don't have an adjacent halfedge)
/// will have this value.
pub const EMPTY: usize = usize::max_value();

/// A data structure used to perform Delaunay triangulation on
/// a set of input vector points. Connectivity between points,
/// triangles, and halfedges is as follows:
///
/// - edge → edges: next_halfedge, prevHalfedge, halfedges[]
/// - edge → points: triangles[]
/// - edge → triangle: triangle_of_edge
/// - triangle → edges: edges_of_triangle
/// - triangle → points: points_of_triangle
/// - triangle → triangles: triangles_adjacent_to_triangle
/// - point → incoming edges: edges_around_point
/// - point → outgoing edges: edges_around_point + halfedge[]
/// - point → points: edges_around_point + triangles[]
/// - point → triangles: edges_around_point + triangle_of_edge
pub struct Triangulation {
    /// A vector of point indices where each triple represents a Delaunay triangle.
    /// All triangles are directed counter-clockwise.
    pub triangles: Vec<usize>,

    /// A vector of adjacent halfedge indices that allows traversing the triangulation graph.
    ///
    /// `i`-th half-edge in the array corresponds to vertex `triangles[i]`
    /// the half-edge is coming from. `halfedges[i]` is the index of a twin half-edge
    /// in an adjacent triangle (or `EMPTY` for outer half-edges on the convex hull).
    pub halfedges: Vec<usize>,

    /// A vector of indices that reference points on the convex hull of the triangulation,
    /// counter-clockwise.
    pub hull: Vec<usize>,
}

impl Triangulation {
    /// Constructs a new *Triangulation*.
    fn new(n: usize) -> Self {
        let max_triangles = 2 * n - 5;
        Self {
            triangles: Vec::with_capacity(max_triangles * 3),
            halfedges: Vec::with_capacity(max_triangles * 3),
            hull: Vec::new(),
        }
    }

    /// The number of triangles in the triangulation.
    pub fn len(&self) -> usize {
        self.triangles.len() / 3
    }

    /// Next halfedge in a triangle.
    pub fn next_halfedge(&self, edge: usize) -> usize {
        if edge % 3 == 2 {
            edge - 2
        } else {
            edge + 1
        }
    }

    /// Previous halfedge in a triangle.
    pub fn prev_halfedge(&self, edge: usize) -> usize {
        if edge % 3 == 0 {
            edge + 2
        } else {
            edge - 1
        }
    }

    /// Returns the triangle of an edge.
    pub fn triangle_of_edge(&self, edge: usize) -> usize {
        edge / 3
    }

    /// Returns the edges of a triangle.
    pub fn edges_of_triangle(&self, triangle: usize) -> [usize; 3] {
        [3 * triangle, 3 * triangle + 1, 3 * triangle + 2]
    }

    /// Returns the points of a triangle.
    pub fn points_of_triangle(&self, triangle: usize) -> [usize; 3] {
        // self.edges_of_triangle(t)
        //     .into_iter()
        //     .map(|e| self.triangles[*e])
        //     .collect()
        let e = self.edges_of_triangle(triangle);
        [
            self.triangles[e[0]],
            self.triangles[e[1]],
            self.triangles[e[2]],
        ]
    }

    /// Triangle circumcenter.
    pub fn triangle_center(&self, points: &[Point2D], triangle: usize) -> Point2D {
        let p = self.points_of_triangle(triangle);
        points[p[0]].circumcenter(&points[p[1]], &points[p[2]])
    }

    /// Returns the edges around a point connected to halfedge '*start*'.
    pub fn edges_around_point(&self, start: usize) -> Vec<usize> {
        let mut result = vec![];
        let mut incoming = start;
        let mut outgoing: usize;
        // let mut i = 0;
        loop {
            if result.contains(&incoming) {
                break;
            }
            result.push(incoming);
            outgoing = self.next_halfedge(incoming);
            incoming = self.halfedges[outgoing];
            if incoming == EMPTY {
                break;
            } else if incoming == start {
                result.push(incoming);
                break;
            }
            // i += 1;
            // if i > 100 {
            //     // println!("{} {} {}", outgoing, incoming, start);
            //     break;
            // }
        }
        result
    }

    pub fn natural_neighbours_from_incoming_edge(&self, start: usize) -> Vec<usize> {
        let mut result = vec![];
        //result.push(self.triangles[self.next_halfedge(start)]);
        let mut incoming = start;
        let mut outgoing: usize;
        loop {
            result.push(self.triangles[incoming]);
            outgoing = self.next_halfedge(incoming);
            incoming = self.halfedges[outgoing];
            if incoming == EMPTY {
                break;
            } else if incoming == start {
                break;
            }
        }
        result
    }

    pub fn natural_neighbours_2nd_order(&self, start: usize) -> Vec<usize> {
        let mut set = HashSet::new();
        let mut edges = vec![];
        // result.push(self.triangles[self.next_halfedge(start)]);
        // set.insert(self.triangles[self.next_halfedge(start)]);
        let mut incoming = start;
        let mut outgoing: usize;
        loop {
            set.insert(self.triangles[incoming]);
            outgoing = self.next_halfedge(incoming);
            incoming = self.halfedges[outgoing];
            edges.push(outgoing);
            if incoming == EMPTY {
                break;
            } else if incoming == start {
                break;
            }
        }

        for start in edges {
            incoming = start;
            loop {
                set.insert(self.triangles[incoming]);
                outgoing = self.next_halfedge(incoming);
                incoming = self.halfedges[outgoing];
                if incoming == EMPTY {
                    break;
                } else if incoming == start {
                    break;
                }
            }
        }

        set.into_iter().map(|i| i).collect()
    }

    /// Returns the indices of the adjacent triangles to a triangle.
    pub fn triangles_adjacent_to_triangle(&self, triangle: usize) -> Vec<usize> {
        let mut adjacent_triangles: Vec<usize> = vec![];
        let mut opposite: usize;
        for e in self.edges_of_triangle(triangle).iter() {
            opposite = self.halfedges[*e];
            if opposite != EMPTY {
                adjacent_triangles.push(self.triangle_of_edge(opposite));
            }
        }
        adjacent_triangles
    }

    fn add_triangle(
        &mut self,
        i0: usize,
        i1: usize,
        i2: usize,
        a: usize,
        b: usize,
        c: usize,
    ) -> usize {
        let t = self.triangles.len();

        self.triangles.push(i0);
        self.triangles.push(i1);
        self.triangles.push(i2);

        self.halfedges.push(a);
        self.halfedges.push(b);
        self.halfedges.push(c);

        if a != EMPTY {
            self.halfedges[a] = t;
        }
        if b != EMPTY {
            self.halfedges[b] = t + 1;
        }
        if c != EMPTY {
            self.halfedges[c] = t + 2;
        }

        t
    }

    fn legalize(&mut self, a: usize, points: &[Point2D], hull: &mut Hull) -> usize {
        let b = self.halfedges[a];

        // if the pair of triangles doesn't satisfy the Delaunay condition
        // (p1 is inside the circumcircle of [p0, pl, pr]), flip them,
        // then do the same check/flip recursively for the new pair of triangles
        //
        //           pl                    pl
        //          /||\                  /  \
        //       al/ || \bl            al/    \a
        //        /  ||  \              /      \
        //       /  a||b  \    flip    /___ar___\
        //     p0\   ||   /p1   =>   p0\---bl---/p1
        //        \  ||  /              \      /
        //       ar\ || /br             b\    /br
        //          \||/                  \  /
        //           pr                    pr
        //
        let ar = self.prev_halfedge(a);

        if b == EMPTY {
            return ar;
        }

        let al = self.next_halfedge(a);
        let bl = self.prev_halfedge(b);

        let p0 = self.triangles[ar];
        let pr = self.triangles[a];
        let pl = self.triangles[al];
        let p1 = self.triangles[bl];

        let illegal = (&points[p0]).in_circle(&points[pr], &points[pl], &points[p1]);
        if illegal {
            self.triangles[a] = p1;
            self.triangles[b] = p0;

            let hbl = self.halfedges[bl];
            let har = self.halfedges[ar];

            // edge swapped on the other side of the hull (rare); fix the halfedge reference
            if hbl == EMPTY {
                let mut e = hull.start;

                loop {
                    if hull.tri[e] == bl {
                        hull.tri[e] = a;
                        break;
                    }
                    e = hull.next[e];
                    if e == hull.start || e == EMPTY {
                        // notice, I added the || e == EMPTY after
                        // finding a bug. I don't know about this.
                        break;
                    }
                }
            }

            self.halfedges[a] = hbl;
            self.halfedges[b] = har;
            self.halfedges[ar] = bl;

            if hbl != EMPTY {
                self.halfedges[hbl] = a;
            }
            if har != EMPTY {
                self.halfedges[har] = b;
            }
            if bl != EMPTY {
                self.halfedges[bl] = ar;
            }

            let br = self.next_halfedge(b);

            self.legalize(a, points, hull);
            return self.legalize(br, points, hull);
        }
        ar
    }
}

// data structure for tracking the edges of the advancing convex hull
struct Hull {
    prev: Vec<usize>,
    next: Vec<usize>,
    tri: Vec<usize>,
    hash: Vec<usize>,
    start: usize,
    center: Point2D,
}

impl Hull {
    fn new(n: usize, center: Point2D, i0: usize, i1: usize, i2: usize, points: &[Point2D]) -> Self {
        let hash_len = (n as f64).sqrt() as usize;

        let mut hull = Self {
            prev: vec![0; n],            // edge to prev edge
            next: vec![0; n],            // edge to next edge
            tri: vec![0; n],             // edge to adjacent halfedge
            hash: vec![EMPTY; hash_len], // angular edge hash
            start: i0,
            center,
        };

        hull.next[i0] = i1;
        hull.prev[i2] = i1;
        hull.next[i1] = i2;
        hull.prev[i0] = i2;
        hull.next[i2] = i0;
        hull.prev[i1] = i0;

        hull.tri[i0] = 0;
        hull.tri[i1] = 1;
        hull.tri[i2] = 2;

        hull.hash_edge(&points[i0], i0);
        hull.hash_edge(&points[i1], i1);
        hull.hash_edge(&points[i2], i2);

        hull
    }

    fn hash_key(&self, p: &Point2D) -> usize {
        let dx = p.x - self.center.x;
        let dy = p.y - self.center.y;

        let p = dx / (dx.abs() + dy.abs());
        let a = (if dy > 0.0 { 3.0 - p } else { 1.0 + p }) / 4.0; // [0..1]

        let len = self.hash.len();
        (((len as f64) * a).floor() as usize) % len
    }

    fn hash_edge(&mut self, p: &Point2D, i: usize) {
        let key = self.hash_key(p);
        self.hash[key] = i;
    }

    fn find_visible_edge(&self, p: &Point2D, points: &[Point2D]) -> (usize, bool) {
        let mut start: usize = 0;
        let key = self.hash_key(p);
        let len = self.hash.len();
        for j in 0..len {
            start = self.hash[(key + j) % len];
            if start != EMPTY && self.next[start] != EMPTY {
                break;
            }
        }
        start = self.prev[start];
        let mut e = start;

        while !p.orient(&points[e], &points[self.next[e]]) {
            e = self.next[e];
            if e == start {
                return (EMPTY, false);
            }
        }
        (e, e == start)
    }
}

fn calc_bbox_center(points: &[Point2D]) -> Point2D {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in points.iter() {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    Point2D {
        x: (min_x + max_x) / 2.0,
        y: (min_y + max_y) / 2.0,
    }
}

fn find_closest_point(points: &[Point2D], p0: &Point2D) -> Option<usize> {
    let mut min_dist = f64::INFINITY;
    let mut k: usize = 0;
    for (i, p) in points.iter().enumerate() {
        let d = p0.distance_squared(p);
        if d > 0.0 && d < min_dist {
            k = i;
            min_dist = d;
        }
    }
    if min_dist == f64::INFINITY {
        None
    } else {
        Some(k)
    }
}

fn find_seed_triangle(points: &[Point2D]) -> Option<(usize, usize, usize)> {
    // pick a seed point close to the center
    let bbox_center = calc_bbox_center(points);
    let i0 = find_closest_point(points, &bbox_center)?;
    let p0 = &points[i0];

    // find the point closest to the seed
    let i1 = find_closest_point(points, p0)?;
    let p1 = &points[i1];

    // find the third point which forms the smallest circumcircle with the first two
    let mut min_radius = f64::INFINITY;
    let mut i2: usize = 0;
    for (i, p) in points.iter().enumerate() {
        if i == i0 || i == i1 {
            continue;
        }
        let r = p0.circumradius2(p1, p);
        if r < min_radius {
            i2 = i;
            min_radius = r;
        }
    }

    if min_radius == f64::INFINITY {
        None
    } else {
        // swap the order of the seed points for counter-clockwise orientation
        Some(if p0.orient(p1, &points[i2]) {
            (i0, i2, i1)
        } else {
            (i0, i1, i2)
        })
    }
}

/// Triangulate a set of 2D points.
/// Returns `None` if no triangulation exists for the input (e.g. all points are collinear).
pub fn triangulate(points: &[Point2D]) -> Option<Triangulation> {
    let n = points.len();

    let (i0, i1, i2) = find_seed_triangle(points)?;
    let center = (&points[i0]).circumcenter(&points[i1], &points[i2]);

    let mut triangulation = Triangulation::new(n);
    triangulation.add_triangle(i0, i1, i2, EMPTY, EMPTY, EMPTY);

    // sort the points by distance from the seed triangle circumcenter
    let mut dists: Vec<_> = points
        .iter()
        .enumerate()
        .map(|(i, point)| (i, center.distance_squared(point)))
        .collect();

    dists.sort_unstable_by(|&(_, da), &(_, db)| da.partial_cmp(&db).unwrap());

    let mut hull = Hull::new(n, center, i0, i1, i2, points);

    for (k, &(i, _)) in dists.iter().enumerate() {
        let p = &points[i];

        // skip near-duplicates
        if k > 0 && p.nearly_equals(&points[dists[k - 1].0]) {
            continue;
        }
        // skip seed triangle points
        if i == i0 || i == i1 || i == i2 {
            continue;
        }

        // find a visible edge on the convex hull using edge hash
        let (mut e, walk_back) = hull.find_visible_edge(p, points);

        if e == EMPTY {
            continue; // likely a near-duplicate point; skip it
        }

        // add the first triangle from the point
        let t = triangulation.add_triangle(e, i, hull.next[e], EMPTY, EMPTY, hull.tri[e]);

        // recursively flip triangles from the point until they satisfy the Delaunay condition
        hull.tri[i] = triangulation.legalize(t + 2, points, &mut hull);
        hull.tri[e] = t; // keep track of boundary triangles on the hull

        // walk forward through the hull, adding more triangles and flipping recursively
        let mut n = hull.next[e];
        loop {
            let q = hull.next[n];
            if !p.orient(&points[n], &points[q]) {
                break;
            }
            let t = triangulation.add_triangle(n, i, q, hull.tri[i], EMPTY, hull.tri[n]);
            hull.tri[i] = triangulation.legalize(t + 2, points, &mut hull);
            hull.next[n] = EMPTY; // mark as removed
            n = q;
        }

        // walk backward from the other side, adding more triangles and flipping
        if walk_back {
            loop {
                let q = hull.prev[e];

                if !p.orient(&points[q], &points[e]) {
                    break;
                }
                let t = triangulation.add_triangle(q, i, e, EMPTY, hull.tri[e], hull.tri[q]);
                triangulation.legalize(t + 2, points, &mut hull);
                hull.tri[q] = t;
                hull.next[e] = EMPTY; // mark as removed
                e = q;
            }
        }

        // update the hull indices
        hull.prev[i] = e;
        hull.next[i] = n;
        hull.prev[n] = i;
        hull.next[e] = i;
        hull.start = e;

        // save the two new edges in the hash table
        hull.hash_edge(p, i);
        hull.hash_edge(&points[e], e);
    }

    // expose hull as a vector of point indices
    let mut e = hull.start;
    loop {
        triangulation.hull.push(e);
        e = hull.next[e];
        if e == hull.start {
            break;
        }
    }

    triangulation.triangles.shrink_to_fit();
    triangulation.halfedges.shrink_to_fit();

    Some(triangulation)
}
