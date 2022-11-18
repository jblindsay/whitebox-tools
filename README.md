![](./img/WhiteboxToolsLogoBlue.png)


*This page is related to the stand-alone command-line program and Python scripting API for geospatial analysis, **WhiteboxTools**.

The official WhiteboxTools User Manual can be found [at this link](https://whiteboxgeo.com/manual/wbt_book/preface.html).

**Contents**

1. [Description](#1-description)
2. [Getting Help](#2-getting-help)
3. [Downloads and Installation](#3-pre-compiled-binaries)
4. [Building From Source Code](#4-building-from-source-code)

## 1 Description

**WhiteboxTools** is an advanced geospatial data analysis platform developed by Prof. John Lindsay ([webpage](http://www.uoguelph.ca/~hydrogeo/index.html); [jblindsay](https://github.com/jblindsay)) at the [University of Guelph's](http://www.uoguelph.ca) [*Geomorphometry and Hydrogeomatics Research Group*](http://www.uoguelph.ca/~hydrogeo/index.html). *WhiteboxTools* can be used to perform common geographical information systems (GIS) analysis operations, such as cost-distance analysis, distance buffering, and raster reclassification. Remote sensing and image processing tasks include image enhancement (e.g. panchromatic sharpening, contrast adjustments), image mosaicing, numerous filtering operations, classification, and common image transformations. *WhiteboxTools* also contains advanced tooling for spatial hydrological analysis (e.g. flow-accumulation, watershed delineation, stream network analysis, sink removal), terrain analysis (e.g. common terrain indices such as slope, curvatures, wetness index, hillshading; hypsometric analysis; multi-scale topographic position analysis), and LiDAR data processing. LiDAR point clouds can be interrogated (LidarInfo, LidarHistogram), segmented, tiled and joined, analyized for outliers, interpolated to rasters (DEMs, intensity images), and ground-points can be classified or filtered. *WhiteboxTools* is not a cartographic or spatial data visualization package; instead it is meant to serve as an analytical backend for other data visualization software, mainly GIS.

## 2 Getting help

WhiteboxToos possesses extensive help documentation. Users are referred to the [User Manual](https://www.whiteboxgeo.com/manual/wbt_book/) located on www.whiteboxgeo.com.

## 3 Pre-compiled binaries

*WhiteboxTools* is a stand-alone executable command-line program with no actual installation. If you intend to use the Python programming interface for *WhiteboxTools* you will need to have Python 3 (or higher) installed. Pre-compiled binaries can be downloaded from the [*Whitebox Geospatial Inc. website*](https://www.whiteboxgeo.com/download-whiteboxtools/) with support for various operating systems.

## 4 Building from source code

It is likely that *WhiteboxTools* will work on a wider variety of operating systems and architectures than the distributed binary files. If you do not find your operating system/architecture in the list of available *WhiteboxTool* binaries, then compilation from source code will be necessary. WhiteboxTools can be compiled from the source code with the following steps:

1. Install the Rust compiler; Rustup is recommended for this purpose. Further instruction can be found at this [link](https://www.rust-lang.org/en-US/install.html).

2. Download the *WhiteboxTools* from this GitHub repo.
```

3. Decompress the zipped download file.

4. Open a terminal (command prompt) window and change the working directory to the `whitebox-tools` folder:

```
>> cd /path/to/folder/whitebox-tools/
```

5. Finally, use the Python build.py script to compile the code:

```
>> python build.py
```

Depending on your system, the compilation may take several minutes. Also depending on your system, it may be necessary to use the `python3` command instead. When completed, the script will have created a new `WBT` folder within `whitebox-tools`. This folder will contain all of the files needed to run the program, including the main Whitebox executable file (whitebox_tools.exe), the Whitebox Runner GUI application, and the various plugins.

Be sure to follow the instructions for installing Rust carefully. In particular, if you are installing on MS Windows, you must have a linker installed prior to installing the Rust compiler (rustc). The Rust webpage recommends either the **MS Visual C++ 2015 Build Tools** or the GNU equivalent and offers details for each installation approach. You should also consider using **RustUp** to install the Rust compiler.
