/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 25/12/2019
Last Modified: 25/12/2019
License: MIT
*/

use crate::raster::*;
use crate::tools::*;
use crate::vector::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// Converts a raster dataset to a vector of the POLYGON shapetype. The user must specify
/// the name of a raster file and the name of the output vector. All grid cells containing 
/// non-zero, non-NoData values will be considered a point. The vector's attribute table 
/// will contain a field called 'VALUE' that will contain the cell value for each point 
/// feature.
pub struct RasterToVectorPolygons {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl RasterToVectorPolygons {
    pub fn new() -> RasterToVectorPolygons {
        // public constructor
        let name = "RasterToVectorPolygons".to_string();
        let toolbox = "Data Tools".to_string();
        let description =
            "Converts a raster dataset to a vector of the POLYGON shapetype.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Raster File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Points File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector points file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
            )),
            default_value: None,
            optional: false,
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e
            .replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input=points.tif -o=out.shp",
            short_exe, name
        )
        .replace("*", &sep);

        RasterToVectorPolygons {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for RasterToVectorPolygons {
    fn get_source_file(&self) -> String {
        String::from(file!())
    }

    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        let mut s = String::from("{\"parameters\": [");
        for i in 0..self.parameters.len() {
            if i < self.parameters.len() - 1 {
                s.push_str(&(self.parameters[i].to_string()));
                s.push_str(",");
            } else {
                s.push_str(&(self.parameters[i].to_string()));
            }
        }
        s.push_str("]}");
        s
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn get_toolbox(&self) -> String {
        self.toolbox.clone()
    }

    fn run<'a>(
        &self,
        args: Vec<String>,
        working_directory: &'a str,
        verbose: bool,
    ) -> Result<(), Error> {


        /*  Diagram 1: 
         *  Cell Numbering
         *  _____________
         *  |     |     |
         *  |  0  |  1  |
         *  |_____|_____|
         *  |     |     |
         *  |  2  |  3  |
         *  |_____|_____|
         * 
         */

        /*  Diagram 2:
         *  Edge Numbering (shared edges between cells)
         *  _____________
         *  |     |     |
         *  |     3     |
         *  |__2__|__0__|
         *  |     |     |
         *  |     1     |
         *  |_____|_____|
         * 
         */

        /* Diagram 3:
         * Cell Edge Numbering
         * 
         *  ___0___
         * |       |
         * |       |
         * 3       1
         * |       |
         * |___2___|
         * 
         */

        let mut input_file = String::new();
        let mut output_file = String::new();

        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no parameters.",
            ));
        }
        for i in 0..args.len() {
            let mut arg = args[i].replace("\"", "");
            arg = arg.replace("\'", "");
            let cmd = arg.split("="); // in case an equals sign was used
            let vec = cmd.collect::<Vec<&str>>();
            let mut keyval = false;
            if vec.len() > 1 {
                keyval = true;
            }
            let flag_val = vec[0].to_lowercase().replace("--", "-");
            if flag_val == "-i" || flag_val == "-input" {
                input_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            }
        }

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Raster::new(&input_file, "r")?;

        let start = Instant::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let res_x = input.configs.resolution_x;
        let res_y = input.configs.resolution_y;
        let east = input.configs.east;
        let west = input.configs.west;
        let ew_range = east - west;
        let north = input.configs.north;
        let south = input.configs.south;
        let ns_range = north - south;

        let mut output = Shapefile::new(&output_file, ShapeType::Polygon)?;

        // set the projection information
        output.projection = input.configs.coordinate_ref_system_wkt.clone();

        // add the attributes
        output.attributes.add_field(
            &AttributeField::new("FID", FieldDataType::Int, 10u8, 0u8)
        );
        output.attributes.add_field(
            &AttributeField::new("VALUE", FieldDataType::Real, 12u8, 4u8)
        );

        
        
        // clump the input raster
        // updateProgress("Clumping raster, please wait...", 0);
        // Clump clump = new Clump(input1, false, true);
        // clump.setOutputHeader(input1.getHeaderFile().replace(".dep", "_clumped.dep"));
        // WhiteboxRaster input = clump.run();
        // input.isTemporaryFile = true;
        
//         int numRegions = (int)input.getMaximumValue() + 1;
//         double[] zValues = new double[numRegions];
        
        
//         // create a temporary raster image.
//         String tempHeader1 = inputFile.replace(".dep", "_temp1.dep");
//         WhiteboxRaster temp1 = new WhiteboxRaster(tempHeader1, "rw", inputFile, WhiteboxRaster.DataType.INTEGER, 0);
//         temp1.isTemporaryFile = true;

//         GeometryFactory factory = new GeometryFactory();
//         List<com.vividsolutions.jts.geom.Polygon> polyList = new ArrayList<>();
//         List<Integer> regionValues = new ArrayList<>();

