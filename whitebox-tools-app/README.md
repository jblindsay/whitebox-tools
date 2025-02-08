![](./img/WhiteboxToolsLogoBlue.png)

*Bringing the power of Whitebox GAT to the world at large*



*This page is related to the stand-alone command-line program and Python scripting API for geospatial analysis, **WhiteboxTools**. If you are instead interested in the open-source GIS, **Whitebox GAT**, please see this [link](https://github.com/jblindsay/whitebox-geospatial-analysis-tools).*

The official WhiteboxTools User Manual can be found [at this link](https://jblindsay.github.io/wbt_book/index.html).

**Contents**

- [1 Description](#1-description)
- [2 Downloads and Installation](#2-downloads-and-installation)
  - [Pre-compiled binaries](#pre-compiled-binaries)
  - [Building from source code](#building-from-source-code)
  - [Using Docker image](#using-docker-image)
- [3 Usage](#3-usage)
- [4 Available Tools](#4-available-tools)
- [5 Supported Data Formats](#5-supported-data-formats)
- [6 Contributing](#6-contributing)
- [7 License](#7-license)
- [8 Reporting Bugs](#8-reporting-bugs)
- [9 Known Issues](#9-known-issues)
- [10 Frequently Asked Questions](#10-frequently-asked-questions)
  - [Do I need Whitebox GAT to use WhiteboxTools?](#do-i-need-whitebox-gat-to-use-whiteboxtools)
  - [How do I request a tool be added?](#how-do-i-request-a-tool-be-added)
  - [Can WhiteboxTools be incorporated into other software and open-source GIS projects?](#can-whiteboxtools-be-incorporated-into-other-software-and-open-source-gis-projects)
  - [What platforms does WhiteboxTools support?](#what-platforms-does-whiteboxtools-support)
  - [What are the system requirements?](#what-are-the-system-requirements)
  - [Are pre-compiled executables of WhiteboxTools available?](#are-pre-compiled-executables-of-whiteboxtools-available)
  - [Why is WhiteboxTools programmed in Rust?](#why-is-whiteboxtools-programmed-in-rust)
  - [Do I need Rust installed on my computer to run WhiteboxTools?](#do-i-need-rust-installed-on-my-computer-to-run-whiteboxtools)
  - [How does WhiteboxTools' design philosophy differ?](#how-does-whiteboxtools-design-philosophy-differ)

## 1 Description

**WhiteboxTools** is an advanced geospatial data analysis platform developed by Prof. John Lindsay ([webpage](http://www.uoguelph.ca/~hydrogeo/index.html); [jblindsay](https://github.com/jblindsay)) at the [University of Guelph's](http://www.uoguelph.ca) [*Geomorphometry and Hydrogeomatics Research Group*](http://www.uoguelph.ca/~hydrogeo/index.html). *WhiteboxTools* can be used to perform common geographical information systems (GIS) analysis operations, such as cost-distance analysis, distance buffering, and raster reclassification. Remote sensing and image processing tasks include image enhancement (e.g. panchromatic sharpening, contrast adjustments), image mosaicing, numerous filtering operations, simple classification (k-means), and common image transformations. *WhiteboxTools* also contains advanced tooling for spatial hydrological analysis (e.g. flow-accumulation, watershed delineation, stream network analysis, sink removal), terrain analysis (e.g. common terrain indices such as slope, curvatures, wetness index, hillshading; hypsometric analysis; multi-scale topographic position analysis), and LiDAR data processing. LiDAR point clouds can be interrogated (LidarInfo, LidarHistogram), segmented, tiled and joined, analyzed for outliers, interpolated to rasters (DEMs, intensity images), and ground-points can be classified or filtered. *WhiteboxTools* is not a cartographic or spatial data visualization package; instead it is meant to serve as an analytical backend for other data visualization software, mainly GIS.

Although *WhiteboxTools* is intended to serve as a source of plugin tools for the [*Whitebox GAT*](http://www.uoguelph.ca/~hydrogeo/Whitebox/) open-source GIS project, the tools contained in the library are stand-alone and can run outside of the larger Whitebox GAT project. See [Usage](#usage) for further details. There have been a large number of requests to call *Whitebox GAT* tools and functionality from outside of the Whitebox user-interface (e.g. from Python automation scripts). *WhiteboxTools* is intended to meet these usage requirements. Eventually most of the approximately 400 tools contained within *Whitebox GAT* [will be ported](tool_porting.md) to *WhiteboxTools*. In addition to separating the processing capabilities and the user-interface (and thereby reducing the reliance on Java), this migration should significantly improve processing efficiency. This is because [Rust](https://www.rust-lang.org/en-US/), the programming language used to develop *WhiteboxTools*, is generally [faster than the equivalent Java code](https://benchmarksgame-team.pages.debian.net/benchmarksgame/faster/rust-java.html) and because many of the *WhiteboxTools* functions are designed to process data in parallel wherever possible. In contrast, the older Java codebase included largely single-threaded applications.

The *WhiteboxTools* project is related to the [*GoSpatial*](https://github.com/jblindsay/go-spatial) project, which has similar goals but is designed using the Go programming language instead of Rust. *WhiteboxTools* has however superseded
the *GoSpatial* project, having subsumed all of its functionality.

## 2 Downloads and Installation

### Pre-compiled binaries

*WhiteboxTools* is a stand-alone executable command-line program with no actual installation. If you intend to use the Python programming interface for *WhiteboxTools* you will need to have Python 3 (or higher) installed. Pre-compiled binaries can be downloaded from the [*Geomorphometry and Hydrogeomatics Research Group*](http://www.uoguelph.ca/~hydrogeo/WhiteboxTools/index.html) software web site for various supported operating systems.

### Building from source code

It is likely that *WhiteboxTools* will work on a wider variety of operating systems and architectures than the distributed binary files. If you do not find your operating system/architecture in the list of available *WhiteboxTool* binaries, then compilation from source code will be necessary. WhiteboxTools can be compiled from the source code with the following steps:

1. Install the Rust compiler; Rustup is recommended for this purpose. Further instruction can be found at this [link](https://www.rust-lang.org/en-US/install.html).

2. Download the *WhiteboxTools* [source code](https://github.com/jblindsay/whitebox-tools). To download the code, click the green Clone or download button on the GitHub repository site. Alternatively, if you have [Git](https://git-scm.com/) installed on your computer, you can use the following command to clone the repository, then skip to step 4.

```
>> git clone https://github.com/jblindsay/whitebox-tools.git
```

3. Decompress the zipped download file.

4. Open a terminal (command prompt) window and change the working directory to the whitebox-tools folder:

```
>> cd /path/to/folder/whitebox-tools/
```

5. Finally, use the rust package manager Cargo, which will be installed along with Rust, to compile the executable:

```
>> cargo build --release
```

Depending on your system, the compilation may take several minutes. When completed, the compiled binary executable file will be contained within the *whitebox-tools/target/release/ folder*. Type *./whitebox_tools --help* at the command prompt (after cd'ing to the containing folder) for information on how to run the executable from the terminal.

Be sure to follow the instructions for installing Rust carefully. In particular, if you are installing on MS Windows, you must have a linker installed prior to installing the Rust compiler (rustc). The Rust webpage recommends either the **MS Visual C++ 2015 Build Tools** or the GNU equivalent and offers details for each installation approach. You should also consider using **RustUp** to install the Rust compiler.

### Using Docker image

For these who don't want to build from sources or can not use pre-build binaries there is also a Docker container that runs Whitebox Tools.

To build the image do:

1. clone Whitebox Tools repository to your local system or download code archive from the GitHub

    ```
    git clone https://github.com/jblindsay/whitebox-tools.git
    ```

2. Open a terminal (command prompt) window and change the working directory to the whitebox-tools folder

    ```
    cd /path/to/folder/whitebox-tools/
    ```

3. Build container

    ```
    docker build -t whitebox-tools -f docker/whitebox-tools.dockerfile .
    ```

4. Depending on your system, the build process may take several minutes. When completed, new image called `whitebox-tool` will be created.

To use container it is necessary to bind mount data directory into container as `/data` and then pass required command-line arguments, like below

```
docker run --rm -it -v "/path/to/data/directory/":/data whitebox-tools --run=IntegralImage -i=dem.tif -o=out.tif
```

## 3 Usage

*WhiteboxTools* is a command-line program and can be run either by calling it, with appropriate commands and arguments, from a terminal application, or, more conveniently, by calling it from a script. The following commands are recognized by the *WhiteboxTools* library:

| Command           | Description                                                                                       |
| ----------------- | ------------------------------------------------------------------------------------------------- |
| --cd, --wd        | Changes the working directory; used in conjunction with --run flag.                               |
| -h, --help        | Prints help information.                                                                          |
| -l, --license     | Prints the whitebox-tools license.                                                                |
| --listtools       | Lists all available tools, with tool descriptions. Keywords may also be used, --listtools slope.  |
| -r, --run         | Runs a tool; used in conjunction with --cd flag; -r="LidarInfo".                                  |
| --toolbox         | Prints the toolbox associated with a tool; --toolbox=Slope.                                       |
| --toolhelp        | Prints the help associated with a tool; --toolhelp="LidarInfo".                                   |
| --toolparameters  | Prints the parameters (in json form) for a specific tool; --toolparameters=\"LidarInfo\".         |
| -v                | Verbose mode. Without this flag, tool outputs will not be printed.                                |
| --viewcode        | Opens the source code of a tool in a web browser; --viewcode=\"LidarInfo\".                       |
| --version         | Prints the version information.                                                                   |

Generally, the Unix convention is that single-letter arguments (options) use a single hyphen (e.g. -h) while word-arguments (longer, more descriptive argument names) use double hyphen (e.g. --help). The same rule is used for passing arguments to tools as well. Use the *--toolhelp* argument to print information about a specific tool (e.g. --toolhelp=Clump). Tool names can be specified either using the snake_case or CamelCase convention (e.g. *lidar_info* or *LidarInfo*).

For examples of how to call functions and run tools from *WhiteboxTools*, see the *whitebox_example.py* Python script, which itself uses the *whitebox_tools.py* script as an interface for interacting with the executable file.

In addition to direct command-line and script-based interaction, a very basic user-interface called *WB Runner* can be used to call the tools within the *WhiteboxTools* executable file, providing the required tool arguments.

**Example command prompt:**

```
>>./whitebox_tools --wd='/Users/johnlindsay/Documents/data/' --run=DevFromMeanElev
--input='DEM clipped.dep' --output='DEV raster.dep' -v
```

Notice the quotation marks (single or double) used around directories and filenames, and string tool arguments in general. Use the '-v' flag (run in verbose mode) to force the tool print output to the command prompt. Please note that the whitebox_tools executable file must have permission to be executed; on some systems, this may require setting special permissions. The '>>' is shorthand for the command prompt and is not intended to be typed. Also, the above example uses the forward slash character (/), the directory path separator used on unix based systems. On Windows, users should use the back slash character (\\) instead.

**Example Python script:**

The following script relies on the imported functions contained within the whitebox_tools.py script, included within the *WhiteboxTools* distribution folder, and can be run using Python 3. **Please note that all of the scripts included with *WhiteboxTools* assumes the user system is configured with Python 3 and may not run as expected using Python 2**.

```Python
import os
import sys
from whitebox_tools import WhiteboxTools

wbt = WhiteboxTools()

# If the WhiteboxTools executable file (whitbox_tools.exe) is not in the same
# directory as this script, its path will need to be set, e.g.:
wbt.set_whitebox_dir(os.path.dirname(
    os.path.abspath(__file__)) + "/target/release/")  # or simply wbt.exe_path = ...

# Set the working directory. This is the path to the folder containing the data,
# i.e. files sent to tools as input/output parameters. You don't need to set
# the working directory if you specify full path names as tool parameters.
wbt.work_dir = os.path.dirname(os.path.abspath(__file__)) + "/testdata/"

# Sets verbose mode (True or False). Most tools will suppress output (e.g. updating
# progress) when verbose mode is False. The default is True
# wbt.set_verbose_mode(False) # or simply, wbt.verbose = False

# The most convenient way to run a tool is to use its associated method, e.g.:
wbt.elev_percentile("DEM.tif", "output.tif", 15, 15)
# You may also provide an optional custom callback for processing output from the
# tool. If you don't provide a callback, and verbose is set to True, tool output
# will simply be printed to the standard output.

# Prints the whitebox-tools help...a listing of available commands
print(wbt.help())

# Prints the whitebox-tools license
print(wbt.license())

# Prints the whitebox-tools version
print("Version information: {}".format(wbt.version()))

# List all available tools in whitebox-tools
print(wbt.list_tools())

# Lists tools with 'lidar' or 'LAS' in tool name or description.
print(wbt.list_tools(['lidar', 'LAS']))

# Print the help for a specific tool.
print(wbt.tool_help("ElevPercentile"))
# Notice that tool names within WhiteboxTools.exe are CamelCase but
# you can also use snake_case here, e.g. print(wbt.tool_help("elev_percentile"))

```

**WhiteboxTools Runner**

There is a Python script contained within the *WhiteboxTools* directory called '*wb_runner.py*'. This script is intended to provide a very basic user-interface for running the tools contained within the *WhiteboxTools* library. The user-interface uses Python's TkInter GUI library and is cross-platform. The user interface is currently experimental and is under heavy testing. Please report any issues that you experience in using it.

![*WhiteboxTools Runner* user-interface](./img/WBRunner.png)

The *WhiteboxTools Runner* does not rely on the *Whitebox GAT* user interface at all and can therefore be used independent of the larger project. The script must be run from a directory that also contains the '*whitebox_tools.py*' Python script and the '*whitebox_tools*' executable file. There are plans to link tool help documentation in *WhiteboxTools Runner*.

## 4 Available Tools

Eventually most of *Whitebox GAT's* approximately 430 tools [will be ported](tool_porting.md) to *WhiteboxTools*, although this is an immense task. Support for vector data (Shapefile/GeoJSON) reading/writing and a topological analysis library (like the Java Topology Suite) will need to be added in order to port the tools involving vector spatial data. Opportunities to parallelize algorithms will be sought during porting. All new plugin tools will be added to *Whitebox GAT* using this library of functions.

The library currently contains more than 397 tools, which are each grouped based on their main function into one of the following categories: Data Tools, GIS Analysis, Hydrological Analysis, Image Analysis, LiDAR Analysis, Mathematical and Statistical Analysis, Stream Network Analysis, and Terrain Analysis. For a listing of available tools, complete with documentation and usage details, please see the [WhiteboxTools User Manual](https://jblindsay.github.io/wbt_book/available_tools/index.html).

To retrieve detailed information about a tool's input arguments and example usage, either use the *--toolhelp* command from the terminal, or the *tool_help('tool_name')* function from the *whitebox_tools.py* script.

## 5 Supported Data Formats
The **WhiteboxTools** library can currently support read/writing raster data in [*Whitebox GAT*](http://www.uoguelph.ca/~hydrogeo/Whitebox/), GeoTIFF, ESRI (ArcGIS) ASCII and binary (.flt & .hdr), GRASS GIS, Idrisi, SAGA GIS (binary and ASCII), and Surfer 7 data formats. The library is primarily tested using Whitebox raster data sets and if you encounter issues when reading/writing data in other formats, you should report the [issue](#reporting-bugs). Please note that there are no plans to incorporate third-party libraries, like [GDAL](http://www.gdal.org), in the project given the design goal of keeping a pure (or as close as possible) Rust codebase.

At present, there is limited ability in *WhiteboxTools* to read vector geospatial data. Support for Shapefile (and other common vector formats) will be enhanced within the library soon.

LiDAR data can be read/written in the common [LAS](https://www.asprs.org/committee-general/laser-las-file-format-exchange-activities.html) data format. *WhiteboxTools* can read and write LAS files that have been compressed (zipped with a .zip extension) using the common DEFLATE algorithm. Note that only LAS file should be contained within a zipped archive file. The compressed LiDAR format LAZ and ESRI LiDAR format are not currently supported by the library. The following is an example of running a LiDAR tool using zipped input/output files:

```
>>./whitebox_tools -r=LidarTophatTransform -v --wd="/path/to/data/"
-i="input.las.zip" -o="output.las.zip" --radius=10.0
```

Note that the double extensions (.las.zip) in the above command are not necessary and are only used for convenience of keeping track of LiDAR data sets (i.e. .zip extensions work too). The extra work of decoding/encoding compressed files does add additional processing time, although the Rust compression library that is used is highly efficient and usually only adds a few seconds to tool run times. Zipping LAS files frequently results 40-60% smaller binary files, making the additional processing time worthwhile for larger LAS file data sets with massive storage requirements.

## 6 Contributing

If you would like to contribute to the project as a developer, follow these instructions to get started:

1. Fork the larger Whitebox project (in which whitebox-tools exists) ( https://github.com/jblindsay/whitebox-geospatial-analysis-tools )
2. Create your feature branch (git checkout -b my-new-feature)
3. Commit your changes (git commit -am 'Add some feature')
4. Push to the branch (git push origin my-new-feature)
5. Create a new Pull Request

Unless explicitly stated otherwise, any contribution intentionally submitted for inclusion in the work shall be licensed [as above](#license) without any additional terms or conditions.

If you would like to contribute financial support for the project, please contact [John Lindsay](http://www.uoguelph.ca/~hydrogeo/index.html). We also welcome contributions in the form of media exposure. If you have written an article or blog about *WhiteboxTools* please let us know about it.

## 7 License

The **WhiteboxTools** library is distributed under the [MIT license](LICENSE.txt), a permissive open-source (free software) license.

## 8 Reporting Bugs

WhiteboxTools is distributed as is and without warranty of suitability for application. If you encounter flaws with the software (i.e. bugs) please report the issue. Providing a detailed description of the conditions under which the bug occurred will help to identify the bug. *Use the Issues tracker on GitHub to report issues with the software and to request feature enhancements.* Please do not email Dr. Lindsay directly with bugs.

## 9 Known Issues

- Given the extreme complexity of the GeoTIFF file format, and the fact that the project uses a custom, stand-alone GeoTIFF library, it is likely that some users will encounter limitations (e.g. the BigTIFF format is currently unsupported) or bugs.
- There is limited support for reading, writing, or analyzing vector data yet. Plans include native support for the ESRI Shapefile format and possibly GeoJSON data.
- The LAZ compressed LiDAR data format is currently unsupported although zipped LAS files (.zip) are.
- File directories cannot contain apostrophes (', e.g. /John's data/) as they will be interpreted in the arguments array as single quoted strings.
- The Python scripts included with **WhiteboxTools** require Python 3. They will not work with Python 2, which is frequently the default Python version installed on many systems.

## 10 Frequently Asked Questions

### Do I need Whitebox GAT to use WhiteboxTools?

No you do not. You can call the tools contained within *WhiteboxTools* completely independent from the *Whitebox GAT* user interface using a Remote Procedure Call (RPC) approach. In fact, you can interact with the tools using Python scripting or directly, using a terminal application (command prompt). See [Usage](#usage) for further details.

### How do I request a tool be added?

Eventually most of the tools in *Whitebox GAT* will be ported over to *WhiteboxTools* and all new tools will be added to this library as well. Naturally, this will take time. The order by which tools are ported is partly a function of ease of porting, existing infrastructure (i.e. raster and LiDAR tools will be ported first since their is currently no support in the library for vector I/O), and interest. If you are interested in making a tool a higher priority for porting, email [John Lindsay](http://www.uoguelph.ca/~hydrogeo/index.html).

### Can WhiteboxTools be incorporated into other software and open-source GIS projects?

*WhiteboxTools* was developed with the open-source GIS [Whitebox GAT](http://www.uoguelph.ca/~hydrogeo/Whitebox/index.html) in mind. That said, the tools can be accessed independently and so long as you abide by the terms of the [MIT license](#license), there is no reason why other software and GIS projects cannot use *WhiteboxTools* as well. In fact, this was one of the motivating factors for creating the library in the first place. Feel free to use *WhiteboxTools* as the geospatial analysis engine in your open-source software project.

### What platforms does WhiteboxTools support?

*WhiteboxTools* is developed using the Rust programming language, which supports a [wide variety of platforms](https://forge.rust-lang.org/platform-support.html) including MS Windows, MacOS, and Linux operating systems and common chip architectures. Interestingly, Rust also supports mobile platforms, and *WhiteboxTools* should therefore be capable of targeting (although no testing has been completed in this regard to date). Nearly all development and testing of the software is currently carried out on MacOS and we cannot guarantee a bug-free performance on other platforms. In particularly, MS Windows is the most different from the other platforms and is therefore the most likely to encounter platform-specific bugs. If you encounter bugs in the software, please consider reporting an issue using the GitHub support for issue-tracking.

### What are the system requirements?

The answer to this question depends strongly on the type of analysis and data that you intend to process. However, generally we find performance to be optimal with a recommended minimum of 8-16GB of memory (RAM), a modern multi-core processor (e.g. 64-bit i5 or i7), and an solid-state-drive (SSD). It is likely that *WhiteboxTools* will have satisfactory performance on lower-spec systems if smaller datasets are being processed. Because *WhiteboxTools* reads entire raster datasets into system memory (for optimal performance, and in recognition that modern systems have increasingly larger amounts of fast RAM), this tends to be the limiting factor for the upper-end of data size successfully processed by the library. 64-bit operating systems are recommended and extensive testing has not been carried out on 32-bit OSs. See [**"What platforms does WhiteboxTools support?"**](#what-platforms-does-whiteboxtools-support) for further details on supported platforms.

### Are pre-compiled executables of WhiteboxTools available?

Pre-compiled binaries for *WhiteboxTools* can be downloaded from the [*Geomorphometry and Hydrogeomatics Research Group*](http://www.uoguelph.ca/~hydrogeo/software.shtml#WhiteboxTools) software web site for various supported operating systems. If you need binaries for other operating systems/system architectures, you will need to compile the executable from source files. See [Installation](#installation) for details.

### Why is WhiteboxTools programmed in Rust?

I spent a long time evaluating potential programming language for future development efforts for the *Whitebox GAT* project. My most important criterion for a language was that it compile to native code, rather than target the Java virtual machine (JVM). I have been keen to move Whitebox GAT away from Java because of some of the challenges that supporting the JVM has included for many Whitebox users. The language should be fast and productive--Java is already quite fast, but if I am going to change development languages, I would like a performance boost. Furthermore, given that many, though not all, of the algorithms used for geospatial analysis scale well with concurrent (parallel) implementations, I favoured languages that offered easy and safe concurrent programming. Although many would consider C/C++ for this work, I was looking for a modern and safe language. Fortunately, we are living through a renaissance period in programming language development and there are many newer languages that fit the bill nicely. Over the past two years, I considered each of Go, Rust, D, Nim, and Crystal for Whitebox development and ultimately decided on Rust. [See [*GoSpatial*](https://github.com/jblindsay/go-spatial) and [*lidario*](https://github.com/jblindsay/lidario).]

Each of the languages I examined has its own advantages of disadvantages, so why Rust? It's a combination of factors that made it a compelling option for this project. Compared with many on the list, Rust is a mature language with a vibrant user community. Like C/C++, it's a high-performance and low-level language that allows for complete control of the system. However, Rust is also one of the safest languages, meaning that I can be confident that *WhiteboxTools* will not contain common bugs, such as memory use-after-release, memory leaks and race conditions within concurrent code. Importantly, and quite uniquely, this safety is achieved in the Rust language without the use of a garbage collector (automatic memory management). Garbage collectors can be great, but they do generally come with a certain efficiency trade-off that Rust does not have. The other main advantage of Rust's approach to memory management is that it allows for  a level of interaction with scripting languages (e.g. Python) that is quite difficult to do in garbage collected languages. Although **WhiteboxTools** is currently set up to use an automation approach to interacting with Python code that calls it, I like the fact that I have the option to create a *WhiteboxTools* shared library.

Not everything with Rust is perfect however. It is still a very young language and there are many pieces still missing from its ecosystem. Furthermore, it is not the easiest language to learn, particularly for people who are inexperienced with programming. This may limit my ability to attract other programmers to the Whitebox project, which would be unfortunate. However, overall, Rust was the best option for this particular application.

### Do I need Rust installed on my computer to run WhiteboxTools?

No, you would only need Rust installed if you were compiling the WhiteboxTools codebase from source files.

### How does WhiteboxTools' design philosophy differ?

*Whitebox GAT* is frequently praised for its consistent design and ease of use. Like *Whitebox GAT*, *WhiteboxTools* follows the convention of *one tool for one function*. For example, in *WhiteboxTools* assigning the links in a stream channel network their Horton, Strahler, Shreve, or Hack stream ordering numbers requires running separate tools (i.e. *HortonStreamOrder*, *StrahlerStreamOrder*, *ShreveStreamMagnitude*, and *HackStreamOrder*). By contrast, in GRASS GIS<sup>1</sup> and ArcGIS single tools (i.e. the *r.stream.order* and *Stream Order* tools respectively) can be configured to output different channel ordering schemes. The *WhiteboxTools* design is intended to simplify the user experience and to make it easier to find the right tool for a task. With more specific tool names that are reflective of their specific purposes, users are not as reliant on reading help documentation to identify the tool for the task at hand. Similarly, it is not uncommon for tools in other GIS to have multiple outputs. For example, in GRASS GIS the *r.slope.aspect* tool can be configured to output slope, aspect, profile curvature, plan curvature, and several other common terrain surface derivatives. Based on the *one tool for one function* design approach of *WhiteboxTools*, multiple outputs are indicative that a tool should be split into different, more specific tools. Are you more likely to go to a tool named *r.slope.aspect* or *TangentialCurvature* when you want to create a tangential curvature raster from a DEM? If you're new to the software and are unfamiliar with it, probably the later is more obvious. The *WhiteboxTools* design approach also has the added benefit of simplifying the documentation for tools. The one downside to this design approach, however, is that it results (or will result) in a large number of tools, often with significant overlap in function.

<sup>1</sup> NOTE: It's not my intent to criticize GRASS GIS, as I deeply respect the work that the GRASS developers have contributed. Rather, I am contrasting the consequences of *WhiteboxTools'* design philosophy to that of other GIS.
