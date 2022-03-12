/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 02/06/2017
Last Modified: 09/02/2019
License: MIT
*/

// pub mod raster;

// extern crate late_static;
// use late_static::LateStatic;
// pub static USE_COMPRESSION: LateStatic<bool> = LateStatic::new();
// pub static USE_COMPRESSION: bool = true;
extern crate num_traits;

mod arcascii_raster;
mod arcbinary_raster;
mod esri_bil;
pub mod geotiff;
mod grass_raster;
mod idrisi_raster;
mod saga_raster;
mod surfer7_raster;
mod surfer_ascii_raster;
mod whitebox_raster;

use self::arcascii_raster::*;
use self::arcbinary_raster::*;
use self::esri_bil::*;
use self::geotiff::*;
use self::grass_raster::*;
use self::idrisi_raster::*;
use self::saga_raster::*;
use self::surfer7_raster::*;
use self::surfer_ascii_raster::*;
use self::whitebox_raster::*;
use num_traits::cast::AsPrimitive;
use whitebox_common::structures::{Array2D, BoundingBox};
use whitebox_common::utils::*;
use std::cmp::Ordering::Equal;
use std::default::Default;
use std::f64;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::Error;
use std::io::ErrorKind;
use std::ops::{AddAssign, Index, IndexMut, SubAssign};
use std::path::Path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
// use rayon::prelude::*;

/// Raster is a common data structure that abstracts over several raster data formats,
/// including GeoTIFFs, ArcGIS ASCII and binary rasters, Whitebox rasters, Idrisi
/// rasters, Saga rasters, and GRASS ASCII rasters.
///
/// Examples:
///
/// ```
/// // Read an existing raster file
/// let input = Raster::new(&input_file, "r")?;
///
/// // Create a new raster file with the dimensions
/// // and location of an existing file.
/// let mut output = Raster::initialize_using_file(&output_file, &input);
/// ```
#[derive(Default, Clone)]
pub struct Raster {
    pub file_name: String,
    pub file_mode: String,
    pub raster_type: RasterType,
    pub configs: RasterConfigs,
    data: Vec<f64>,
}

impl Index<(isize, isize)> for Raster {
    type Output = f64;

    fn index<'a>(&'a self, index: (isize, isize)) -> &'a f64 {
        let row = index.0;
        let column = index.1;

        if column < 0 {
            return &self.configs.nodata;
        }
        if row < 0 {
            return &self.configs.nodata;
        }

        let c: usize = column as usize;
        let r: usize = row as usize;

        if c >= self.configs.columns {
            return &self.configs.nodata;
        }
        if r >= self.configs.rows {
            return &self.configs.nodata;
        }
        let idx: usize = r * self.configs.columns + c;
        &self.data[idx]
    }
}

impl IndexMut<(isize, isize)> for Raster {
    fn index_mut<'a>(&'a mut self, index: (isize, isize)) -> &'a mut f64 {
        let row = index.0;
        let column = index.1;
        if column < 0 {
            return &mut self.configs.nodata;
        }
        if row < 0 {
            return &mut self.configs.nodata;
        }
        let c: usize = column as usize;
        let r: usize = row as usize;
        if c >= self.configs.columns {
            return &mut self.configs.nodata;
        }
        if r >= self.configs.rows {
            return &mut self.configs.nodata;
        }
        let idx = r * self.configs.columns + c;
        &mut self.data[idx as usize]
    }
}