//         int[] parts;

//         oldProgress = -1;
//         for (row = 0; row < rows; row++) {
//             for (col = 0; col < cols; col++) {
//                 z = input.getValue(row, col);
//                 if (z > 0 && z != noData) {
//                     int region = (int)z;
//                     zValues[region] = input1.getValue(row, col);
                    
//                     zN1 = input.getValue(row - 1, col);
//                     zN2 = input.getValue(row, col - 1);

//                     if (zN1 != z || zN2 != z) {
//                         flag = false;
//                         if (zN1 != z) {
//                             i = (int) temp1.getValue(row, col);
//                             if (!BitOps.checkBit(i, 0)) {
//                                 flag = true;
//                             }
//                         }
//                         if (zN2 != z) {
//                             i = (int) temp1.getValue(row, col);
//                             if (!BitOps.checkBit(i, 3)) {
//                                 flag = true;
//                             }
//                         }
//                         if (flag) {

//                             currentHalfRow = row - 0.5;
//                             currentHalfCol = col - 0.5;

//                             traceDirection = -1;

//                             numPoints = 0;
//                             FID++;
//                             PointsList points = new PointsList();

//                             do {

//                                 // Get the data for the 2 x 2 
//                                 // window, i.e. the window in Diagram 1 above.
//                                 rowVals[0] = (int) Math.floor(currentHalfRow);
//                                 rowVals[1] = (int) Math.ceil(currentHalfRow);
//                                 colVals[0] = (int) Math.floor(currentHalfCol);
//                                 colVals[1] = (int) Math.ceil(currentHalfCol);

//                                 inputValueData[0] = input.getValue(rowVals[0], colVals[0]);
//                                 inputValueData[1] = input.getValue(rowVals[0], colVals[1]);
//                                 inputValueData[2] = input.getValue(rowVals[1], colVals[0]);
//                                 inputValueData[3] = input.getValue(rowVals[1], colVals[1]);

//                                 previousTraceDirection = traceDirection;
//                                 traceDirection = -1;

//                                 // The scan order is used to prefer accute angles during 
//                                 // the vectorizing. This is important for reducing the
//                                 // occurance of bow-tie or figure-8 (self-intersecting) polygons.
//                                 byte[] scanOrder = new byte[4];
//                                 switch (previousTraceDirection) {
//                                     case 0:
//                                         scanOrder = new byte[]{3, 1, 2, 0};
//                                         break;
//                                     case 1:
//                                         scanOrder = new byte[]{0, 2, 3, 1};
//                                         break;
//                                     case 2:
//                                         scanOrder = new byte[]{3, 1, 0, 2};
//                                         break;
//                                     case 3:
//                                         scanOrder = new byte[]{2, 0, 1, 3};
//                                         break;
//                                 }

//                                 for (int a = 0; a < 4; a++) {
//                                     switch (scanOrder[a]) {
//                                         case 0:
//                                             // traceDirection 0
//                                             if (inputValueData[1] != inputValueData[3]
//                                                     && inputValueData[1] == z) {
//                                                 // has the bottom edge of the top-right cell been traversed?
//                                                 i = (int) temp1.getValue(rowVals[0], colVals[1]);
//                                                 if (!BitOps.checkBit(i, 2)) {
//                                                     temp1.setValue(rowVals[0], colVals[1], BitOps.setBit(i, 2));
//                                                     traceDirection = 0;
//                                                 }
//                                             }

//                                             if (inputValueData[1] != inputValueData[3]
//                                                     && inputValueData[3] == z) {
//                                                 // has the top edge of the bottom-right cell been traversed?
//                                                 i = (int) temp1.getValue(rowVals[1], colVals[1]);
//                                                 if (!BitOps.checkBit(i, 0)) {
//                                                     temp1.setValue(rowVals[1], colVals[1], BitOps.setBit(i, 0));
//                                                     traceDirection = 0;
//                                                 }
//                                             }
//                                             break;

//                                         case 1:
//                                             // traceDirection 1
//                                             if (inputValueData[2] != inputValueData[3]
//                                                     && inputValueData[2] == z) {
//                                                 // has the right edge of the bottom-left cell been traversed?
//                                                 i = (int) temp1.getValue(rowVals[1], colVals[0]);
//                                                 if (!BitOps.checkBit(i, 1)) {
//                                                     temp1.setValue(rowVals[1], colVals[0], BitOps.setBit(i, 1));
//                                                     traceDirection = 1;
//                                                 }
//                                             }

