//! Based on rbf-interp, a library for multidimensional interpolation.
//! by Raph Levien (raphlinus)
//! https://github.com/linebender/rbf-interp/blob/master/src/lib.rs
use nalgebra::{
    DMatrix, 
    DVector, 
    SVD
};

#[derive(Clone, Copy)]
pub enum Basis {
    ThinPlateSpine(f64),
    PolyHarmonic(i32),
    Gaussian(f64),
    MultiQuadric(f64),
    InverseMultiQuadric(f64),
}

impl Basis {
    fn eval(&self, r: f64) -> f64 {
        match self {
            Basis::ThinPlateSpine(c) => (c * c + r * r) * (c * c + r * r).ln(), // from Surfer (see Geospatial Analysis 6th Ed., 2018)
            Basis::PolyHarmonic(n) if n % 2 == 0 => {
                // Somewhat arbitrary but don't expect tiny nonzero values.
                if r < 1e-12 {
                    0.0
                } else {
                    r.powi(*n) * r.ln()
                }
            }
            Basis::PolyHarmonic(n) => r.powi(*n),
            // Note: it might be slightly more efficient to pre-recip c, but
            // let's keep code clean for now.
            Basis::Gaussian(c) => (-(r / c).powi(2)).exp(),
            Basis::MultiQuadric(c) => r.hypot(*c),
            Basis::InverseMultiQuadric(c) => (r * r + c * c).powf(-0.5),
        }
    }
}

pub struct RadialBasisFunction {
    // Note: could make basis a type-level parameter
    basis: Basis,
    // TODO(explore): use matrix & slicing instead (fewer allocs).
    // An array of n vectors each of size m.
    centers: Vec<DVector<f64>>,
    // An m x n' matrix, where n' is the number of basis functions (including polynomial),
    // and m is the number of coords.
    deltas: DMatrix<f64>,
}

impl RadialBasisFunction {
    pub fn eval(&self, coords: DVector<f64>) -> DVector<f64> {
        let n = self.centers.len();
        let basis = DVector::from_fn(self.deltas.ncols(), |row, _c| {
            if row < n {
                // component from basis functions
                self.basis.eval((&coords - &self.centers[row]).norm())
            } else if row == n {
                // constant component
                1.0
            } else {
                // linear component
                coords[row - n - 1]
            }
        });
        &self.deltas * basis
    }

    // The order for the polynomial part, meaning terms up to (order - 1) are included.
    // This usage is consistent with Wilna du Toit's masters thesis "Radial Basis
    // Function Interpolation"
    // Notice, a PolyHarmonic 2 and order of 2 is a thin plate spline.
    pub fn create(
        centers: Vec<DVector<f64>>,
        vals: Vec<DVector<f64>>,
        basis: Basis,
        order: usize,
    ) -> RadialBasisFunction {
        let n = centers.len();
        // n x m matrix. There's probably a better way to do this, ah well.
        let mut vals = DMatrix::from_columns(&vals).transpose();
        let n_aug = match order {
            // Pure radial basis functions
            0 => n,
            // Constant term
            1 => n + 1,
            // Affine terms
            2 => n + 1 + centers[0].len(),
            _ => unimplemented!("don't yet support higher order polynomials"),
        };
        // Augment to n' x m matrix, where n' is the total number of basis functions.
        if n_aug > n {
            vals = vals.resize_vertically(n_aug, 0.0);
        }
        // We translate the system to center the mean at the origin so that when
        // the system is degenerate, the pseudoinverse below minimizes the linear
        // coefficients.
        let means: Vec<_> = if order == 2 {
            let n = centers.len();
            let n_recip = (n as f64).recip();
            (0..centers[0].len())
                .map(|i| centers.iter().map(|c| c[i]).sum::<f64>() * n_recip)
                .collect()
        } else {
            Vec::new()
        };
        let mat = DMatrix::from_fn(n_aug, n_aug, |r, c| {
            if r < n && c < n {
                basis.eval((&centers[r] - &centers[c]).norm())
            } else if r < n {
                if c == n {
                    1.0
                } else {
                    centers[r][c - n - 1] - means[c - n - 1]
                }
            } else if c < n {
                if r == n {
                    1.0
                } else {
                    centers[c][r - n - 1] - means[r - n - 1]
                }
            } else {
                0.0
            }
        });
        // inv is an n' x n' matrix.
        let svd = SVD::new(mat, true, true);
        // Use pseudo-inverse here to get "least squares fit" when there's
        // no unique result (for example, when dimensionality is too small).
        let inv = svd.pseudo_inverse(1e-6).expect("error inverting matrix");
        // Again, this transpose feels like I don't know what I'm doing.
        let mut deltas = (inv * vals).transpose();
        if order == 2 {
            let m = centers[0].len();
            for i in 0..deltas.nrows() {
                let offset: f64 = (0..m).map(|j| means[j] * deltas[(i, n + 1 + j)]).sum();
                deltas[(i, n)] -= offset;
            }
        }
        RadialBasisFunction {
            basis,
            centers,
            deltas,
        }
    }
}