impl Raster {
    /// Creates an in-memory `Raster` object. The data are either
    /// read from an existing file (`file_name`; `file_mode` is 'r') or
    /// prepared for new file creation (`file_mode` is 'w') The raster format
    /// will be determined by the file extension of the `file_name` string.
    ///
    /// To create a new `Raster` file, most applications should prefer the
    /// `initialize_using_config` or `initialize_using_file` functions instead.
    pub fn new<'a>(file_name: &'a str, file_mode: &'a str) -> Result<Raster, Error> {
        let fm: String = file_mode.to_lowercase();
        let mut r = Raster {
            file_name: file_name.to_string(),
            file_mode: fm.clone(),
            raster_type: get_raster_type_from_file(file_name.to_string(), fm.clone()),
            ..Default::default()
        };
        if r.file_mode.contains("r") {
            match get_raster_type_from_file(file_name.to_string(), fm) {
                RasterType::ArcBinary => {
                    let _ = read_arcbinary(&r.file_name, &mut r.configs, &mut r.data)?;
                    return Ok(r);
                }
                RasterType::ArcAscii => {
                    let _ = read_arcascii(&r.file_name, &mut r.configs, &mut r.data)?;
                    return Ok(r);
                }
                RasterType::EsriBil => {
                    let _ = read_esri_bil(&r.file_name, &mut r.configs, &mut r.data)?;
                    return Ok(r);
                }
                RasterType::GeoTiff => {
                    let _ = read_geotiff(&r.file_name, &mut r.configs, &mut r.data)?;
                    r.update_min_max();
                    return Ok(r);
                }
                RasterType::GrassAscii => {
                    let _ = read_grass_raster(&r.file_name, &mut r.configs, &mut r.data)?;
                    return Ok(r);
                }
                RasterType::IdrisiBinary => {
                    let _ = read_idrisi(&r.file_name, &mut r.configs, &mut r.data)?;
                    return Ok(r);
                }
                RasterType::SagaBinary => {
                    let _ = read_saga(&r.file_name, &mut r.configs, &mut r.data)?;
                    return Ok(r);
                }
                RasterType::Surfer7Binary => {
                    let _ = read_surfer7(&r.file_name, &mut r.configs, &mut r.data)?;
                    return Ok(r);
                }
                RasterType::SurferAscii => {
                    let _ = read_surfer_ascii_raster(&r.file_name, &mut r.configs, &mut r.data)?;
                    return Ok(r);
                }
                RasterType::Whitebox => {
                    let _ = read_whitebox(&r.file_name, &mut r.configs, &mut r.data)?;
                    return Ok(r);
                }
                RasterType::Unknown => {
                    return Err(Error::new(ErrorKind::Other, "Unrecognized raster type"));
                }
            }
        } else {
            // write
            return Ok(r);
        }
        // Err(Error::new(ErrorKind::Other, "Error creating raster"))
    }

    /// Creates a new in-memory `Raster` object with grid extent and location
    /// based on specified configurations contained within a `RasterConfigs`.
    pub fn initialize_using_config<'a>(file_name: &'a str, configs: &'a RasterConfigs) -> Raster {
        let new_file_name = if file_name.contains(".") {
            file_name.to_string()
        } else {
            // likely no extension provided; default to .tif
            format!("{}.tif", file_name)
        };
        let mut output = Raster {
            file_name: new_file_name.clone(),
            // configs: configs.clone(),
            ..Default::default()
        };
        output.file_mode = "w".to_string();
        output.raster_type = get_raster_type_from_file(new_file_name.clone(), "w".to_string());

        output.configs.rows = configs.rows;
        output.configs.columns = configs.columns;
        output.configs.north = configs.north;
        output.configs.south = configs.south;
        output.configs.east = configs.east;
        output.configs.west = configs.west;
        output.configs.resolution_x = configs.resolution_x;
        output.configs.resolution_y = configs.resolution_y;
        output.configs.nodata = configs.nodata;
        output.configs.data_type = configs.data_type;
        output.configs.photometric_interp = configs.photometric_interp;
        output.configs.palette = configs.palette.clone();
        output.configs.projection = configs.projection.clone();
        output.configs.xy_units = configs.xy_units.clone();
        output.configs.z_units = configs.z_units.clone();
        output.configs.endian = configs.endian.clone();
        output.configs.pixel_is_area = configs.pixel_is_area;
        output.configs.epsg_code = configs.epsg_code;
        output.configs.coordinate_ref_system_wkt = configs.coordinate_ref_system_wkt.clone();
        output.configs.model_tiepoint = configs.model_tiepoint.clone();
        output.configs.model_pixel_scale = configs.model_pixel_scale.clone();
        output.configs.model_transformation = configs.model_transformation.clone();
        output.configs.geo_key_directory = configs.geo_key_directory.clone();
        output.configs.geo_double_params = configs.geo_double_params.clone();
        output.configs.geo_ascii_params = configs.geo_ascii_params.clone();

        if output.raster_type == RasterType::SurferAscii
            || output.raster_type == RasterType::Surfer7Binary
        {
            output.configs.nodata = 1.71041e38;
        }
        output
            .data
            .reserve(output.configs.rows * output.configs.columns);
        output.data = vec![output.configs.nodata; output.configs.rows * output.configs.columns];

        output
    }

    /// Creates a new in-memory `Raster` object with grid extent and location
    /// based on specified configurations contained within a `RasterConfigs`.
    pub fn initialize_using_array2d<'a, T: AsPrimitive<f64> + Copy + AddAssign + SubAssign>(file_name: &'a str, configs: &'a RasterConfigs, data: Array2D<T>) -> Raster {
        let new_file_name = if file_name.contains(".") {
            file_name.to_string()
        } else {
            // likely no extension provided; default to .tif
            format!("{}.tif", file_name)
        };
        let mut output = Raster {
            file_name: new_file_name.clone(),
            // configs: configs.clone(),
            ..Default::default()
        };
        output.file_mode = "w".to_string();
        output.raster_type = get_raster_type_from_file(new_file_name.clone(), "w".to_string());

        output.configs.rows = configs.rows;
        output.configs.columns = configs.columns;
        output.configs.north = configs.north;
        output.configs.south = configs.south;
        output.configs.east = configs.east;
        output.configs.west = configs.west;
        output.configs.resolution_x = configs.resolution_x;
        output.configs.resolution_y = configs.resolution_y;
        output.configs.nodata = configs.nodata;
        output.configs.data_type = configs.data_type;
        output.configs.photometric_interp = configs.photometric_interp;
        output.configs.palette = configs.palette.clone();
        output.configs.projection = configs.projection.clone();
        output.configs.xy_units = configs.xy_units.clone();
        output.configs.z_units = configs.z_units.clone();
        output.configs.endian = configs.endian.clone();
        output.configs.pixel_is_area = configs.pixel_is_area;
        output.configs.epsg_code = configs.epsg_code;
        output.configs.coordinate_ref_system_wkt = configs.coordinate_ref_system_wkt.clone();
        output.configs.model_tiepoint = configs.model_tiepoint.clone();
        output.configs.model_pixel_scale = configs.model_pixel_scale.clone();
        output.configs.model_transformation = configs.model_transformation.clone();
        output.configs.geo_key_directory = configs.geo_key_directory.clone();
        output.configs.geo_double_params = configs.geo_double_params.clone();
        output.configs.geo_ascii_params = configs.geo_ascii_params.clone();

        if output.raster_type == RasterType::SurferAscii
            || output.raster_type == RasterType::Surfer7Binary
        {
            output.configs.nodata = 1.71041e38;
        }
        output
            .data
            .reserve(output.configs.rows * output.configs.columns);
        // output.data = vec![output.configs.nodata; output.configs.rows * output.configs.columns];
        for row in 0..output.configs.rows {
            for col in 0..output.configs.columns {
                output.data.push(data.get_value(row as isize, col as isize).as_());
            }
        }

        output
    }

    /// Creates a new in-memory `Raster` object with grid extent and location based
    /// on an existing `Raster` contained within `file_name`.
    pub fn initialize_using_file<'a>(file_name: &'a str, input: &'a Raster) -> Raster {
        let new_file_name = if file_name.contains(".") {
            file_name.to_string()
        } else {
            // likely no extension provided; default to .tif
            format!("{}.tif", file_name)
        };
        let mut output = Raster {
            file_name: new_file_name.clone(),
            ..Default::default()
        };
        output.file_mode = "w".to_string();
        output.raster_type = get_raster_type_from_file(new_file_name.clone(), "w".to_string());
        output.configs.rows = input.configs.rows;
        output.configs.columns = input.configs.columns;
        output.configs.north = input.configs.north;
        output.configs.south = input.configs.south;
        output.configs.east = input.configs.east;
        output.configs.west = input.configs.west;
        output.configs.resolution_x = input.configs.resolution_x;
        output.configs.resolution_y = input.configs.resolution_y;
        output.configs.nodata = input.configs.nodata;
        output.configs.data_type = input.configs.data_type;
        output.configs.photometric_interp = input.configs.photometric_interp;
        output.configs.palette = input.configs.palette.clone();
        output.configs.projection = input.configs.projection.clone();
        output.configs.xy_units = input.configs.xy_units.clone();
        output.configs.z_units = input.configs.z_units.clone();
        output.configs.endian = input.configs.endian.clone();
        // output.configs.palette_nonlinearity = input.configs.palette_nonlinearity;
        output.configs.pixel_is_area = input.configs.pixel_is_area;
        output.configs.epsg_code = input.configs.epsg_code;
        output.configs.coordinate_ref_system_wkt = input.configs.coordinate_ref_system_wkt.clone();
        output.configs.model_tiepoint = input.configs.model_tiepoint.clone();
        output.configs.model_pixel_scale = input.configs.model_pixel_scale.clone();
        output.configs.model_transformation = input.configs.model_transformation.clone();
        output.configs.geo_key_directory = input.configs.geo_key_directory.clone();
        output.configs.geo_double_params = input.configs.geo_double_params.clone();
        output.configs.geo_ascii_params = input.configs.geo_ascii_params.clone();

        if output.raster_type == RasterType::SurferAscii
            || output.raster_type == RasterType::Surfer7Binary
        {
            output.configs.nodata = 1.71041e38;
        }
        output
            .data
            .reserve(output.configs.rows * output.configs.columns);
        output.data = vec![output.configs.nodata; output.configs.rows * output.configs.columns];
        output
    }

    pub fn initialize_from_array2d<'a, T: Into<f64> + Copy + AddAssign + SubAssign>(
        file_name: &'a str,
        configs: &'a RasterConfigs,
        array: &'a Array2D<T>,
    ) -> Raster {
        let new_file_name = if file_name.contains(".") {
            file_name.to_string()
        } else {
            // likely no extension provided; default to .tif
            format!("{}.tif", file_name)
        };
        let mut output = Raster {
            file_name: new_file_name.clone(),
            ..Default::default()
        };
        if array.rows as usize != configs.rows || array.columns as usize != configs.columns {
            eprintln!("Warning: the Array2D and configs don't share the same dimensions. This may cause problems.");
        }
        output.file_mode = "w".to_string();
        output.raster_type = get_raster_type_from_file(new_file_name.clone(), "w".to_string());
        output.configs.rows = array.rows as usize;
        output.configs.columns = array.columns as usize;
        output.configs.north = configs.north;
        output.configs.south = configs.south;
        output.configs.east = configs.east;
        output.configs.west = configs.west;
        output.configs.resolution_x = configs.resolution_x;
        output.configs.resolution_y = configs.resolution_y;
        output.configs.nodata = array.nodata().into();
        output.configs.data_type = configs.data_type;
        output.configs.photometric_interp = configs.photometric_interp;
        output.configs.palette = configs.palette.clone();
        output.configs.projection = configs.projection.clone();
        output.configs.xy_units = configs.xy_units.clone();
        output.configs.z_units = configs.z_units.clone();
        output.configs.endian = configs.endian.clone();
        output.configs.pixel_is_area = configs.pixel_is_area;
        output.configs.epsg_code = configs.epsg_code;
        output.configs.coordinate_ref_system_wkt = configs.coordinate_ref_system_wkt.clone();
        output.configs.model_tiepoint = configs.model_tiepoint.clone();
        output.configs.model_pixel_scale = configs.model_pixel_scale.clone();
        output.configs.model_transformation = configs.model_transformation.clone();
        output.configs.geo_key_directory = configs.geo_key_directory.clone();
        output.configs.geo_double_params = configs.geo_double_params.clone();
        output.configs.geo_ascii_params = configs.geo_ascii_params.clone();

        if output.raster_type == RasterType::SurferAscii
            || output.raster_type == RasterType::Surfer7Binary
        {
            output.configs.nodata = 1.71041e38;
        }
        output
            .data
            .reserve_exact(output.configs.rows * output.configs.columns);
        for row in 0..array.rows {
            for col in 0..array.columns {
                output.data.push(array.get_value(row, col).into());
            }
        }
        output
    }

    /// Returns the file name of the `Raster`, without the directory and file extension.
    pub fn get_short_filename(&self) -> String {
        let path = Path::new(&self.file_name);
        let file_name = path.file_stem().unwrap();
        let f = file_name.to_str().unwrap();
        f.to_string()
    }

    /// Returns the file extension.
    pub fn get_file_extension(&self) -> String {
        let path = Path::new(&self.file_name);
        let extension = path.extension().unwrap();
        let e = extension.to_str().unwrap();
        e.to_string()
    }

    /// Returns the value contained within a grid cell specified
    /// by `row` and `column`.
    pub fn get_value(&self, row: isize, column: isize) -> f64 {
        // if row < 0 || column < 0 { return self.configs.nodata; }
        // if row as usize >= self.configs.rows || column as usize >= self.configs.columns { return self.configs.nodata; }
        // self.data[row as usize * self.configs.columns + column as usize]

        if column >= 0
            && row >= 0
            && column < self.configs.columns as isize
            && row < self.configs.rows as isize
        {
            let c: usize = column as usize;
            let r: usize = row as usize;

            let idx: usize = r * self.configs.columns + c;
            return self.data[idx];
        }

        // it's not within the area of the data
        if !self.configs.reflect_at_edges {
            return self.configs.nodata;
        }

        let mut c = column;
        let mut r = row;

        // if you get to this point, it should be reflected at the edges
        if r < 0 {
            r = -r - 1;
        }
        if r >= self.configs.rows as isize {
            r = self.configs.rows as isize - (r - self.configs.rows as isize) - 1;
        }
        if c < 0 {
            c = -c - 1;
        }
        if c >= self.configs.columns as isize {
            c = self.configs.columns as isize - (c - self.configs.columns as isize) - 1;
        }
        if c >= 0
            && c < self.configs.columns as isize
            && row >= 0
            && row < self.configs.rows as isize
        {
            return self.get_value(r, c);
        }

        // it was too off grid to be reflected.
        self.configs.nodata
    }

    pub fn set_value(&mut self, row: isize, column: isize, value: f64) {
        if column >= 0 && row >= 0 {
            let c: usize = column as usize;
            let r: usize = row as usize;
            if c < self.configs.columns && r < self.configs.rows {
                let idx = r * self.configs.columns + c;
                self.data[idx] = value;
            }
        }
    }

    pub fn decrement(&mut self, row: isize, column: isize, value: f64) {
        if column >= 0 && row >= 0 {
            let c: usize = column as usize;
            let r: usize = row as usize;
            if c < self.configs.columns && r < self.configs.rows {
                let idx = r * self.configs.columns + c;
                if self.data[idx] != self.configs.nodata {
                    self.data[idx] -= value;
                } else {
                    self.data[idx] = value;
                }
            }
        }
    }

    pub fn increment(&mut self, row: isize, column: isize, value: f64) {
        if column >= 0 && row >= 0 {
            let c: usize = column as usize;
            let r: usize = row as usize;
            if c < self.configs.columns && r < self.configs.rows {
                let idx = r * self.configs.columns + c;
                if self.data[idx] != self.configs.nodata {
                    self.data[idx] += value;
                } else {
                    self.data[idx] = value;
                }
            }
        }
    }

    pub fn set_row_data(&mut self, row: isize, values: Vec<f64>) {
        for column in 0..values.len() {
            if row >= 0 {
                let c: usize = column as usize;
                let r: usize = row as usize;
                if c < self.configs.columns && r < self.configs.rows {
                    let idx = r * self.configs.columns + c;
                    self.data[idx] = values[c];
                }
            }
        }
    }

    pub fn get_row_data(&self, row: isize) -> Vec<f64> {
        let mut values: Vec<f64> = vec![self.configs.nodata; self.configs.columns];
        if row >= 0 && row < self.configs.rows as isize {
            for column in 0..values.len() {
                values[column] = self.data[row as usize * self.configs.columns + column];
            }
        }
        values
    }

    pub fn increment_row_data(&mut self, row: isize, values: Vec<f64>) {
        assert!(values.len() == self.configs.columns);
        if row < 0 {
            return;
        }
        let r = row as usize;
        if r >= self.configs.rows {
            return;
        }
        for column in 0..self.configs.columns {
            let idx = r * self.configs.columns + column;
            if self.data[idx] != self.configs.nodata {
                self.data[idx] += values[column];
            } else {
                self.data[idx] = values[column];
            }
        }
    }

    pub fn decrement_row_data(&mut self, row: isize, values: Vec<f64>) {
        assert!(values.len() == self.configs.columns);
        if row < 0 {
            return;
        }
        let r = row as usize;
        if r >= self.configs.rows {
            return;
        }
        for column in 0..self.configs.columns {
            let idx = r * self.configs.columns + column;
            if self.data[idx] != self.configs.nodata {
                self.data[idx] -= values[column];
            } else {
                self.data[idx] = values[column];
            }
        }
    }

    pub fn set_data_from_raster(&mut self, other: &Raster) -> Result<(), Error> {
        if self.configs.rows != other.configs.rows || self.configs.columns != other.configs.columns
        {
            return Err(Error::new(
                ErrorKind::Other,
                "Rasters must have the same dimensions and extent.",
            ));
        }
        for row in 0..self.configs.rows as isize {
            self.set_row_data(row, other.get_row_data(row));
        }
        Ok(())
    }

    pub fn get_data_as_array2d(&self) -> Array2D<f64> {
        let mut data: Array2D<f64> = Array2D::new(
            self.configs.rows as isize,
            self.configs.columns as isize,
            self.configs.nodata,
            self.configs.nodata,
        )
        .unwrap();
        for row in 0..self.configs.rows as isize {
            data.set_row_data(row, self.get_row_data(row));
        }
        data
    }

    pub fn get_data_as_f32_array2d(&self) -> Array2D<f32> {
        let out_nodata = self.configs.nodata as f32; 
        let mut data: Array2D<f32> = Array2D::new(
            self.configs.rows as isize,
            self.configs.columns as isize,
            out_nodata,
            out_nodata,
        )
        .unwrap();
        let mut z: f64;
        for row in 0..self.configs.rows as isize {
            for col in 0..self.configs.columns as isize {
                z = self.get_value(row, col);
                if z != self.configs.nodata {
                    data.set_value(row, col, z as f32);
                }
            }
        }
        data
    }

    pub fn set_data_from_array2d<'a, T: Into<f64> + Copy + AddAssign + SubAssign>(
        &mut self,
        array: &'a Array2D<T>,
    ) -> Result<(), Error> {
        // quality control
        if array.rows * array.columns != self.data.len() as isize {
            return Err(Error::new(
                ErrorKind::Other,
                "Rasters must have the same dimensions and extent.",
            ));
        }
        let mut i: usize;
        for row in 0..array.rows {
            for col in 0..array.columns {
                i = row as usize * self.configs.columns + col as usize;
                self.data[i] = array.get_value(row, col).into();
            }
        }
        self.configs.nodata = array.nodata().into();
        Ok(())
    }

    pub fn reinitialize_values(&mut self, value: f64) {
        self.data = vec![value; self.configs.rows * self.configs.columns];
    }

    pub fn get_value_as_rgba(&self, row: isize, column: isize) -> (u8, u8, u8, u8) {
        if column < 0 {
            return (0, 0, 0, 0); //self.configs.nodata;
        }
        if row < 0 {
            return (0, 0, 0, 0); //return self.configs.nodata;
        }

        let c: usize = column as usize;
        let r: usize = row as usize;

        if c >= self.configs.columns {
            return (0, 0, 0, 0);
        }
        if r >= self.configs.rows {
            return (0, 0, 0, 0);
        }
        let idx: usize = r * self.configs.columns + c;
        let z = self.data[idx];

        let r = (z as u32 & 0xFF) as u8;
        let g = ((z as u32 >> 8) & 0xFF) as u8;
        let b = ((z as u32 >> 16) & 0xFF) as u8;
        let a = ((z as u32 >> 24) & 0xFF) as u8;

        (r, g, b, a)
    }

    pub fn set_value_from_rgba(&mut self, row: isize, column: isize, rgba: (u32, u32, u32, u32)) {
        if column >= 0 && row >= 0 {
            let c: usize = column as usize;
            let r: usize = row as usize;
            if c < self.configs.columns && r < self.configs.rows {
                let idx = r * self.configs.columns + c;
                let (r, g, b, a) = rgba;
                self.data[idx] += ((a << 24) | (b << 16) | (g << 8) | r) as f64;
            }
        }
    }

    /// Returns the size of the pixel data in bytes.
    pub fn get_data_size_in_bytes(&self) -> usize {
        use std::mem;
        mem::size_of_val(&*self.data)
    }

    pub fn get_x_from_column(&self, column: isize) -> f64 {
        // self.configs.west - self.configs.resolution_x / 2f64 +
        // column as f64 * self.configs.resolution_x
        // Not sure why it must be + 1/2 resolution rather than minus
        self.configs.west
            + self.configs.resolution_x / 2f64
            + column as f64 * self.configs.resolution_x
    }

    pub fn get_y_from_row(&self, row: isize) -> f64 {
        self.configs.north
            - self.configs.resolution_y / 2f64
            - row as f64 * self.configs.resolution_y
    }

    pub fn get_column_from_x(&self, x: f64) -> isize {
        ((x - self.configs.west) / self.configs.resolution_x).floor() as isize
    }

    pub fn get_row_from_y(&self, y: f64) -> isize {
        ((self.configs.north - y) / self.configs.resolution_y).floor() as isize
    }

    pub fn clip_display_min_max(&mut self, percent: f64) {
        let t = (percent / 100.0 * (self.configs.rows * self.configs.columns) as f64) as usize;
        let mut d = self.data.clone();
        d.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
        let mut sum = 0;
        for i in 0..d.len() {
            if d[i] != self.configs.nodata {
                sum += 1;
                if sum >= t {
                    self.configs.display_min = d[i];
                    break;
                }
            }
        }

        sum = 0;
        for i in (0..d.len()).rev() {
            if d[i] != self.configs.nodata {
                sum += 1;
                if sum >= t {
                    self.configs.display_max = d[i];
                    break;
                }
            }
        }
    }

    pub fn clip_display_min(&mut self, percent: f64) {
        let t = (percent / 100.0 * (self.configs.rows * self.configs.columns) as f64) as usize;
        let mut d = self.data.clone();
        d.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
        let mut sum = 0;
        for i in 0..d.len() {
            if d[i] != self.configs.nodata {
                sum += 1;
                if sum >= t {
                    self.configs.display_min = d[i];
                    break;
                }
            }
        }
    }

    pub fn clip_display_max(&mut self, percent: f64) {
        let t = (percent / 100.0 * (self.configs.rows * self.configs.columns) as f64) as usize;
        let mut d = self.data.clone();
        d.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
        let mut sum = 0;
        for i in (0..d.len()).rev() {
            if d[i] != self.configs.nodata {
                sum += 1;
                if sum >= t {
                    self.configs.display_max = d[i];
                    break;
                }
            }
        }
        // for value in &self.data {
        //     if *value < self.configs.minimum && *value != self.configs.nodata {
        //         self.configs.minimum = *value;
        //     }
        //     if *value > self.configs.maximum && *value != self.configs.nodata {
        //         self.configs.maximum = *value;
        //     }
        // }
        // let mut histo: [usize; 1025] = [0; 1025];
        // let mut bin: isize;
        // for value in &self.data {
        //     if *value != self.configs.nodata {
        //         bin = ((*value - self.configs.minimum) / 1025.0).floor() as isize;
        //         if bin > 1024 {
        //             bin = 1024;
        //         }
        //         if bin < 0 {
        //             bin = 0;
        //         }
        //         histo[bin as usize] += 1;
        //     }
        // }

        // let bin_size = (self.configs.maximum - self.configs.minimum) / 1025.0;
        // let mut sum = 0;
        // for i in (0..1025).rev() {
        //     sum += histo[i];
        //     if sum == t {
        //         self.configs.display_max = bin_size * i as f64 + self.configs.minimum;
        //         break;
        //     } else if sum > t {
        //         self.configs.display_max = bin_size * (i + 1) as f64 + self.configs.minimum;
        //         println!("i = {}; disp max = {}", i, self.configs.display_max);
        //         break;
        //     }
        // }
    }

    pub fn clip_min_by_percent(&mut self, percent: f64) {
        let t = (percent / 100.0 * (self.configs.rows * self.configs.columns) as f64) as usize;
        let mut d = self.data.clone();
        d.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
        let mut sum = 0;
        let mut val = 0.0;
        for i in 0..self.num_cells() {
            if d[i] != self.configs.nodata {
                sum += 1;
                if sum >= t {
                    val = d[i];
                    break;
                }
            }
        }

        for i in 0..self.data.len() {
            if self.data[i] != self.configs.nodata {
                if self.data[i] < val {
                    self.data[i] = val;
                }
            }
        }

        self.configs.display_min = val;
    }

    pub fn clip_max_by_percent(&mut self, percent: f64) {
        let t = (percent / 100.0 * (self.configs.rows * self.configs.columns) as f64) as usize;
        let mut d = self.data.clone();
        d.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
        let mut sum = 0;
        let mut val = 0.0;
        for i in (0..self.num_cells()).rev() {
            if d[i] != self.configs.nodata {
                sum += 1;
                if sum >= t {
                    val = d[i];
                    break;
                }
            }
        }

        for i in 0..self.data.len() {
            if self.data[i] != self.configs.nodata {
                if self.data[i] > val {
                    self.data[i] = val;
                }
            }
        }

        self.configs.display_max = val;
    }

    pub fn clip_min_and_max_by_percent(&mut self, percent: f64) {
        let t = (percent / 100.0 * (self.configs.rows * self.configs.columns) as f64) as usize;
        let mut d = self.data.clone();
        d.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
        let mut sum = 0;
        let mut lower_val = 0.0;
        for i in 0..self.num_cells() {
            if d[i] != self.configs.nodata {
                sum += 1;
                if sum >= t {
                    lower_val = d[i];
                    break;
                }
            }
        }

        let mut upper_val = 0.0;
        let mut sum = 0;
        for i in (0..self.num_cells()).rev() {
            if d[i] != self.configs.nodata {
                sum += 1;
                if sum >= t {
                    upper_val = d[i];
                    break;
                }
            }
        }

        for i in 0..self.data.len() {
            if self.data[i] != self.configs.nodata {
                if self.data[i] < lower_val {
                    self.data[i] = lower_val;
                } else if self.data[i] > upper_val {
                    self.data[i] = upper_val;
                }
            }
        }

        self.configs.display_min = lower_val;
        self.configs.display_max = upper_val;
    }

    pub fn update_min_max(&mut self) {
        self.configs.minimum = f64::INFINITY;
        self.configs.maximum = f64::NEG_INFINITY;
        let num_procs = num_cpus::get();
        let nodata = self.configs.nodata;
        let values = Arc::new(self.data.clone());
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let values = values.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut min_val = f64::INFINITY;
                let mut max_val = f64::NEG_INFINITY;
                let mut value: f64;
                for i in (0..values.len()).filter(|v| v % num_procs == tid) {
                    value = values[i];
                    if value != nodata {
                        if value < min_val {
                            min_val = value;
                        }
                        if value > max_val {
                            max_val = value;
                        }
                    }
                }
                tx.send((min_val, max_val)).unwrap();
            });
        }

        for _ in 0..num_procs {
            let (min_val, max_val) = rx.recv().expect("Error receiving data from thread.");
            if min_val != nodata {
                if min_val < self.configs.minimum {
                    self.configs.minimum = min_val;
                }
            }
            if max_val != nodata {
                if max_val > self.configs.maximum {
                    self.configs.maximum = max_val;
                }
            }
        }

        if self.configs.display_min == f64::INFINITY {
            self.configs.display_min = self.configs.minimum;
        }
        if self.configs.display_max == f64::NEG_INFINITY {
            self.configs.display_max = self.configs.maximum;
        }
    }

    pub fn update_display_min_max(&mut self) {
        self.configs.display_min = self.configs.minimum;
        self.configs.display_max = self.configs.maximum;
    }

    pub fn num_cells(&self) -> usize {
        self.configs.rows * self.configs.columns
    }

    pub fn num_valid_cells(&self) -> usize {
        if self.data.len() == 0 {
            return 0usize;
        }
        let nodata = self.configs.nodata;
        let values = Arc::new(self.data.clone());
        let num_procs = num_cpus::get();
        let num_cells = self.num_cells();
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let values = values.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut count = 0usize;
                for i in (0..num_cells).filter(|r| r % num_procs == tid) {
                    if values[i] != nodata {
                        count += 1;
                    }
                }
                tx.send(count).unwrap();
            });
        }

        let mut count = 0usize;
        for _ in 0..num_procs {
            count += rx.recv().expect("Error receiving data from thread.");
        }

        count
    }

    pub fn calculate_mean(&self) -> f64 {
        if self.data.len() == 0 {
            return 0.0;
        }
        let nodata = self.configs.nodata;
        let values = Arc::new(self.data.clone());
        let num_procs = num_cpus::get();
        let num_cells = self.num_cells();
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let values = values.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut sum = 0.0f64;
                let mut count = 0.0f64;
                for i in (0..num_cells).filter(|r| r % num_procs == tid) {
                    if values[i] != nodata {
                        sum += values[i];
                        count += 1.0;
                    }
                }
                tx.send((sum, count)).unwrap();
            });
        }

        let mut sum = 0.0f64;
        let mut count = 0.0f64;
        for _ in 0..num_procs {
            let (s, c) = rx.recv().expect("Error receiving data from thread.");
            sum += s;
            count += c;
        }

        sum / count
    }

    pub fn calculate_mean_and_stdev(&self) -> (f64, f64) {
        if self.data.len() == 0 {
            return (0.0, 0.0);
        }

        let mean = self.calculate_mean();
        let nodata = self.configs.nodata;
        let values = Arc::new(self.data.clone());
        let num_procs = num_cpus::get();
        let num_cells = self.num_cells();
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let values = values.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut sq_diff_sum = 0.0f64;
                let mut count = 0.0f64;
                for i in (0..num_cells).filter(|r| r % num_procs == tid) {
                    if values[i] != nodata {
                        sq_diff_sum += (values[i] - mean) * (values[i] - mean);
                        count += 1.0;
                    }
                    tx.send((sq_diff_sum, count)).unwrap();
                }
            });
        }

        let mut sq_diff_sum = 0.0f64;
        let mut count = 0.0f64;
        for _ in 0..num_cells {
            let (s, c) = rx.recv().expect("Error receiving data from thread.");
            sq_diff_sum += s;
            count += c;
        }

        (mean, (sq_diff_sum / count).sqrt())
    }

    pub fn calculate_clip_values(&self, percent: f64) -> (f64, f64) {
        let t = (percent / 100.0 * (self.configs.rows * self.configs.columns) as f64) as usize;
        let mut lower_tail = f64::NEG_INFINITY;
        let mut upper_tail = f64::NEG_INFINITY;
        let mut d = self.data.clone();
        d.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
        let mut sum = 0;
        for i in 0..d.len() {
            if d[i] != self.configs.nodata {
                sum += 1;
                if sum >= t {
                    lower_tail = d[i];
                    break;
                }
            }
        }

        sum = 0;
        for i in (0..d.len()).rev() {
            if d[i] != self.configs.nodata {
                sum += 1;
                if sum >= t {
                    upper_tail = d[i];
                    break;
                }
            }
        }

        (lower_tail, upper_tail)
    }

    pub fn write(&mut self) -> Result<(), Error> {
        if !self.file_mode.contains("w") {
            return Err(Error::new(
                ErrorKind::Other,
                "Cannot write raster that is not created in write mmode ('w').",
            ));
        }
        match self.raster_type {
            RasterType::ArcAscii => {
                let _ = match write_arcascii(self) {
                    Ok(_) => (),
                    Err(e) => println!("error while writing: {:?}", e),
                };
            }
            RasterType::ArcBinary => {
                let _ = match write_arcbinary(self) {
                    Ok(_) => (),
                    Err(e) => println!("error while writing: {:?}", e),
                };
            }
            RasterType::EsriBil => {
                let _ = match write_esri_bil(self) {
                    Ok(_) => (),
                    Err(e) => println!("error while writing: {:?}", e),
                };
            }
            RasterType::GeoTiff => {
                let _ = match write_geotiff(self) {
                    Ok(_) => (),
                    Err(e) => println!("error while writing: {:?}", e),
                };
            }
            RasterType::GrassAscii => {
                let _ = match write_grass_raster(self) {
                    Ok(_) => (),
                    Err(e) => println!("error while writing: {:?}", e),
                };
            }
            RasterType::IdrisiBinary => {
                let _ = match write_idrisi(self) {
                    Ok(_) => (),
                    Err(e) => println!("error while writing: {:?}", e),
                };
            }
            RasterType::SagaBinary => {
                let _ = match write_saga(self) {
                    Ok(_) => (),
                    Err(e) => println!("error while writing: {:?}", e),
                };
            }
            RasterType::Surfer7Binary => {
                let _ = match write_surfer7(self) {
                    Ok(_) => (),
                    Err(e) => println!("error while writing: {:?}", e),
                };
            }
            RasterType::SurferAscii => {
                let _ = match write_surfer_ascii_raster(self) {
                    Ok(_) => (),
                    Err(e) => println!("error while writing: {:?}", e),
                };
            }
            RasterType::Whitebox => {
                let _ = match write_whitebox(self) {
                    Ok(_) => (),
                    Err(e) => println!("error while writing: {:?}", e),
                };
            }
            RasterType::Unknown => {
                return Err(Error::new(ErrorKind::Other, "Unrecognized raster type"));
            }
        }
        Ok(())
    }

    pub fn add_metadata_entry(&mut self, value: String) {
        self.configs.metadata.push(value);
    }

    pub fn get_metadata_entry(&self, idx: usize) -> String {
        if idx < self.configs.metadata.len() {
            return self.configs.metadata.get(idx).unwrap().clone();
        }
        String::new()
    }

    pub fn get_bounding_box(&self) -> BoundingBox {
        BoundingBox::new(
            self.configs.west,
            self.configs.east,
            self.configs.south,
            self.configs.north,
        )
    }

    pub fn is_in_geographic_coordinates(&self) -> bool {
        if self.configs.west < -180f64
            || self.configs.east > 180f64
            || self.configs.north > 90f64
            || self.configs.south < -90f64
        {
            return false;
        }
        if self.configs.epsg_code == 4322
            || self.configs.epsg_code == 4326
            || self.configs.epsg_code == 4629
            || self.configs.epsg_code == 4277
        {
            return true;
        }
        let wkt = self.configs.coordinate_ref_system_wkt.to_lowercase();
        if !wkt.contains("projcs[") && !wkt.to_lowercase().contains("not specified") {
            return true;
        }
        if self.configs.xy_units.to_lowercase().contains("deg") {
            return true;
        }
        false
    }
}