//                                             if (inputValueData[2] != inputValueData[3]
//                                                     && inputValueData[3] == z) {
//                                                 // has the left edge of the bottom-right cell been traversed?
//                                                 i = (int) temp1.getValue(rowVals[1], colVals[1]);
//                                                 if (!BitOps.checkBit(i, 3)) {
//                                                     temp1.setValue(rowVals[1], colVals[1], BitOps.setBit(i, 3));
//                                                     traceDirection = 1;
//                                                 }
//                                             }
//                                             break;

//                                         case 2:
//                                             // traceDirection 2
//                                             if (inputValueData[0] != inputValueData[2]
//                                                     && inputValueData[0] == z) {
//                                                 // has the bottom edge of the top-left cell been traversed?
//                                                 i = (int) temp1.getValue(rowVals[0], colVals[0]);
//                                                 if (!BitOps.checkBit(i, 2)) {
//                                                     temp1.setValue(rowVals[0], colVals[0], BitOps.setBit(i, 2));
//                                                     traceDirection = 2;
//                                                 }
//                                             }

//                                             if (inputValueData[0] != inputValueData[2]
//                                                     && inputValueData[2] == z) {
//                                                 // has the top edge of the bottom-left cell been traversed?
//                                                 i = (int) temp1.getValue(rowVals[1], colVals[0]);
//                                                 if (!BitOps.checkBit(i, 0)) {
//                                                     temp1.setValue(rowVals[1], colVals[0], BitOps.setBit(i, 0));
//                                                     traceDirection = 2;
//                                                 }
//                                             }
//                                             break;

//                                         case 3:
//                                             // traceDirection 3
//                                             if (inputValueData[0] != inputValueData[1]
//                                                     && inputValueData[0] == z) {
//                                                 // has the right edge of the top-left cell been traversed?
//                                                 i = (int) temp1.getValue(rowVals[0], colVals[0]);
//                                                 if (!BitOps.checkBit(i, 1)) {
//                                                     temp1.setValue(rowVals[0], colVals[0], BitOps.setBit(i, 1));
//                                                     traceDirection = 3;
//                                                 }
//                                             }

//                                             if (inputValueData[0] != inputValueData[1]
//                                                     && inputValueData[1] == z) {
//                                                 // has the left edge of the top-right cell been traversed?
//                                                 i = (int) temp1.getValue(rowVals[0], colVals[1]);
//                                                 if (!BitOps.checkBit(i, 3)) {
//                                                     temp1.setValue(rowVals[0], colVals[1], BitOps.setBit(i, 3));
//                                                     traceDirection = 3;
//                                                 }
//                                             }

//                                     }
//                                     if (traceDirection != -1) {
//                                         break;
//                                     }
//                                 }

//                                 if (previousTraceDirection != traceDirection) {
//                                     xCoord = west + (currentHalfCol / cols) * EWRange;
//                                     yCoord = north - (currentHalfRow / rows) * NSRange;
//                                     points.addPoint(xCoord, yCoord);
//                                 }

//                                 switch (traceDirection) {
//                                     case 0:
//                                         currentHalfCol += 1.0;
//                                         break;
//                                     case 1:
//                                         currentHalfRow += 1.0;
//                                         break;
//                                     case 2:
//                                         currentHalfCol -= 1.0;
//                                         break;
//                                     case 3:
//                                         currentHalfRow -= 1.0;
//                                         break;
//                                     default:
//                                         flag = false;
//                                         break;
//                                 }
//                                 numPoints++;
//                                 if (numPoints > numCells) { // stopping condtion in case things get crazy
//                                     flag = false;
//                                 }
//                             } while (flag);

//                             if (numPoints > 1) {
//                                 com.vividsolutions.jts.geom.Polygon poly = factory.createPolygon(points.getCoordinateArraySequence());
//                                 if (!poly.isValid()) {
//                                     // fix the geometry with a buffer(0) as recommended in JTS docs
//                                     com.vividsolutions.jts.geom.Geometry jtsGeom2 = poly.buffer(0d);
//                                     int numGs = jtsGeom2.getNumGeometries();
//                                     for (int a = 0; a < numGs; a++) {
//                                         com.vividsolutions.jts.geom.Geometry gN = jtsGeom2.getGeometryN(a);
//                                         if (gN instanceof com.vividsolutions.jts.geom.Polygon) {
//                                             poly = (com.vividsolutions.jts.geom.Polygon) gN.clone();
//                                             poly.setSRID(regionValues.size());
//                                             polyList.add(poly);
//                                             regionValues.add((int)z);
//                                         }
//                                     }
//                                 } else {
//                                     int numGs = poly.getNumGeometries();
//                                     for (int a = 0; a < numGs; a++) {
//                                         com.vividsolutions.jts.geom.Geometry gN = poly.getGeometryN(a);
//                                         if (gN instanceof com.vividsolutions.jts.geom.Polygon) {
//                                             poly = (com.vividsolutions.jts.geom.Polygon) gN.clone();
//                                             poly.setSRID(regionValues.size());
//                                             polyList.add(poly);
//                                             regionValues.add((int)z);
//                                         }
//                                     }
// //                                        polyList.add(poly); //factory.createPolygon(points.getCoordinateArraySequence()));
// //                                        zVals.add(z);
//                                 }
//                             }
//                         }
//                     }
//                 }
//             }

