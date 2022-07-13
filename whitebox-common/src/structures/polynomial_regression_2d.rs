use crate::na::{DMatrix, DVector};
use std::f64::EPSILON;
use std::io::Error;
use std::io::ErrorKind;

pub struct PolynomialRegression2D {
    poly_order: usize,
    pub num_coefficients: usize,
    pub coefficients: Vec<[f64; 2]>,
    pub residuals: Vec<f64>,
}

impl PolynomialRegression2D {
    pub fn new(
        poly_order: usize,
        x_prime: &[f64],
        y_prime: &[f64],
        x: &[f64],
        y: &[f64],
    ) -> Result<PolynomialRegression2D, Error> {
        // how many points are there?
        let n = x.len();

        // make sure that they all have the same length.
        if y.len() != n || x_prime.len() != n || y_prime.len() != n {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Error: All input data to polynomial_regression_2d must have the same length",
            ));
        }

        let mut num_coefficients = 0;
        for j in 0..=poly_order {
            for _ in 0..=(poly_order - j) {
                num_coefficients += 1;
            }
        }
        // Solve the forward transformation equations
        let mut vals = Vec::with_capacity(n * num_coefficients);
        for i in 0..n {
            for j in 0..=poly_order {
                for k in 0..=(poly_order - j) {
                    vals.push(x[i].powi(j as i32) * y[i].powi(k as i32));
                }
            }
        }

        let a = DMatrix::from_row_slice(n, num_coefficients, &vals);
        let a_svd = a.svd(true, true);
        let x_eq = a_svd
            .solve(&DVector::from_row_slice(x_prime), EPSILON)
            .unwrap();
        let y_eq = a_svd
            .solve(&DVector::from_row_slice(y_prime), EPSILON)
            .unwrap();

        let mut coefficients = Vec::with_capacity(num_coefficients);
        for i in 0..num_coefficients {
            coefficients.push([x_eq[i], y_eq[i]]);
        }

        // residuals
        let mut residuals = Vec::with_capacity(n);
        let (mut diff_x, mut diff_y): (f64, f64);
        let (mut v1, mut v2): (f64, f64);
        for i in 0..n {
            let p = i * num_coefficients;
            v1 = 0.0;
            v2 = 0.0;
            for q in 0..num_coefficients {
                v1 += coefficients[q][0] * vals[p + q];
                v2 += coefficients[q][1] * vals[p + q];
            }
            diff_x = x_prime[i] - v1;
            diff_y = y_prime[i] - v2;
            residuals.push((diff_x * diff_x + diff_y * diff_y).sqrt());
        }

        let regress = PolynomialRegression2D {
            poly_order: poly_order,
            num_coefficients: num_coefficients,
            coefficients: coefficients,
            residuals: residuals,
        };

        Ok(regress)
    }

    pub fn get_value(&self, x: f64, y: f64) -> (f64, f64) {
        let mut x_prime = 0.0;
        let mut y_prime = 0.0;
        let mut q = 0;
        let mut v: f64;
        for j in 0..=self.poly_order {
            for k in 0..=(self.poly_order - j) {
                v = x.powi(j as i32) * y.powi(k as i32);
                x_prime += self.coefficients[q][0] * v;
                y_prime += self.coefficients[q][1] * v;
                q += 1;
            }
        }
        (x_prime, y_prime)
    }

    pub fn get_average_residual(&self) -> f64 {
        let mut sum = 0.0;
        for i in 0..self.residuals.len() {
            sum += self.residuals[i];
        }
        sum / self.residuals.len() as f64
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::prelude::*;
    #[test]
    fn test_polynomial_regression_2d() {
        let mut rng = thread_rng();

        let mut x = vec![];
        let mut y = vec![];
        let mut x_prime = vec![];
        let mut y_prime = vec![];
        let mut x_val: f64;
        let mut y_val: f64;
        for _ in 0..150 {
            x_val = rng.gen_range(0.0, 10.0);
            y_val = rng.gen_range(0.0, 10.0);
            x.push(x_val);
            y.push(y_val);
            x_prime.push(0.58 * x_val + 0.32 * y_val + 46.0);
            y_prime.push(0.29 * x_val + 0.91 * y_val + 13.8);
        }

        let pr2d = PolynomialRegression2D::new(1, &x_prime, &y_prime, &x, &y).unwrap();

        const SMALL_NUM: f64 = 0.0000001f64;

        assert!(pr2d.coefficients[0][0] - 46.0 < SMALL_NUM);
        assert!(pr2d.coefficients[1][0] - 0.32 < SMALL_NUM);
        assert!(pr2d.coefficients[2][0] - 0.91 < SMALL_NUM);

        assert!(pr2d.coefficients[0][1] - 13.8 < SMALL_NUM);
        assert!(pr2d.coefficients[1][1] - 0.91 < SMALL_NUM);
        assert!(pr2d.coefficients[2][1] - 0.29 < SMALL_NUM);

        let val = pr2d.get_value(1.0, 1.0);
        assert!(val.0 - (0.58 + 0.32 + 46.0) < SMALL_NUM);
        assert!(val.1 - (0.29 + 0.91 + 13.8) < SMALL_NUM);
    }
}
