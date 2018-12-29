/////////////////////////////////////////////
// A generic 2-dimensional array structure //
/////////////////////////////////////////////
use std::io::Error;
use std::io::ErrorKind;
use std::ops::{AddAssign, Index, IndexMut, SubAssign};

#[derive(Debug)]
pub struct Array2D<T: Copy + AddAssign + SubAssign> {
    pub columns: isize,
    pub rows: isize,
    data: Vec<T>,
    pub nodata: T,
}

impl<T> Array2D<T>
where
    T: Copy + AddAssign + SubAssign,
{
    pub fn new(
        rows: isize,
        columns: isize,
        initial_value: T,
        nodata: T,
    ) -> Result<Array2D<T>, Error> {
        if rows < 0 || columns < 0 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Only non-negative rows and columns values accepted.",
            ));
        }
        let array = Array2D {
            columns: columns,
            rows: rows,
            nodata: nodata,
            data: vec![initial_value; (rows * columns) as usize],
        };
        Ok(array)
    }

    pub fn set_value(&mut self, row: isize, column: isize, value: T) {
        if column >= 0 && row >= 0 {
            if column < self.columns && row < self.rows {
                self.data[(row * self.columns + column) as usize] = value;
            }
        }
    }

    pub fn get_value(&self, row: isize, column: isize) -> T {
        // if column < 0 { return self.nodata; }
        // if row < 0 { return self.nodata; }
        // if column >= self.columns { return self.nodata; }
        // if row >= self.rows { return self.nodata; }
        // let idx = row * self.columns + column;
        // self.data[idx as usize]
        if row < 0 || column < 0 {
            return self.nodata;
        }
        if row >= self.rows || column >= self.columns {
            return self.nodata;
        }
        self.data[(row * self.columns + column) as usize]
    }

    pub fn increment(&mut self, row: isize, column: isize, value: T) {
        if column >= 0 && row >= 0 {
            if column < self.columns && row < self.rows {
                self.data[(row * self.columns + column) as usize] += value;
            }
        }
    }

    pub fn decrement(&mut self, row: isize, column: isize, value: T) {
        if column >= 0 && row >= 0 {
            if column < self.columns && row < self.rows {
                self.data[(row * self.columns + column) as usize] -= value;
            }
        }
    }

    pub fn set_row_data(&mut self, row: isize, values: Vec<T>) {
        for column in 0..values.len() as isize {
            if row >= 0 {
                if column < self.columns && row < self.rows {
                    self.data[(row * self.columns + column) as usize] = values[column as usize];
                }
            }
        }
    }

    pub fn get_row_data(&self, row: isize) -> Vec<T> {
        let columns = self.columns as usize;
        let mut values: Vec<T> = vec![self.nodata; columns];
        if row >= 0 && row < self.rows as isize {
            for column in 0..values.len() {
                values[column] = self.data[row as usize * columns + column];
            }
        }
        values
    }

    /// Increments an entire row of data at one time.
    pub fn increment_row_data(&mut self, row: isize, values: Vec<T>) {
        for column in 0..values.len() as isize {
            if row >= 0 {
                if column < self.columns && row < self.rows {
                    self.data[(row * self.columns + column) as usize] += values[column as usize];
                }
            }
        }
    }

    /// Decrements an entire row of data at one time.
    pub fn decrement_row_data(&mut self, row: isize, values: Vec<T>) {
        for column in 0..values.len() as isize {
            if row >= 0 {
                if column < self.columns && row < self.rows {
                    self.data[(row * self.columns + column) as usize] -= values[column as usize];
                }
            }
        }
    }

    pub fn set_data_from_other(&mut self, other: &Array2D<T>) -> Result<(), Error> {
        if self.rows != other.rows || self.columns != other.columns {
            return Err(Error::new(
                ErrorKind::Other,
                "Rasters must have the same dimensions and extent.",
            ));
        }
        self.data = other.data.clone();
        Ok(())
    }

    pub fn columns(&self) -> isize {
        self.columns
    }
    pub fn rows(&self) -> isize {
        self.rows
    }
    pub fn nodata(&self) -> T {
        self.nodata
    }
}

impl<T: Copy> Index<(isize, isize)> for Array2D<T>
where
    T: Copy + AddAssign + SubAssign,
{
    type Output = T;

    fn index<'a>(&'a self, index: (isize, isize)) -> &'a T {
        let row = index.0;
        let column = index.1;
        if column < 0 {
            return &self.nodata;
        }
        if row < 0 {
            return &self.nodata;
        }
        if column >= self.columns {
            return &self.nodata;
        }
        if row >= self.rows {
            return &self.nodata;
        }
        let idx = row * self.columns + column;
        &self.data[idx as usize]
    }
}

impl<T: Copy> IndexMut<(isize, isize)> for Array2D<T>
where
    T: Copy + AddAssign + SubAssign,
{
    fn index_mut<'a>(&'a mut self, index: (isize, isize)) -> &'a mut T {
        let row = index.0;
        let column = index.1;
        if column < 0 {
            return &mut self.nodata;
        }
        if row < 0 {
            return &mut self.nodata;
        }
        if column >= self.columns {
            return &mut self.nodata;
        }
        if row >= self.rows {
            return &mut self.nodata;
        }
        let idx = row * self.columns + column;
        &mut self.data[idx as usize]
    }
}