//             progress = (int) (100f * row / (rows - 1));
//             if (progress != oldProgress) {
//                 updateProgress("Tracing polygons:", progress);
//                 oldProgress = progress;
//                 if (cancelOp) {
//                     cancelOperation();
//                     return;
//                 }
//             }
//         }

//         temp1.close();
//         input.close();
//         input1.close();

//         Collections.sort(polyList, new Comparator<com.vividsolutions.jts.geom.Polygon>() {

//             @Override
//             public int compare(com.vividsolutions.jts.geom.Polygon o1, com.vividsolutions.jts.geom.Polygon o2) {
//                 Double area1 = o1.getArea();
//                 Double area2 = o2.getArea();
//                 return area2.compareTo(area1);
//             }
            
//         });
        
        
//         int numPoly = polyList.size();
//         int[] regionData = new int[numPoly];
//         double[] zData = new double[numPoly];
//         for (i = 0; i < numPoly; i++) {
//             regionData[i] = regionValues.get(polyList.get(i).getSRID());
//             zData[i] = zValues[regionData[i]];
//         }
        
//         boolean[] outputted = new boolean[numPoly];
        
//         oldProgress = -1;
//         FID = 0;
//         for (i = 0; i < numPoly; i++) {
//             if (!outputted[i]) {
//                 outputted[i] = true;
                
//                 List<Integer> polyPartNums = new ArrayList<>();
//                 polyPartNums.add(i);
//                 for (int j = i + 1; j < numPoly; j++) {
//                     if (regionData[j] == regionData[i]) {
//                         polyPartNums.add(j);
//                         outputted[j] = true;
//                     }
//                 }
                
//                 FID++;
                
                
//                 int numHoles = polyPartNums.size() - 1;

//                 parts = new int[polyPartNums.size()];

//                 Object[] rowData = new Object[2];
//                 rowData[0] = (double) FID;
//                 rowData[1] = zData[i];

//                 com.vividsolutions.jts.geom.Polygon p = polyList.get(polyPartNums.get(0));
//                 PointsList points = new PointsList();
//                 Coordinate[] coords = p.getExteriorRing().getCoordinates();
//                 if (!Topology.isClockwisePolygon(coords)) {
//                     for (int j = coords.length - 1; j >= 0; j--) {
//                         points.addPoint(coords[j].x, coords[j].y);
//                     }
//                 } else {
//                     for (Coordinate coord : coords) {
//                         points.addPoint(coord.x, coord.y);
//                     }
//                 }

//                 for (int k = 0; k < numHoles; k++) {
//                     parts[k + 1] = points.size();

//                     p = polyList.get(polyPartNums.get(k + 1));
//                     coords = p.getExteriorRing().getCoordinates();
//                     if (Topology.isClockwisePolygon(coords)) {
//                         for (int j = coords.length - 1; j >= 0; j--) {
//                             points.addPoint(coords[j].x, coords[j].y);
//                         }
//                     } else {
//                         for (Coordinate coord : coords) {
//                             points.addPoint(coord.x, coord.y);
//                         }
//                     }

//                 }

//                 Polygon poly = new Polygon(parts, points.getPointsArray());
//                 output.addRecord(poly, rowData);
                
    
//             }
//             progress = (int) (100f * i / (numPoly - 1));
//             if (progress != oldProgress) {
//                 updateProgress("Writing data:", progress);
//                 oldProgress = progress;
//                 if (cancelOp) {
//                     cancelOperation();
//                     return;
//                 }
//             }
//         }
        









//         let mut rec_num = 1i32;
//         let (mut x, mut y): (f64, f64);
//         let mut z: f64;
//         for row in 0..rows {
//             for col in 0..columns {
//                 z = input.get_value(row, col);
//                 if z != 0.0f64 && z != nodata {
//                     x = input.get_x_from_column(col);
//                     y = input.get_y_from_row(row);
//                     output.add_point_record(x, y);
//                     output
//                         .attributes
//                         .add_record(vec![FieldData::Int(rec_num), FieldData::Real(z)], false);
//                     rec_num += 1i32;
//                 }
//             }
//             if verbose {
//                 progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
//                 if progress != old_progress {
//                     println!("Progress: {}%", progress);
//                     old_progress = progress;
//                 }
//             }
//         }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
            Ok(_) => {
                if verbose {
                    println!("Output file written")
                }
            }
            Err(e) => return Err(e),
        };
        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