#[derive(Debug, Clone)]
pub struct RasterConfigs {
    pub title: String,
    pub rows: usize,
    pub columns: usize,
    pub bands: u8,
    pub nodata: f64,
    pub north: f64,
    pub south: f64,
    pub east: f64,
    pub west: f64,
    pub resolution_x: f64,
    pub resolution_y: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub display_min: f64,
    pub display_max: f64,
    pub palette: String,
    pub projection: String,
    pub endian: Endianness,
    pub photometric_interp: PhotometricInterpretation,
    pub data_type: DataType,
    pub palette_nonlinearity: f64,
    pub z_units: String,
    pub xy_units: String,
    pub reflect_at_edges: bool,
    pub pixel_is_area: bool,
    pub epsg_code: u16,
    pub coordinate_ref_system_wkt: String,
    pub model_tiepoint: Vec<f64>,
    pub model_pixel_scale: [f64; 3],
    pub model_transformation: [f64; 16],
    pub geo_key_directory: Vec<u16>,
    pub geo_double_params: Vec<f64>,
    pub geo_ascii_params: String,
    pub metadata: Vec<String>,
}

impl Default for RasterConfigs {
    fn default() -> RasterConfigs {
        RasterConfigs {
            title: String::from(""),
            bands: 1,
            rows: 0,
            columns: 0,
            nodata: -32768.0,
            north: f64::NEG_INFINITY,
            south: f64::INFINITY,
            east: f64::NEG_INFINITY,
            west: f64::INFINITY,
            resolution_x: f64::NEG_INFINITY,
            resolution_y: f64::NEG_INFINITY,
            minimum: f64::INFINITY,
            maximum: f64::NEG_INFINITY,
            display_min: f64::INFINITY,
            display_max: f64::NEG_INFINITY,
            palette: "not specified".to_string(),
            projection: "not specified".to_string(),
            endian: Endianness::LittleEndian,
            photometric_interp: PhotometricInterpretation::Unknown,
            data_type: DataType::Unknown,
            palette_nonlinearity: 1.0,
            z_units: "not specified".to_string(),
            xy_units: "not specified".to_string(),
            reflect_at_edges: false,
            pixel_is_area: true,
            epsg_code: 0u16,
            coordinate_ref_system_wkt: "not specified".to_string(),
            model_tiepoint: vec![],
            model_pixel_scale: [0f64; 3],
            model_transformation: [0f64; 16],
            geo_key_directory: vec![],
            geo_double_params: vec![],
            geo_ascii_params: String::new(),
            metadata: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RasterType {
    Unknown,
    ArcAscii,
    ArcBinary,
    EsriBil,
    GeoTiff,
    GrassAscii,
    IdrisiBinary,
    SagaBinary,
    Surfer7Binary,
    SurferAscii,
    Whitebox,
}

impl Default for RasterType {
    fn default() -> RasterType {
        RasterType::Unknown
    }
}

fn get_raster_type_from_file(file_name: String, file_mode: String) -> RasterType {
    // get the file extension
    let extension: String = match Path::new(&file_name).extension().unwrap().to_str() {
        Some(n) => n.to_string().to_lowercase(),
        None => "".to_string(),
    };
    if extension.is_empty() {
        panic!(
            "The file type could not be determined for the file:\n{}\n due to missing extension.",
            file_name
        );
    }
    if extension == "tas" || extension == "dep" {
        return RasterType::Whitebox;
    } else if extension == "tif"
        || extension == "tiff"
        || extension == "gtif"
        || extension == "gtiff"
    {
        return RasterType::GeoTiff;
    } else if extension == "bil" {
        return RasterType::EsriBil;
    } else if extension == "flt" {
        return RasterType::ArcBinary;
    } else if extension == "rdc" || extension == "rst" {
        return RasterType::IdrisiBinary;
    } else if extension == "sdat" || extension == "sgrd" {
        return RasterType::SagaBinary;
    } else if extension == "grd" {
        if file_mode == "r" {
            // It could be a SurferAscii or a Surfer7Binary.
            let mut f = File::open(file_name).unwrap();
            let mut buffer = [0; 4];
            f.read_exact(&mut buffer).unwrap();
            //let small_chunk = String::from_utf8_lossy(&buffer[0..8]).to_string();
            //if small_chunk.contains("DSAA") {
            if buffer[0] == 68 && buffer[1] == 83 && buffer[2] == 65 && buffer[3] == 65 {
                // DSAA signature
                return RasterType::SurferAscii;
            } else {
                return RasterType::Surfer7Binary;
            }
        }
        return RasterType::Surfer7Binary;
    } else if extension == "asc" || extension == "txt" || extension == "" {
        // what mode is this raster in?
        if file_mode == "r" {
            // It could be an ArcAscii or a GrassAscii.
            let f = File::open(file_name).unwrap();
            let file = BufReader::new(&f);
            let mut line_count = 0;
            for line in file.lines() {
                let l = line.unwrap();
                if l.contains("north")
                    || l.contains("south")
                    || l.contains("east")
                    || l.contains("west")
                {
                    return RasterType::GrassAscii;
                }
                if l.contains("xllcorner")
                    || l.contains("yllcorner")
                    || l.contains("xllcenter")
                    || l.contains("yllcenter")
                {
                    return RasterType::ArcAscii;
                }
                if line_count > 7 {
                    break;
                }
                line_count += 1;
            }
        }
        // For a file_mode "w", there is not way of knowing if it is an Arc or GRASS ASCII raster.
        // Default to ArcAscii.
        return RasterType::ArcAscii;
    }

    RasterType::Unknown
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DataType {
    F64,
    F32,
    I64,
    I32,
    I16,
    I8,
    U64,
    U32,
    U16,
    U8,
    RGB24,
    RGB48,
    RGBA32,
    Unknown,
}

impl Default for DataType {
    fn default() -> DataType {
        DataType::Unknown
    }
}

impl DataType {
    pub fn get_data_size(&self) -> usize {
        match *self {
            DataType::F64 => 8usize,
            DataType::F32 => 4usize,
            DataType::I64 => 8usize,
            DataType::I32 => 4usize,
            DataType::I16 => 2usize,
            DataType::I8 => 1usize,
            DataType::U64 => 8usize,
            DataType::U32 => 4usize,
            DataType::U16 => 2usize,
            DataType::U8 => 1usize,
            DataType::RGB24 => 3usize,
            DataType::RGB48 => 6usize,
            DataType::RGBA32 => 4usize,
            DataType::Unknown => 0usize,
        }
    }

    pub fn is_float(&self) -> bool {
        match *self {
            DataType::F64 => true,
            DataType::F32 => true,
            _ => false
        }
    }

    pub fn is_integer(&self) -> bool {
        match *self {
            DataType::U64 => true,
            DataType::U32 => true,
            DataType::U16 => true,
            DataType::U8 => true,
            DataType::I64 => true,
            DataType::I32 => true,
            DataType::I16 => true,
            DataType::I8 => true,
            _ => false
        }
    }

    pub fn is_unsigned_integer(&self) -> bool {
        match *self {
            DataType::U64 => true,
            DataType::U32 => true,
            DataType::U16 => true,
            DataType::U8 => true,
            _ => false
        }
    }

    pub fn is_signed_integer(&self) -> bool {
        match *self {
            DataType::I64 => true,
            DataType::I32 => true,
            DataType::I16 => true,
            DataType::I8 => true,
            _ => false
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PhotometricInterpretation {
    Continuous,
    Categorical,
    Boolean,
    RGB,
    Paletted,
    Unknown,
}

impl Default for PhotometricInterpretation {
    fn default() -> PhotometricInterpretation {
        PhotometricInterpretation::Unknown
    }
}
