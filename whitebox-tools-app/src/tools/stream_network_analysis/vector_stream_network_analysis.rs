/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 24/09/2018
Last Modified: 18/10/2019
License: MIT
*/

/*
This tool is a work in progress actively being ported from Whitebox GAT. It is not ready for use.
*/

use whitebox_raster::*;
use whitebox_common::structures::{Array2D, Point2D};
use crate::tools::*;
use whitebox_vector::ShapefileGeometry;
use whitebox_vector::*;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool calculates stream network geometry from vector streams.
/// 
/// # See Also
/// 
pub struct VectorStreamNetworkAnalysis {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl VectorStreamNetworkAnalysis {
    pub fn new() -> VectorStreamNetworkAnalysis {
        // public constructor
        let name = "VectorStreamNetworkAnalysis".to_string();
        let toolbox = "Stream Network Analysis".to_string();
        let description = "Calculates stream network geometry from vector streams.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Streams File".to_owned(),
            flags: vec!["--streams".to_owned()],
            description: "Input raster streams file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input D8 Pointer File".to_owned(),
            flags: vec!["--d8_pntr".to_owned()],
            description: "Input raster D8 pointer file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Line,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Does the pointer file use the ESRI pointer scheme?".to_owned(),
            flags: vec!["--esri_pntr".to_owned()],
            description: "D8 pointer uses the ESRI style scheme.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_owned()),
            optional: true,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --streams=streams.tif --d8_pntr=D8.tif -o=output.shp
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --streams=streams.tif --d8_pntr=D8.tif -o=output.shp --esri_pntr", short_exe, name).replace("*", &sep);

        VectorStreamNetworkAnalysis {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for VectorStreamNetworkAnalysis {
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
        let mut streams_file = String::new();
        let mut dem_file = String::new();
        let mut lakes_file = String::new();
        let mut lakes_used = false;
        let mut output_file = String::new();
        let mut snap_distance = 0f64;

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
            if flag_val == "-streams" {
                streams_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-dem" {
                dem_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-lakes" {
                lakes_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                lakes_used = true;
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-snap_dist" || flag_val == "-snap" {
                snap_distance = if keyval {
                    vec[1].to_string().parse::<f64>().expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1].to_string().parse::<f64>().expect(&format!("Error parsing {}", flag_val))
                };
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !streams_file.contains(&sep) && !streams_file.contains("/") {
            streams_file = format!("{}{}", working_directory, streams_file);
        }
        if !dem_file.contains(&sep) && !dem_file.contains("/") {
            dem_file = format!("{}{}", working_directory, dem_file);
        }
        if !lakes_file.contains(&sep) && !lakes_file.contains("/") {
            lakes_file = format!("{}{}", working_directory, lakes_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let start = Instant::now();

        // read the input DEM
        if verbose {
            println!("Reading DEM raster...")
        };
        let dem = Raster::new(&dem_file, "r")?;
        let rows = dem.configs.rows as isize;
        let columns = dem.configs.columns as isize;
        let nodata = dem.configs.nodata;

        // If the input DEM is in geographic coordinates, the snapdistance 
        // will need to be converted.
        let mut dist_multiplier = 1f64;
        if dem.is_in_geographic_coordinates() {
            let mut mid_lat = (input.configs.north - input.configs.south) / 2.0;
            if mid_lat <= 90.0 && mid_lat >= -90.0 {
                mid_lat = mid_lat.to_radians();
                let a = 6378137.0f64;
                let b = 6356752.314f64;
                let e2 = (a * a - b * b) / (a * a);
                let num = (f64::consts::PI * a * midLat.cos());
                let denum = (180f64 * (1f64 - e2 * midLat.sin() * midLat.sin()).sqrt());
                let long_deg_dist = num / denum;
                let lat_deg_dist = 111132.954f64 - 559.822f64 * (2f64 * midLat).cos() + 1.175f64 * (4f64 * midLat).cos();
                dist_multiplier = (long_deg_dist + lat_deg_dist) / 2f64;
                snap_distance = snap_distance / dist_multiplier;
            }
        }

        // snap_distance = snap_distance * snap_distance;

        // Read the streams file in
        let input = Shapefile::read(&streams_file)?;

        // make sure the input vector file is of polyline type
        if input.header.shape_type.base_shape_type() != ShapeType::PolyLine {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of POLYLINE base shape type.",
            ));
        }

        let num_features = input.num_records;

        ShapeFile lakes;
        let mut num_lakes = 0;
        let mut lakes_tree = KdTree::new_with_capacity(2, 64);
        let mut lakes_node_IDs: Vec<usize> = vec![];
        if lakes_used {
            lakes = Shapefile::read(&streams_file)?;
            if lakes.header.shape_type.base_shape_type() != ShapeType::PolyLine {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "The input vector data must be of POLYLINE base shape type.",
                ));
            }

            num_lakes = lakes.num_records;
            lakes_node_IDs = vec![-1; num_lakes];

            // read all of the lake vertices into a k-d tree.
            for record_num in 0..lakes.num_records {
                let record = lakes.get_record(record_num);
                for p in &record.points {
                    lakes_tree.add([p.x, p.y], 1).unwrap();
                }
            }
        }

        // create the output file
        let mut output = Shapefile::new(&output_file, ShapeType::PolyLine)?;
        // add the attributes
        output.attributes.add_field(&AttributeField::new("FID", FieldDataType::Int, 6u8, 0u8));
        output.attributes.add_field(&AttributeField::new("OUTLET", FieldDataType::Int, 10u8, 0u8));
        output.attributes.add_field(&AttributeField::new("TUCL", FieldDataType::Real, 10u8, 3u8));
        output.attributes.add_field(&AttributeField::new("MAXUPSDIST", FieldDataType::Real, 10u8, 3u8));
        output.attributes.add_field(&AttributeField::new("DS_NODES", FieldDataType::Int, 6u8, 0u8));
        output.attributes.add_field(&AttributeField::new("DIST2MOUTH", FieldDataType::Real, 10u8, 3u8));
        output.attributes.add_field(&AttributeField::new("HORTON", FieldDataType::Int, 6u8, 0u8));
        output.attributes.add_field(&AttributeField::new("STRAHLER", FieldDataType::Int, 6u8, 0u8));
        output.attributes.add_field(&AttributeField::new("SHREVE", FieldDataType::Real, 10u8, 3u8));
        output.attributes.add_field(&AttributeField::new("HACK", FieldDataType::Int, 6u8, 0u8));
        output.attributes.add_field(&AttributeField::new("MAINSTEM", FieldDataType::Int, 1u8, 0u8));
        output.attributes.add_field(&AttributeField::new("TRIB_ID", FieldDataType::Int, 6u8, 0u8));
        output.attributes.add_field(&AttributeField::new("DISCONT", FieldDataType::Int, 4u8, 0u8));
        
        let mut output_nodes = Shapefile::new(&output_file.replace(".shp", "_nodes.shp"), ShapeType::Point)?;
        output_nodes.attributes.add_field(&AttributeField::new("FID", FieldDataType::Int, 6u8, 0u8));
        output_nodes.attributes.add_field(&AttributeField::new("TYPE", FieldDataType::Text, 14u8, 0u8));
        
        /* Find all exterior nodes in the network. This includes nodes that
        are not associated with bifurcations as well as nodes where only
        one of the ajoining links overlaps with the DEM. 
    
        To do this, first read in the shapefile, retreiving each starting
        and ending nodes (called end-nodes) of the contained lines. Place 
        the end points into a k-d tree. Visit each site and count the 
        number of end points at each node. Those with more than one are
        bifurcations and exterior nodes have only one end point.
        */

        ///////////////////
        // Find edge cells
        ///////////////////
        let rows_less_one = rows - 1;
        // let mut nc: usize; // neighbouring cell
        let mut dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let mut dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let mut is_edge_cell: Array2D<i8> = Array2D::new(rows, columns, 0, -1)?;

        for row in 0..rows {
            for col in 0..columns {
                z = dem.get_value(row, col);
                if z != nodata {
                    for nc in 0..8 {
                        if dem.get_value(row + dy[nc], col + dx[nc]) == nodata {
                            is_edgeCell.setValue(row, col, 1);
                            break;
                        }
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        /////////////////////////////
        // count the number of parts
        /////////////////////////////
        let mut num_links = 0usize;
        let mut total_vertices = 0usize;
         for record_num in 0..input.num_records {
            let record = input.get_record(record_num);
            num_links += record.getGeometry().getParts().length;
            total_vertices += record.getGeometry().getPoints().length;
        }

        
        /*
        // Declare some variables
        int progress, oldProgress, col, row;
        int n, j;
        double x, y, z, z1, z2; //, x1, x2, y1, y2;
        double length;
        double distMultiplier = 1.0;
        Object[] rowData;
        int count = 0;
        double[][] points;
        int[] partData;
        int startingPointInPart, endingPointInPart;
        int i, numParts, numPoints, recNum, part, p;
        int outletNum = 1;
        int featureNum = 0;
        List<KdTree.Entry<Integer>> results;
        List<KdTree.Entry<Integer>> resultsLakes;
        double[] entry;
        //List<Integer> outletsLinkIDs = new ArrayList<>();

        KdTree<Integer> pointsTree;
        whitebox.geospatialfiles.shapefile.PolyLine wbGeometry;

            

            
            // first enter the line end-nodes into a kd-tree
            
            
            PriorityQueue<EndPoint> streamQueue = new PriorityQueue<>(totalVertices);
            links = new Link[numLinks];
            boolean[] crossesDemEdge = new boolean[numLinks];
            boolean[] isFeatureMapped = new boolean[numLinks];

            pointsTree = new KdTree.SqrEuclid<>(2, null);

            /////////////////////////////////////////////////////////////
            // Read the end-nodes into the KD-tree. 
            // Find potential outlet nodes and push them into the queue.
            /////////////////////////////////////////////////////////////
            boolean crossesValidData;
            boolean crossesNodata;
            boolean edgeValue1, edgeValue2;
            featureNum = -1;
            oldProgress = -1;
            int currentEndPoint = 0;
            int k = 0;
            for (ShapeFileRecord record : input.records) {
                recNum = record.getRecordNumber();
                points = record.getGeometry().getPoints();
                numPoints = points.length;
                partData = record.getGeometry().getParts();
                numParts = partData.length;
                for (part = 0; part < numParts; part++) {
                    featureNum++;
                    startingPointInPart = partData[part];
                    if (part < numParts - 1) {
                        endingPointInPart = partData[part + 1] - 1;
                    } else {
                        endingPointInPart = numPoints - 1;
                    }

                    length = 0;
                    for (i = startingPointInPart + 1; i <= endingPointInPart; i++) {
                        length += distMultiplier * Math.sqrt((points[i][0] - points[i - 1][0])
                                * (points[i][0] - points[i - 1][0]) + (points[i][1] - points[i - 1][1])
                                * (points[i][1] - points[i - 1][1]));
                    }

                    crossesValidData = false;
                    crossesNodata = false;
                    for (i = startingPointInPart; i <= endingPointInPart; i++) {

                        row = dem.getRowFromYCoordinate(points[i][1]);
                        col = dem.getColumnFromXCoordinate(points[i][0]);
                        z = dem.getValue(row, col);
                        if (z != nodata) {
                            crossesValidData = true;
                            isFeatureMapped[featureNum] = true;
                        }
                        if (isEdgeCell.getValue(row, col)) {
                            crossesNodata = true;
                        }
                        if (z == nodata) {
                            crossesNodata = true;
                        }
                    }

                    //linkLengths[featureNum] = length;
                    if (crossesNodata && crossesValidData) {
                        crossesDemEdge[featureNum] = true;
                    }

                    row = dem.getRowFromYCoordinate(points[startingPointInPart][1]);
                    col = dem.getColumnFromXCoordinate(points[startingPointInPart][0]);
                    z1 = dem.getValue(row, col);
                    edgeValue1 = isEdgeCell.getValue(row, col);

                    row = dem.getRowFromYCoordinate(points[endingPointInPart][1]);
                    col = dem.getColumnFromXCoordinate(points[endingPointInPart][0]);
                    z2 = dem.getValue(row, col);
                    edgeValue2 = isEdgeCell.getValue(row, col);

                    if (isFeatureMapped[featureNum]) {
                        x = points[startingPointInPart][0];
                        y = points[startingPointInPart][1];
                        entry = new double[]{x, y};
                        pointsTree.addPoint(entry, currentEndPoint);
                        EndPoint e1 = new EndPoint(currentEndPoint, featureNum, x, y, z1); //links.length, x, y, z1);
                        endPoints.add(e1);

                        x = points[endingPointInPart][0];
                        y = points[endingPointInPart][1];
                        entry = new double[]{x, y};
                        pointsTree.addPoint(entry, currentEndPoint + 1);
                        EndPoint e2 = new EndPoint(currentEndPoint + 1, featureNum, x, y, z2); //links.length, x, y, z2);
                        endPoints.add(e2);
                        //k++;

                        // This is a possible outlet.
                        if (crossesDemEdge[featureNum]) {
                            // rules for deciding with end point of the link is the actual outlet
//                            if ((z1 == nodata && z2 != nodata) || (edgeValue1 && (!edgeValue2 && z2 != nodata)) || z1 < z2) {
//                            if ((z1 == nodata && z2 != nodata) || (edgeValue1 && !edgeValue2) || (z1 < z2 && z1 != nodata)) {
//                                streamQueue.add(e1);
//                                e1.outflowingNode = true;
//                                
//                                whitebox.geospatialfiles.shapefile.Point pointOfInterest
//                                    = new whitebox.geospatialfiles.shapefile.Point(e1.x, e1.y);
//                                rowData = new Object[2];
//                                rowData[0] = new Double(1); //new Double(e1.nodeID);
//                                rowData[1] = "outlet";
//                                outputNodes.addRecord(pointOfInterest, rowData);
//                            } else {
//                                streamQueue.add(e2);
//                                e2.outflowingNode = true;
//                                whitebox.geospatialfiles.shapefile.Point pointOfInterest
//                                    = new whitebox.geospatialfiles.shapefile.Point(e2.x, e2.y);
//                                rowData = new Object[2];
//                                rowData[0] = new Double(2); //new Double(e2.nodeID);
//                                rowData[1] = "outlet";
//                                outputNodes.addRecord(pointOfInterest, rowData);
//                            }

                            // rules for deciding which end point of the link is the actual outlet
                            EndPoint e3 = e1;
                            if (z1 == nodata && z2 != nodata) { // first rule: one of end points is nodata and not the other
                                e3 = e1;
                            } else if (z2 == nodata && z1 != nodata) {
                                e3 = e2;
                            } else if (edgeValue1 && (!edgeValue2 && z2 != nodata)) { // second rule: one of the end points is and edge cell and not the other
                                e3 = e1;
                            } else if (edgeValue2 && (!edgeValue1 && z1 != nodata)) {
                                e3 = e2;
                            } else if (z1 < z2 && z2 != nodata) { // third rule: one of the points is lower
                                e3 = e1;
                            } else if (z2 < z1 && z1 != nodata) {
                                e3 = e2;
                            }

                            streamQueue.add(e3);
                            e3.outflowingNode = true;
//                            whitebox.geospatialfiles.shapefile.Point pointOfInterest
//                                = new whitebox.geospatialfiles.shapefile.Point(e3.x, e3.y);
//                            rowData = new Object[2];
//                            rowData[0] = new Double(2); //new Double(e2.nodeID);
//                            rowData[1] = "outlet";
//                            outputNodes.addRecord(pointOfInterest, rowData);

                        }
                        links[featureNum] = new Link(featureNum, currentEndPoint, currentEndPoint + 1, length);
                        currentEndPoint += 2;
                    }
                }

                progress = (int) (100f * recNum / numFeatures);
                if (progress != oldProgress) {
                    updateProgress("Characterizing nodes (loop 1 of 2):", progress);
                    oldProgress = progress;
                    // check to see if the user has requested a cancellation
                    if (cancelOp) {
                        cancelOperation();
                        return;
                    }
                }
            }

            isEdgeCell = null;

            boolean[] visitedEndPoint = new boolean[endPoints.size()];
            EndPoint e, e2;
            progress = -1;

            for (i = 0; i < endPoints.size(); i++) {
                if (!visitedEndPoint[i]) {
                    e = endPoints.get(i);
                    x = e.x;
                    y = e.y;
                    z = e.z;

                    entry = new double[]{x, y};
                    results = pointsTree.neighborsWithinRange(entry, snapDistance);

                    if (!results.isEmpty()) {
                        if (results.size() == 1 && lakesUsed && !e.outflowingNode) { // end node
                            visitedEndPoint[i] = true;
                            // check to see if it's a lake inlet/outlet
                            resultsLakes = lakesTree.neighborsWithinRange(entry, snapDistance);
                            if (!resultsLakes.isEmpty()) {
                                // which lake is this stream endnode connected to?
                                int lakeNum = (int) resultsLakes.get(0).value;

                                // does this lake already have a node?
                                int nodeNum = lakesNodeIDs[lakeNum];
                                if (nodeNum != -1) { // yes it does
                                    nodes.get(nodeNum).addPoint(i);
                                    endPoints.get(i).nodeID = nodeNum;
                                } else { // no, create a new node for it
                                    Node node = new Node();
                                    node.addPoint(i);
                                    endPoints.get(i).nodeID = nodes.size();
                                    lakesNodeIDs[lakeNum] = nodes.size();
                                    nodes.add(node);
                                }
                            } else {
                                Node node = new Node();
                                node.addPoint(i);
                                endPoints.get(i).nodeID = nodes.size();
                                nodes.add(node);
                                visitedEndPoint[i] = true;
                            }
                        } else {
                            Node node = new Node();
                            for (j = 0; j < results.size(); j++) {
                                currentEndPoint = (int) results.get(j).value;
                                node.addPoint(currentEndPoint);
                                visitedEndPoint[currentEndPoint] = true;
                                endPoints.get(currentEndPoint).nodeID = nodes.size();
                            }
                            nodes.add(node);
                        }
                    }
                }

                progress = (int) (100f * i / endPoints.size());
                if (progress != oldProgress) {
                    updateProgress("Characterizing nodes (loop 2 of 2):", progress);
                    oldProgress = progress;
                    // check to see if the user has requested a cancellation
                    if (cancelOp) {
                        cancelOperation();
                        return;
                    }
                }
            }

            /////////////////////////////////////////////////////////////////////
            // Priority-queue operation, progresses from downstream to upstream.
            // The flow-directions among connected arcs is determined in this step.
            /////////////////////////////////////////////////////////////////////
            Node node;
            Link link;
            int epNum;
            int numDownstreamNodes;
            double distToOutlet;
            int numPopped = 0;
            int outletID;
            int outletLinkID;
            oldProgress = -1;
            while (!streamQueue.isEmpty()) {
                numPopped++;
                e = streamQueue.poll();
                link = links[e.linkID];
                numDownstreamNodes = link.numDownstreamNodes;
                distToOutlet = link.distToOutlet;
                outletID = link.outlet;
                if (outletID == -1) {
                    links[e.linkID].outlet = outletNum;
                    //outletsLinkIDs.add(e.linkID);
                    outletID = outletNum;
                    outletNum++;
                    links[e.linkID].isOutletLink = true;
                    links[e.linkID].outletLinkID = e.linkID;
//                    links[e.linkID].isMainstem = true;
                    whitebox.geospatialfiles.shapefile.Point pointOfInterest
                            = new whitebox.geospatialfiles.shapefile.Point(e.x, e.y);
                    rowData = new Object[2];
                    rowData[0] = new Double(e.nodeID);
                    rowData[1] = "outlet";
                    outputNodes.addRecord(pointOfInterest, rowData);
                }
                outletLinkID = links[e.linkID].outletLinkID;
                // are there any unvisited links connected to this node directly?
                node = nodes.get(endPoints.get(e.endPointID).nodeID);
                for (int epNum2 : node.points) {
                    e2 = endPoints.get(epNum2);
                    if (links[e2.linkID].outlet == -1) { // hasn't previously been encountered
                        links[e2.linkID].outlet = outletID; //.get(e2.linkID).outlet = outletID;
                        links[e2.linkID].outletLinkID = outletLinkID;
                        links[e2.linkID].numDownstreamNodes = numDownstreamNodes + 1; //.get(e2.linkID).numDownstreamNodes = numDownstreamNodes + 1;
                        links[e2.linkID].distToOutlet = distToOutlet + links[e2.linkID].length; //.get(e2.linkID).distToOutlet = distToOutlet + links.get(e2.linkID).length;
                        links[e2.linkID].addOutflowingLink(link.index); //.get(e2.linkID).addDownstreamLink(link.index);
                        streamQueue.add(e2);
                        e2.outflowingNode = true;
                    }
                }

                // get the upstream end point and add its node's points to the queue
                epNum = link.getOtherEndPoint(e.endPointID);
                node = nodes.get(endPoints.get(epNum).nodeID);
//                if (node.numPoints > 2) { // it's either a bifurcation or a channel head
                for (int epNum2 : node.points) {
                    e2 = endPoints.get(epNum2);
                    if (links[e2.linkID].outlet == -1) { // hasn't previously been encountered
                        links[e2.linkID].outlet = outletID; //.get(e2.linkID).outlet = outletID;
                        links[e2.linkID].outletLinkID = outletLinkID;
                        links[e2.linkID].numDownstreamNodes = numDownstreamNodes + 1; //.get(e2.linkID).numDownstreamNodes = numDownstreamNodes + 1;
                        links[e2.linkID].distToOutlet = distToOutlet + links[e2.linkID].length; //.get(e2.linkID).distToOutlet = distToOutlet + links.get(e2.linkID).length;
                        links[e2.linkID].addOutflowingLink(link.index); //.get(e2.linkID).addDownstreamLink(link.index);
                        streamQueue.add(e2);
                        e2.outflowingNode = true;
                    } else if (links[e2.linkID].outlet == outletID
                            && e2.linkID != e.linkID && e2.outflowingNode) {
                        //!links[link.index].outflowingLinksInclude(e2.linkID)) { // diffluence
                        links[e2.linkID].addOutflowingLink(link.index);

                        whitebox.geospatialfiles.shapefile.Point pointOfInterest
                                = new whitebox.geospatialfiles.shapefile.Point(e2.x, e2.y);
                        rowData = new Object[2];
                        rowData[0] = new Double(e2.nodeID);
                        rowData[1] = "diffluence";
                        outputNodes.addRecord(pointOfInterest, rowData);

                    } else if (links[e2.linkID].outlet != outletID && !links[e2.linkID].isOutletLink) {
                        whitebox.geospatialfiles.shapefile.Point pointOfInterest
                                = new whitebox.geospatialfiles.shapefile.Point(e2.x, e2.y);
                        rowData = new Object[2];
                        rowData[0] = new Double(e2.nodeID);
                        rowData[1] = "joined head";
                        outputNodes.addRecord(pointOfInterest, rowData);
                    }
                }

                progress = (int) (100f * numPopped / endPoints.size());
                if (progress != oldProgress) {
                    updateProgress("Priority flood:", progress);
                    oldProgress = progress;
                    // check to see if the user has requested a cancellation
                    if (cancelOp) {
                        cancelOperation();
                        return;
                    }
                }
            }

            //////////////////////////////////////////////////////////////
            // Calculate the total upstream channel length (TUCL), 
            // Shreve stream orders, and the tributary ID by traversing
            // the graph from headwater channels towards their outlets
            //////////////////////////////////////////////////////////////
            updateProgress("Calculating downstream indices...", 0);

            int[] numInflowingLinks = new int[numLinks];
            for (Link lk : links) {
                if (lk != null) {
                    for (int dsl : lk.outflowingLinks) {
                        numInflowingLinks[dsl]++;
                        links[dsl].addInflowingLink(lk.index);
                    }
                }
            }

            LinkedList<Integer> stack = new LinkedList<>();
            int currentTribNum = 1;
            for (i = 0; i < numLinks; i++) {
                if (numInflowingLinks[i] == 0 && isFeatureMapped[i]) {
                    if (links[i].outlet != -1) {
                        stack.push(i);
                        links[i].shreveOrder = 1;
                        links[i].tribID = currentTribNum;
                        currentTribNum++;
                    }
                }
            }

            while (!stack.isEmpty()) {
                int currentLinkIndex = stack.pop();
                links[currentLinkIndex].tucl += links[currentLinkIndex].length;
                links[currentLinkIndex].maxUpstreamDist += links[currentLinkIndex].length;
                int numOutflows = links[currentLinkIndex].outflowingLinks.size();
                for (int dsl : links[currentLinkIndex].outflowingLinks) {
                    links[dsl].tucl += links[currentLinkIndex].tucl / numOutflows;
                    links[dsl].shreveOrder += links[currentLinkIndex].shreveOrder / numOutflows;
                    if (links[currentLinkIndex].maxUpstreamDist > links[dsl].maxUpstreamDist) {
                        links[dsl].maxUpstreamDist = links[currentLinkIndex].maxUpstreamDist;
                    }
                    numInflowingLinks[dsl]--;
                    if (numInflowingLinks[dsl] == 0) {
                        stack.push(dsl);
                        if (links[dsl].inflowingLinks.size() > 1) {
                            //i = 0;
                            //int largestOrder = 0;
                            //int secondLargestOrder = 0;
                            double largestTUCL = 0;
                            int tribOfLargestTUCL = -1;
                            double furthestHead = 0;
                            int tribOfFurthestHead = -1;
                            for (int usl : links[dsl].inflowingLinks) {
                                //i += links[usl].shreveOrder;
//                                if (links[usl].strahlerOrder >= largestOrder) {
//                                    secondLargestOrder = largestOrder;
//                                    largestOrder = links[usl].strahlerOrder;
//                                }
                                if (links[usl].tucl > largestTUCL) {
                                    largestTUCL = links[usl].tucl;
                                    tribOfLargestTUCL = links[usl].tribID;
                                }
                                if (links[usl].maxUpstreamDist > furthestHead) {
                                    furthestHead = links[usl].maxUpstreamDist;
                                    tribOfFurthestHead = links[usl].tribID;
                                }
                            }
//                            if (largestOrder == secondLargestOrder) {
//                                links[dsl].strahlerOrder = largestOrder + 1;
//                            } else {
//                                links[dsl].strahlerOrder = largestOrder;
//                            }
                            //links[dsl].shreveOrder = i;
                            links[dsl].tribID = tribOfFurthestHead; //tribOfLargestTUCL;
                        } else if (links[dsl].inflowingLinks.size() == 1) {
//                            links[dsl].strahlerOrder = links[currentLinkIndex].strahlerOrder;
                            //links[dsl].shreveOrder = links[currentLinkIndex].shreveOrder;
                            links[dsl].tribID = links[currentLinkIndex].tribID;
                        }
                    }
                }
            }

            ///////////////////////////////////////////////////////////
            // Descend from channel heads to outlets a second time to 
            // calculate the Strahler order, and to ID the main stem.
            ///////////////////////////////////////////////////////////
            numInflowingLinks = new int[numLinks];
            for (Link lk : links) {
                if (lk != null) {
                    for (int dsl : lk.outflowingLinks) {
                        numInflowingLinks[dsl]++;
                    }
                }
            }

            stack = new LinkedList<>();
            for (i = 0; i < numLinks; i++) {
                if (numInflowingLinks[i] == 0 && isFeatureMapped[i]) {
                    stack.push(i);
                    links[i].strahlerOrder = 1;
                    //links[i].shreveOrder = 1;
                }
            }

            while (!stack.isEmpty()) {
                int currentLinkIndex = stack.pop();
                if (links[currentLinkIndex].outlet != -1) {
                    // if the tribID of the outlet of this link is the same as the tribID of the link, it's a mainstem link.
                    if (links[links[currentLinkIndex].outletLinkID].tribID == links[currentLinkIndex].tribID) {
                        links[currentLinkIndex].isMainstem = true;
                    }
                }
                for (int dsl : links[currentLinkIndex].outflowingLinks) {
                    numInflowingLinks[dsl]--;
                    if (numInflowingLinks[dsl] == 0) {
                        stack.push(dsl);
                        if (links[dsl].inflowingLinks.size() > 1) {
                            i = 0;
                            int largestOrder = 0;
                            int tribIDLargestOrder = -1;
                            int secondLargestOrder = 0;
                            int tribIDSecondLargestOrder = -1;

                            for (int usl : links[dsl].inflowingLinks) {
                                if (links[usl].strahlerOrder >= largestOrder) {
                                    secondLargestOrder = largestOrder;
                                    tribIDSecondLargestOrder = tribIDLargestOrder;
                                    largestOrder = links[usl].strahlerOrder;
                                    tribIDLargestOrder = links[usl].tribID;
                                }
                            }
                            if (largestOrder == secondLargestOrder && tribIDLargestOrder != tribIDSecondLargestOrder) {
                                links[dsl].strahlerOrder = largestOrder + 1;
                            } else {
                                links[dsl].strahlerOrder = largestOrder;
                            }
                        } else if (links[dsl].inflowingLinks.size() == 1) {
                            links[dsl].strahlerOrder = links[currentLinkIndex].strahlerOrder;
                        }
                    }
                }
            }

            ////////////////////////////////////////////////////////////////////
            // Traverse the graph upstream from outlets to their channel heads
            // to calculate the Horton and Hack stream orders.
            ////////////////////////////////////////////////////////////////////
            updateProgress("Calculating upstream indices...", 0);
            stack = new LinkedList<>();
            boolean[] visited = new boolean[numLinks];
            for (i = 0; i < numLinks; i++) {
                if (links[i] != null && links[i].isOutletLink) {
                    stack.push(i);
                    links[i].hortonOrder = links[i].strahlerOrder;
                    links[i].hackOrder = 1;
                    visited[i] = true;
                }
            }

            int currentHorton, currentHack, currentTrib;
            while (!stack.isEmpty()) {
                int currentLinkIndex = stack.pop();
                currentHorton = links[currentLinkIndex].hortonOrder;
                currentHack = links[currentLinkIndex].hackOrder;
                currentTrib = links[currentLinkIndex].tribID;

                // Visit each the inflowing links to this link.
                for (int usl : links[currentLinkIndex].inflowingLinks) {
                    if (!visited[usl]) {
                        if (links[usl].tribID == currentTrib) {
                            links[usl].hortonOrder = currentHorton;
                            links[usl].hackOrder = currentHack;
                        } else {
                            links[usl].hortonOrder = links[usl].strahlerOrder;
                            links[usl].hackOrder = currentHack + 1;
                        }
                        stack.push(usl);
                        visited[usl] = true;
                    }
                }
            }

            // Outputs
            int[] outParts = {0};
            k = 0;
            PointsList pointsList;
            featureNum = -1;
            oldProgress = -1;
            for (ShapeFileRecord record : input.records) {
                recNum = record.getRecordNumber();
                points = record.getGeometry().getPoints();
                numPoints = points.length;
                partData = record.getGeometry().getParts();
                numParts = partData.length;
                for (part = 0; part < numParts; part++) {
                    featureNum++;
                    if (isFeatureMapped[featureNum]) {
                        startingPointInPart = partData[part];
                        if (part < numParts - 1) {
                            endingPointInPart = partData[part + 1] - 1;
                        } else {
                            endingPointInPart = numPoints - 1;
                        }
                        pointsList = new PointsList();
                        for (i = startingPointInPart; i <= endingPointInPart; i++) {
                            pointsList.addPoint(points[i][0], points[i][1]);
                        }
                        wbGeometry = new whitebox.geospatialfiles.shapefile.PolyLine(outParts, pointsList.getPointsArray());
                        rowData = new Object[13];
                        rowData[0] = new Double(k);
                        link = links[featureNum];
                        rowData[1] = new Double(link.outlet);
                        rowData[2] = link.tucl;
                        rowData[3] = link.maxUpstreamDist;
                        rowData[4] = new Double(link.numDownstreamNodes);
                        rowData[5] = link.distToOutlet;
                        rowData[6] = new Double(link.hortonOrder);
                        rowData[7] = new Double(link.strahlerOrder);
                        rowData[8] = new Double(link.shreveOrder);
                        rowData[9] = new Double(link.hackOrder);
                        if (link.isMainstem) {
                            rowData[10] = 1.0;
                        } else {
                            rowData[10] = 0.0;
                        }
                        rowData[11] = new Double(link.tribID);
                        if (link.outlet != -1) {
                            rowData[12] = 0.0;
                        } else {
                            rowData[12] = 1.0;
                        }
                        output.addRecord(wbGeometry, rowData);
                        k++;
                    }
                }

                progress = (int) (100f * recNum / numFeatures);
                if (progress != oldProgress) {
                    updateProgress("Saving output:", progress);
                    oldProgress = progress;
                    // check to see if the user has requested a cancellation
                    if (cancelOp) {
                        cancelOperation();
                        return;
                    }
                }
            }

            output.write();
            outputNodes.write();
            dem.close();

            pluginHost.updateProgress("Displaying output vector:", 0);

            String paletteDirectory = pluginHost.getResourcesDirectory() + "palettes" + File.separator;
            VectorLayerInfo vli = new VectorLayerInfo(outputFile, paletteDirectory, 255, -1);
            vli.setPaletteFile(paletteDirectory + "qual.pal");
            vli.setOutlinedWithOneColour(false);
            vli.setFillAttribute("OUTLET");
            vli.setPaletteScaled(false);
            vli.setRecordsColourData();
            pluginHost.returnData(vli);
        */

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
