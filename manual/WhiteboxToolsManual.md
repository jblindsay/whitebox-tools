---
title: 'WhiteboxTools User Manual'
subtitle: 'Bringing the power of Whitebox GAT to the world at large'
author: John B. Lindsay, PhD, University of Guelph
fontsize: 12pt
mainfont: 'Open Sans'
documentclass: report
# output:
#   pdf_document:
#     toc: true
header-includes:
    - \usepackage[vmargin=1in,hmargin=1in]{geometry}
    - \usepackage{fancyhdr}
    - \fancyhf{}
    - \pagestyle{fancy}
    - \fancyhead[C]{\textit{WhiteboxTools User Manual / Lindsay}}
    - \fancyhead[LE,RO]{\thepage}
    - \fancyfoot[C]{}
    - \renewcommand{\headrulewidth}{0.5pt}
    - \usepackage{caption}
    - \captionsetup[figure]{labelformat=empty}
    - \usepackage{framed}
    - \usepackage{xcolor}
    - \let\oldquote=\quote
    - \let\endoldquote=\endquote
    - \colorlet{shadecolor}{gray!15}
    - \renewenvironment{quote}{\begin{shaded*}\begin{oldquote}}{\end{oldquote}\end{shaded*}}
    - \usepackage{titling}
    - \pretitle{\begin{center}\LARGE\includegraphics[width=12cm]{./img/WhiteboxToolsLogoBlue.png}\\[\bigskipamount]}
    - \posttitle{\end{center}}
---

\thispagestyle{empty}

\newpage

<!-- ![*Bringing the power of Whitebox GAT to the world at large*](./img/WhiteboxToolsLogoBlue.png)   -->

WhiteboxTools Version 0.4  \
Dr. John B. Lindsay &#169; March 2, 2018  \
Geomorphometry and Hydrogeomatics Research Group  \
University of Guelph  \
Guelph, Canada \

![](./img/GHRGLogoSm.png){width=54% height=54%}

\newpage


## 1. Introduction

***WhiteboxTools*** is an advanced geospatial data analysis engine developed by Prof. John Lindsay ([webpage](http://www.uoguelph.ca/~hydrogeo/index.html); [jblindsay](https://github.com/jblindsay)) at the [University of Guelph's](http://www.uoguelph.ca) [Geomorphometry and Hydrogeomatics Research Group](http://www.uoguelph.ca/~hydrogeo/index.html) (GHRG). The project began in January 2017 and quickly evolved in terms of its analytical capabilities. *WhiteboxTools* can be used to perform common geographical information systems (GIS) analysis operations, such as cost-distance analysis, distance buffering, and raster reclassification. Remote sensing and image processing tasks include image enhancement (e.g. panchromatic sharpening, contrast adjustments), image mosaicking, numerous filtering operations, simple classification (k-means clustering), and common image transformations. *WhiteboxTools* also contains advanced tooling for spatial hydrological analysis (e.g. flow-accumulation, watershed delineation, stream network analysis, sink removal), terrain analysis (e.g. common terrain indices such as slope, curvatures, wetness index, hillshading; hypsometric analysis; multi-scale topographic position analysis), and LiDAR data processing. LiDAR point clouds can be interrogated (LidarInfo, LidarHistogram), segmented, tiled and joined, analyzed for outliers, interpolated to rasters (DEMs, intensity images), and ground-points can be classified or filtered. *WhiteboxTools* is not a cartographic or spatial data visualization package; instead it is meant to serve as an analytical backend for other data visualization software, mainly GIS.

> In this manual, ***WhiteboxTools*** refers to the standalone geospatial analysis library, a collection of tools contained within a compiled binary executable command-line program and the associated Python scripts that are distributed alongside the binary file (e.g. *whitebox_tools.py* and *wb_runner.py*). ***Whitebox Geospatial Analysis Tools*** and ***Whitebox GAT*** refer to the GIS software, which includes a user-interface (front-end), point-and-click tool interfaces, and cartographic data visualization capabilities.

Although *WhiteboxTools* is intended to serve as a source of plugin tools for the [*Whitebox Geospatial Analysis Tools (GAT)*](http://www.uoguelph.ca/~hydrogeo/Whitebox/) open-source GIS project, the tools contained in the library are stand-alone and can run outside of the larger *Whitebox GAT* project. See [Interacting With *WhiteboxTools* From the Command Prompt](#interacting-with-whiteboxtools-from-the-command-prompt) for further details. There have been a large number of requests to call *Whitebox GAT* tools and functionality from outside of the *Whitebox GAT* user-interface (e.g. from Python automation scripts). *WhiteboxTools* is intended to meet these usage requirements. The current version of *Whitebox GAT* contains many equivelent tools to those found in the *WhiteboxTools* library, although they are developed using the Java programming language. A future version of *Whitebox GAT* will replace these previous tools with the new *WhiteboxTools* backend. This transition will occur over the next several releases. Eventually most of the approximately 450 tools contained within *Whitebox GAT* [will be ported](tool_porting.md) to *WhiteboxTools*. In addition to separating the processing capabilities and the user-interface (and thereby reducing the reliance on Java), this migration should significantly improve processing efficiency. This is because [Rust](https://www.rust-lang.org/en-US/), the programming language used to develop *WhiteboxTools*, is generally [faster than the equivalent Java code](http://benchmarksgame.alioth.debian.org/u64q/compare.php?lang=rust&lang2=java) and because many of the *WhiteboxTools* functions are designed to process data in parallel wherever possible. In contrast, the older Java codebase included largely single-threaded applications.

In addition to *Whitebox GAT*, the *WhiteboxTools* project is related to other GHRG software projects including, the [*GoSpatial*](https://github.com/jblindsay/go-spatial) project, which has similar goals but is designed using the Go programming language instead of Rust. *WhiteboxTools* has however superseded the *GoSpatial* project, having subsumed all of its functionality. *GoSpatial* users should now transition to *WhiteboxTools*.

## 2. Downloads and Installation

*WhiteboxTools* is a stand-alone executable command-line program with no actual installation. If you intend to use the Python programming interface for *WhiteboxTools* you will need to have Python 3 (or higher) installed. Pre-compiled binaries can be downloaded from the [*Geomorphometry and Hydrogeomatics Research Group*](http://www.uoguelph.ca/~hydrogeo/software.shtml#WhiteboxTools) software web site for various supported operating systems. It is likely that *WhiteboxTools* will work on a wider variety of operating systems and architectures. If you do not find your operating system/architecture in the list of available *WhiteboxTool* binaries, then compilation from source code will be necessary. WhiteboxTools can be compiled from the source code with the following steps:

1. Install the Rust compiler; Rustup is recommended for this purpose. Further instruction can be found at this [link](https://www.rust-lang.org/en-US/install.html).

2. Download the *Whitebox GAT* [source code](https://github.com/jblindsay/whitebox-geospatial-analysis-tools). Note: *WhiteboxTools* is currently housed as a sub-repository of the main *Whitebox GAT* repo. To download the code, click the green Clone or download button on the GitHub repository site.

3. Decompress the zipped download file.

4. Open a terminal (command prompt) window and change the working directory to the whitebox_tools sub-folder, which is contained within the decompressed downloaded Whitebox GAT folder:

```
>> cd /path/to/folder/whitebox_tools/
```

5. Finally, use the rust package manager Cargo, which will be installed alongside Rust, to compile the executable:

```
>> cargo build --release
```

Depending on your system, the compilation may take several minutes. When completed, the compiled binary executable file will be contained within the *whitebox_tools/target/release/ folder*. Type *./whitebox_tools -\-help* at the command prompt (after changing the directory to the containing folder) for information on how to run the executable from the terminal.

> The '>>' is shorthand used in this document to denote the command prompt and is not intended to be typed. 

Be sure to follow the instructions for installing Rust carefully. In particular, if you are installing on Microsoft Windows, you must have a linker installed prior to installing the Rust compiler (*rustc*). The Rust webpage recommends either the **MS Visual C++ 2015 Build Tools** or the GNU equivalent and offers details for each installation approach. You should also consider using **RustUp** to install the Rust compiler.

## 3. Interacting With *WhiteboxTools* From the Command Prompt

*WhiteboxTools* is a command-line program and can be run either by calling it from a terminal application with appropriate commands and arguments, or, more conveniently, by calling it from a script. The following commands are recognized by the *WhiteboxTools* library:

 **Command**         **Description**                                                                                       
 ------------------  ----------------------------------------------------------------------------------- 
 -\-cd, -\-wd        Changes the working directory; used in conjunction with -\-run flag.                 
 -h, -\-help         Prints help information.                                                          
 -l, -\-license      Prints the whitebox-tools license.                                                
 -\-listtools        Lists all available tools, with tool descriptions. Keywords may also be 
                     used, -\-listtools slope.  
 -r, -\-run          Runs a tool; used in conjunction with -\-cd flag; -r="LidarInfo".                  
 -\-toolbox          Prints the toolbox associated with a tool; -\-toolbox=Slope.                 
 -\-toolhelp         Prints the help associated with a tool; -\-toolhelp="LidarInfo".                
 -\-toolparameters   Prints the parameters (in json form) for a specific tool;
                     e.g. -\-toolparameters="FeaturePreservingDenoise".
 -v                  Verbose mode. Without this flag, tool outputs will not be printed.              
 -\-viewcode         Opens the source code of a tool in a web browser; -\-viewcode=\"LidarInfo\". 
 -\-version          Prints the version information.                                                                   

Generally, the Unix convention is that single-letter arguments (options) use a single hyphen (e.g. -h) while word-arguments (longer, more descriptive argument names) use double hyphens (e.g. -\-help). The same rule is used for passing arguments to tools as well. Use the *-\-toolhelp* argument to print information about a specific tool (e.g. -\-toolhelp=Clump). 

> Tool names can be specified either using the snake_case or CamelCase convention (e.g. *lidar_info* or *LidarInfo*).

The following is an example of calling the *WhiteboxTools* binary executable file directly from the command prompt: 

```

>>./whitebox_tools --wd='/Users/johnlindsay/Documents/data/' ^
--run=DevFromMeanElev --input='DEM clipped.dep' ^
--output='DEV raster.dep' -v


```

Notice the quotation marks (single or double) used around directories and filenames, and string tool arguments in general. Use the '-v' flag (run in verbose mode) to force the tool print output to the command prompt. Please note that the whitebox_tools executable file must have permission to be executed; on some systems, this may require setting special permissions. Also, the above example uses the forward slash character (/), the directory path separator used on unix based systems. On Windows, users should use the back slash character (\\) instead. Also, it is sometimes necessary to break (^) commands across multiple lines, as above, in order to better fit with the documents format. Actual command prompts should be contained to a single line.

## 4. Interacting With *WhiteboxTools* Using Python Scripting

By combining the *WhiteboxTools* library with the a high-level scripting language, such as Python, users are capable of creating powerful stand-alone geospatial applications and workflow automation scripts. In fact, *WhiteboxTools* functionality can be called from many different programming languages. However, given the prevalent use of the Python language in the geospatial field, the library is distributed with several resources specifically aimed at Python scripting. This section focuses on how Python programming can be used to interact with the *WhiteboxTools* library.

> Note that all of the following material assumes the user system is configured with Python 3. The code snippets below are not guaranteed to work with older versions of the language. 

Interacting with *WhiteboxTools* from Python scripts is easy. To begin, each script must start by importing the *WhiteboxTools* class, contained with the *whitebox_tools.py* script; a new ```WhiteboxTools``` object can then be created:

```Python

from whitebox_tools import WhiteboxTools

wbt = WhiteboxTools() 


```

The `WhiteboxTools` class expects to find the *WhiteboxTools* executable file (*whitebox_tools.exe* on Windows and *whitebox_tools* on other platforms) within the same directory as the *whitebox_tools.py* script. If the binary file is located in a separate directory, you will need to set the executable directory as follows:

```Python

wbt.set_whitebox_dir('/local/path/to/whitebox/binary/')  
# Or alternatively...
wbt.exe_path = '/local/path/to/whitebox/binary/'


```

Individual tools can be called using the convenience methods provided in the `WhiteboxTools` class:

```Python

# This line performs a 5 x 5 mean filter on 'inFile.tif':
wbt.mean_filter('/file/path/inFile.tif', '/file/path/outFile.tif', 5, 5)


```

Each tool has a cooresponding convenience method. Tools can also be called using the `run_tool()` method, specifying the tool name and a list of tool arguments. Each of the tool-specific convenience methods collect their arguments into a properly formated list and then ultimately call the `run_tools()` method. Notice that while internally *whitebox_tools.exe* uses CamelCase (e.g. MeanFilter) to denote tool names, the Python interface of *whitebox_tools.py* uses snake_case (e.g. mean_filter), according to Python style conventions. The only exceptions are tools with names that clash with Python keywords (e.g. `And()`, `Not()`, and `Or()`).

The return value can be used to check for errors during operation:

```Python

if wbt.ruggedness_index('/path/DEM.flt', '/path/ruggedness.flt') != 0:
    # Non-zero returns indicate an error.
    print('ERROR running ruggedness_index')


```

If, like me, your data files tend to be burried deeply in layers of sub-directories, specifying complete file names as input parameters can be tedius. In this case, the best option is setting the working directory before calling tools:

```Python

from whitebox_tools import WhiteboxTools

wbt = WhiteboxTools() 
wbt.work_dir = "/path/to/data/" # Sets the Whitebox working directory

# Because the working directory has been set, file arguments can be
# specified simply using file names, without paths.
wbt.d_inf_flow_accumulation("DEM.dep", "output.dep", log=True)


```

An advanced text editor, such as VS Code or Atom, can provide hints and autocompletion for available tool convenience methods and their parameters, including default values. 

![Autocompletion in Atom text editor makes calling *WhiteboxTools* functions easier.](./img/wbt_auotcomplete.png)

Sometimes, however, it can be useful to print a complete list of available tools:

```Python

print(wbt.list_tools()) # List all tools in WhiteboxTools


```

The `list_tools()` method also takes an optional keywords list to search for tools:

```Python

# Lists tools with 'lidar' or 'LAS' in tool name or description.
print(wbt.list_tools(['lidar', 'LAS']))


```

To retrieve more detailed information for a specific tool, use the `tool_help()` method:

```Python

print(wbt.tool_help("elev_percentile"))


```

`tool_help()` prints tool details including a description, tool parameters (and their flags), and example usage at the command line prompt. The above statement prints this report:

```

ElevPercentile
Description:
Calculates the elevation percentile raster from a DEM.
Toolbox: Geomorphometric Analysis
Parameters:

Flag               Description
-----------------  -----------
-i, --input, --dem Input raster DEM file.
-o, --output       Output raster file.
--filterx          Size of the filter kernel in the x-direction.
--filtery          Size of the filter kernel in the y-direction.
--sig_digits       Number of significant digits.

Example usage:
>>./whitebox_tools -r=ElevPercentile -v --wd="/path/to/data/" --dem=DEM.dep 
>>-o=output.dep --filterx=25


````

Tools will frequently print text to the standard output during their execution, including warnings, progress updates and other notifications. Sometimes, when users run many tools in complex workflows and in batch mode, these output messages can be undesirable. Most tools will have their outputs suppressed by setting the *verbose* mode to *False* as follows:

```Python

wbt.set_verbose_mode(False) 
# Or, alternatively...
wbt.verbose = False


```

Alternatively, it may be helpful to capture the text output of a tool for custom processing. This is achieved by specifying a custom *callback* function to the tool's run method:

```Python

# This callback function suppresses printing progress updates,
# which always use the '%' character. The callback function 
# approach is flexible and allows for any level of complex
# interaction with tool outputs.
def my_callback(value):
    if not "%" in value:
        print(value)

wbt.slope('DEM.tif', 'slope_raster.tif', callback=my_callback)


```

Callback functions can also serve as a means of cancelling operations:

```Python

def my_callback(value):
    if user_selected_cancel_btn: # Assumes a 'Cancel' button on a GUI
        print('Cancelling operation...')
        wbt.cancel_op = True
    else:
        print(value)

wbt.breach_depressions('DEM.tif', 'DEM_breached.tif', callback=my_callback)


```

The *whitebox_tools.py* script provides several other functions for interacting with the *WhiteboxTools* library, including: 

```Python

# Print the WhiteboxTools help...a listing of available commands
print(wbt.help())

# Print the WhiteboxTools license
print(wbt.license())

# Print the WhiteboxTools version
print("Version information: {}".format(wbt.version()))

# Get the toolbox associated with a tool
tb = wbt.toolbox('lidar_info')

# Retrieve a JSON object of a tool's parameters.
tp = tool_parameters('raster_histogram')

# Opens a browser and navigates to a tool's source code in the 
# WhiteboxTools GitHub repository
wbt.view_code('watershed')


```

For a working example of how to call functions and run tools from Python, see the *whitebox_example.py* Python script, which is distributed with the *WhiteboxTools* library.


## 5. WhiteboxTools Runner

There is a Python script contained within the *WhiteboxTools* directory called '*wb_runner.py*'. This script is intended to provide a very basic user-interface, *WhiteboxTools Runner*, for running the tools contained within the *WhiteboxTools* library. The user-interface uses Python's TkInter GUI library and is cross-platform. The user interface is currently experimental and is under heavy testing. Please report any issues that you experience in using it.

![The *WhiteboxTools Runner* user-interface](./img/WBRunner.png)

The *WhiteboxTools Runner* does not rely on the *Whitebox GAT* user interface at all and can therefore be used indepedent of the larger project. The script must be run from a directory that also contains the '*whitebox_tools.py*' Python script and the '*whitebox_tools*' executable file. There are plans to link tool help documentation in *WhiteboxTools Runner* and to incorporate toolbox information, rather than one large listing of available tools.

## 6. Available Tools

Eventually most of *Whitebox GAT's* approximately 400 tools [will be ported](tool_porting.md) to *WhiteboxTools*, although this is an immense task. Support for vector data (Shapefile/GeoJSON) reading/writing and a topological analysis library (like the Java Topology Suite) will need to be added in order to port the tools involving vector spatial data. Opportunities to parallelize algorithms will be sought during porting. All new plugin tools will be added to *Whitebox GAT* using this library of functions. 

The library currently contains the following 279 tools, which are each grouped based on their main function into one of the following categories: *Data Tools*, *Geomorphometric Analysis* (i.e. digital terrain analysis), *GIS Analysis*, *Hydrological Analysis*, *Image Analysis*, *LiDAR Analysis*, *Mathematical and Statistical Analysis*, and *Stream Network Analysis*. To retrieve detailed information about a tool's input arguments and example usage, either use the *-\-toolhelp* command from the terminal, or the *tool_help('tool_name')* function from the *whitebox_tools.py* script. The following is a complete listing of available tools, with brief descriptions, tool parameter, and example usage.








### 6.1 Data Tools

#### 6.1.1 ConvertNodataToZero

Converts nodata values in a raster to zero.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=ConvertNodataToZero -v ^
--wd="/path/to/data/" --input=in.dep -o=NewRaster.dep 


```

*Python function name*: ```convert_nodata_to_zero```


#### 6.1.2 ConvertRasterFormat

Converts raster data from one format to another.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=ConvertRasterFormat -v ^
--wd="/path/to/data/" --input=DEM.dep -o=output.dep 


```

*Python function name*: ```convert_raster_format```


#### 6.1.3 NewRasterFromBase

Creates a new raster using a base image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-base          Input base raster file
-o, -\-output        Output raster file
-\-value             Constant value to fill raster with; either 'nodata' or numeric value
-\-data_type         Output raster data type; options include 'double' (64-bit), 'float' (32-bit), 
                     and 'integer' (signed 16-bit) (default is 'float') 

*Command-line Interface*:
```
>>./whitebox_tools -r=NewRasterFromBase -v ^
--wd="/path/to/data/" --base=base.dep -o=NewRaster.dep ^
--value=0.0 --data_type=integer
>>./whitebox_tools ^
-r=NewRasterFromBase -v --wd="/path/to/data/" --base=base.dep ^
-o=NewRaster.dep --value=nodata 


```

*Python function name*: ```new_raster_from_base```


#### 6.1.4 PrintGeoTiffTags

Prints the tags within a GeoTIFF.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input GeoTIFF file

*Command-line Interface*:
```
>>./whitebox_tools -r=PrintGeoTiffTags -v ^
--wd="/path/to/data/" --input=DEM.tiff 


```

*Python function name*: ```print_geo_tiff_tags```


#### 6.1.5 SetNodataValue

Assign a specified value in an input image to the NoData value.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-back_value        Background value to set to nodata

*Command-line Interface*:
```
>>./whitebox_tools -r=SetNodataValue -v --wd="/path/to/data/" ^
-i=in.dep -o=newRaster.dep --back_value=1.0 


```

*Python function name*: ```set_nodata_value```

### 6.2 GIS Analysis

#### 6.2.1 AggregateRaster

Aggregates a raster to a lower resolution.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-agg_factor        Aggregation factor, in pixels
-\-type              Statistic used to fill output pixels

*Command-line Interface*:
```
>>./whitebox_tools -r=AggregateRaster -v ^
--wd="/path/to/data/" -i=input.dep -o=output.dep ^
--output_text 


```

*Python function name*: ```aggregate_raster```


#### 6.2.2 Centroid

Calculates the centroid, or average location, of raster polygon objects.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-text_output       Optional text output

*Command-line Interface*:
```
>>./whitebox_tools -r=Centroid -v --wd="/path/to/data/" ^
-i=polygons.dep -o=output.dep
>>./whitebox_tools -r=Centroid ^
-v --wd="/path/to/data/" -i=polygons.dep -o=output.dep ^
--text_output 


```

*Python function name*: ```centroid```


#### 6.2.3 Clump

Groups cells that form physically discrete areas, assigning them unique identifiers.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-diag              Flag indicating whether diagonal connections should be considered
-\-zero_back         Flag indicating whether zero values should be treated as a background

*Command-line Interface*:
```
>>./whitebox_tools -r=Clump -v --wd="/path/to/data/" ^
-i=input.dep -o=output.dep --diag 


```

*Python function name*: ```clump```


#### 6.2.4 CreatePlane

Creates a raster image based on the equation for a simple plane.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-base              Input base raster file
-o, -\-output        Output raster file
-\-gradient          Slope gradient in degrees (-85.0 to 85.0)
-\-aspect            Aspect (direction) in degrees clockwise from north (0.0-360.0)
-\-constant          Constant value

*Command-line Interface*:
```
>>./whitebox_tools -r=CreatePlane -v --wd="/path/to/data/" ^
--base=base.dep -o=NewRaster.dep --gradient=15.0 ^
--aspect=315.0 


```

*Python function name*: ```create_plane```


#### 6.2.5 RadiusOfGyration

Calculates the distance of cells from their polygon's centroid.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-text_output       Optional text output

*Command-line Interface*:
```
>>./whitebox_tools -r=RadiusOfGyration -v ^
--wd="/path/to/data/" -i=polygons.dep -o=output.dep ^
--text_output 


```

*Python function name*: ```radius_of_gyration```


#### 6.2.6 RasterCellAssignment

Assign row or column number to cells.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-a, -\-assign        Which variable would you like to assign to grid cells? Options include 
                     'column', 'row', 'x', and 'y' 

*Command-line Interface*:
```
>>./whitebox_tools -r=RasterCellAssignment -v ^
--wd="/path/to/data/" -i='input.dep' -o=output.dep ^
--assign='column' 


```

*Python function name*: ```raster_cell_assignment```

### 6.3 GIS Analysis => Distance Tools

#### 6.3.1 BufferRaster

Maps a distance-based buffer around each non-background (non-zero/non-nodata) grid cell in an input image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-size              Buffer size
-\-gridcells         Optional flag to indicate that the 'size' threshold should be measured in grid 
                     cells instead of the default map units 

*Command-line Interface*:
```
>>./whitebox_tools -r=BufferRaster -v --wd="/path/to/data/" ^
-i=DEM.dep -o=output.dep 


```

*Python function name*: ```buffer_raster```


#### 6.3.2 CostAllocation

Identifies the source cell to which each grid cell is connected by a least-cost pathway in a cost-distance analysis.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-source            Input source raster file
-\-backlink          Input backlink raster file generated by the cost-distance tool
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=CostAllocation -v --wd="/path/to/data/" ^
--source='source.dep' --backlink='backlink.dep' ^
-o='output.dep' 


```

*Python function name*: ```cost_allocation```


#### 6.3.3 CostDistance

Performs cost-distance accumulation on a cost surface and a group of source cells.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-source            Input source raster file
-\-cost              Input cost (friction) raster file
-\-out_accum         Output cost accumulation raster file
-\-out_backlink      Output backlink raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=CostDistance -v --wd="/path/to/data/" ^
--source=src.dep --cost=cost.dep --out_accum=accum.dep ^
--out_backlink=backlink.dep 


```

*Python function name*: ```cost_distance```


#### 6.3.4 CostPathway

Performs cost-distance pathway analysis using a series of destination grid cells.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-destination       Input destination raster file
-\-backlink          Input backlink raster file generated by the cost-distance tool
-o, -\-output        Output cost pathway raster file
-\-zero_background   Flag indicating whether zero values should be treated as a background

*Command-line Interface*:
```
>>./whitebox_tools -r=CostPathway -v --wd="/path/to/data/" ^
--destination=dst.dep --backlink=backlink.dep ^
--output=cost_path.dep 


```

*Python function name*: ```cost_pathway```


#### 6.3.5 EuclideanAllocation

Assigns grid cells in the output raster the value of the nearest target cell in the input image, measured by the Shih and Wu (2004) Euclidean distance transform.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=EuclideanAllocation -v ^
--wd="/path/to/data/" -i=DEM.dep -o=output.dep 


```

*Python function name*: ```euclidean_allocation```


#### 6.3.6 EuclideanDistance

Calculates the Shih and Wu (2004) Euclidean distance transform.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=EuclideanDistance -v ^
--wd="/path/to/data/" -i=DEM.dep -o=output.dep 


```

*Python function name*: ```euclidean_distance```

### 6.4 GIS Analysis => Overlay Tools

#### 6.4.1 AverageOverlay

Calculates the average for each grid cell from a group of raster images.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input raster files
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=AverageOverlay -v --wd='/path/to/data/' ^
-i='image1.dep;image2.dep;image3.dep' -o=output.dep 


```

*Python function name*: ```average_overlay```


#### 6.4.2 HighestPosition

Identifies the stack position of the maximum value within a raster stack on a cell-by-cell basis.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input raster files
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=HighestPosition -v ^
--wd='/path/to/data/' -i='image1.dep;image2.dep;image3.dep' ^
-o=output.dep 


```

*Python function name*: ```highest_position```


#### 6.4.3 LowestPosition

Identifies the stack position of the minimum value within a raster stack on a cell-by-cell basis.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input raster files
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=LowestPosition -v --wd='/path/to/data/' ^
-i='image1.dep;image2.dep;image3.dep' -o=output.dep 


```

*Python function name*: ```lowest_position```


#### 6.4.4 MaxAbsoluteOverlay

Evaluates the maximum absolute value for each grid cell from a stack of input rasters.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input raster files
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=MaxAbsoluteOverlay -v ^
--wd='/path/to/data/' -i='image1.dep;image2.dep;image3.dep' ^
-o=output.dep 


```

*Python function name*: ```max_absolute_overlay```


#### 6.4.5 MaxOverlay

Evaluates the maximum value for each grid cell from a stack of input rasters.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input raster files
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=MaxOverlay -v --wd='/path/to/data/' ^
-i='image1.dep;image2.dep;image3.dep' -o=output.dep 


```

*Python function name*: ```max_overlay```


#### 6.4.6 MinAbsoluteOverlay

Evaluates the minimum absolute value for each grid cell from a stack of input rasters.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input raster files
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=MinAbsoluteOverlay -v ^
--wd='/path/to/data/' -i='image1.dep;image2.dep;image3.dep' ^
-o=output.dep 


```

*Python function name*: ```min_absolute_overlay```


#### 6.4.7 MinOverlay

Evaluates the minimum value for each grid cell from a stack of input rasters.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input raster files
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=MinOverlay -v --wd='/path/to/data/' ^
-i='image1.dep;image2.dep;image3.dep' -o=output.dep 


```

*Python function name*: ```min_overlay```


#### 6.4.8 PercentEqualTo

Calculates the percentage of a raster stack that have cell values equal to an input on a cell-by-cell basis.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input raster files
-\-comparison        Input comparison raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=PercentEqualTo -v --wd='/path/to/data/' ^
-i='image1.dep;image2.dep;image3.dep' --comparison='comp.dep' ^
-o='output.dep' 


```

*Python function name*: ```percent_equal_to```


#### 6.4.9 PercentGreaterThan

Calculates the percentage of a raster stack that have cell values greather than an input on a cell-by-cell basis.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input raster files
-\-comparison        Input comparison raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=PercentGreaterThan -v ^
--wd='/path/to/data/' -i='image1.dep;image2.dep;image3.dep' ^
--comparison='comp.dep' -o='output.dep' 


```

*Python function name*: ```percent_greater_than```


#### 6.4.10 PercentLessThan

Calculates the percentage of a raster stack that have cell values less than an input on a cell-by-cell basis.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input raster files
-\-comparison        Input comparison raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=PercentLessThan -v ^
--wd='/path/to/data/' -i='image1.dep;image2.dep;image3.dep' ^
--comparison='comp.dep' -o='output.dep' 


```

*Python function name*: ```percent_less_than```


#### 6.4.11 PickFromList

Outputs the value from a raster stack specified by a position raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input raster files
-\-pos_input         Input position raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=PickFromList -v --wd='/path/to/data/' ^
--pos_input=position.dep -i='image1.dep;image2.dep;image3.dep' ^
-o=output.dep 


```

*Python function name*: ```pick_from_list```


#### 6.4.12 WeightedSum

Performs a weighted-sum overlay on multiple input raster images.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input raster files
-o, -\-output        Output raster file
-w, -\-weights       Weight values, contained in quotes and separated by commas or semicolons

*Command-line Interface*:
```
>>./whitebox_tools -r=WeightedSum -v --wd='/path/to/data/' ^
-i='image1.dep;image2.dep;image3.dep' -o=output.dep ^
--weights='0.3;0.2;0.5' 


```

*Python function name*: ```weighted_sum```

### 6.5 GIS Analysis => Patch Shape Tools

#### 6.5.1 EdgeProportion

Calculate the proportion of cells in a raster polygon that are edge cells.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-output_text       flag indicating whether a text report should also be output

*Command-line Interface*:
```
>>./whitebox_tools -r=EdgeProportion -v --wd="/path/to/data/" ^
-i=input.dep -o=output.dep --output_text 


```

*Python function name*: ```edge_proportion```


#### 6.5.2 FindPatchOrClassEdgeCells

Finds all cells located on the edge of patch or class features.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=FindPatchOrClassEdgeCells -v ^
--wd="/path/to/data/" -i=input.dep -o=output.dep 


```

*Python function name*: ```find_patch_or_class_edge_cells```

### 6.6 GIS Analysis => Reclass Tools

#### 6.6.1 Reclass

Reclassifies the values in a raster image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-reclass_vals      Reclassification triplet values (new value; from value; to less than), e.g. 
                     '0.0;0.0;1.0;1.0;1.0;2.0' 
-\-assign_mode       Optional Boolean flag indicating whether to operate in assign mode, 
                     reclass_vals values are interpreted as new value; old value pairs 

*Command-line Interface*:
```
>>./whitebox_tools -r=Reclass -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep ^
--reclass_vals='0.0;0.0;1.0;1.0;1.0;2.0'
>>./whitebox_tools ^
-r=Reclass -v --wd="/path/to/data/" -i='input.dep' ^
-o=output.dep --reclass_vals='10;1;20;2;30;3;40;4' ^
--assign_mode 


```

*Python function name*: ```reclass```


#### 6.6.2 ReclassEqualInterval

Reclassifies the values in a raster image based on equal-ranges.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-interval          Class interval size
-\-start_val         Optional starting value (default is input minimum value)
-\-end_val           Optional ending value (default is input maximum value)

*Command-line Interface*:
```
>>./whitebox_tools -r=ReclassEqualInterval -v ^
--wd="/path/to/data/" -i='input.dep' -o=output.dep ^
--interval=10.0 --start_val=0.0 


```

*Python function name*: ```reclass_equal_interval```


#### 6.6.3 ReclassFromFile

Reclassifies the values in a raster image using reclass ranges in a text file.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-\-reclass_file      Input text file containing reclass ranges
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=ReclassFromFile -v ^
--wd="/path/to/data/" -i='input.dep' ^
--reclass_file='reclass.txt' -o=output.dep 


```

*Python function name*: ```reclass_from_file```

### 6.7 Geomorphometric Analysis

#### 6.7.1 Aspect

Calculates an aspect raster from an input DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-zfactor           Optional multiplier for when the vertical and horizontal units are not the same

*Command-line Interface*:
```
>>./whitebox_tools -r=Aspect -v --wd="/path/to/data/" ^
--dem=DEM.dep -o=output.dep 


```

*Python function name*: ```aspect```


#### 6.7.2 DevFromMeanElev

Calculates deviation from mean elevation.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input, -\-dem Input raster DEM file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction

*Command-line Interface*:
```
>>./whitebox_tools -r=DevFromMeanElev -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=output.dep ^
--filter=25 


```

*Python function name*: ```dev_from_mean_elev```


#### 6.7.3 DiffFromMeanElev

Calculates difference from mean elevation (equivalent to a high-pass filter).

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input, -\-dem Input raster DEM file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction

*Command-line Interface*:
```
>>./whitebox_tools -r=DiffFromMeanElev -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=output.dep ^
--filter=25 


```

*Python function name*: ```diff_from_mean_elev```


#### 6.7.4 DirectionalRelief

Calculates relief for cells in an input DEM for a specified direction.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-azimuth           Wind azimuth in degrees
-\-max_dist          Optional maximum search distance (unspecified if none; in xy units)

*Command-line Interface*:
```
>>./whitebox_tools -r=DirectionalRelief -v ^
--wd="/path/to/data/" -i='input.dep' -o=output.dep ^
--azimuth=315.0 


```

*Python function name*: ```directional_relief```


#### 6.7.5 DownslopeIndex

Calculates the Hjerdt et al. (2004) downslope index.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-drop              Vertical drop value (default is 2.0)
-\-out_type          Output type, options include 'tangent', 'degrees', 'radians', 'distance' 
                     (default is 'tangent') 

*Command-line Interface*:
```
>>./whitebox_tools -r=DownslopeIndex -v --wd="/path/to/data/" ^
--dem=pointer.dep -o=dsi.dep --drop=5.0 --out_type=distance 


```

*Python function name*: ```downslope_index```


#### 6.7.6 ElevAbovePit

Calculate the elevation of each grid cell above the nearest downstream pit cell or grid edge cell.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=ElevAbovePit -v --wd="/path/to/data/" ^
--dem=DEM.dep -o=output.dep 


```

*Python function name*: ```elev_above_pit```


#### 6.7.7 ElevPercentile

Calculates the elevation percentile raster from a DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input, -\-dem Input raster DEM file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction
-\-sig_digits        Number of significant digits

*Command-line Interface*:
```
>>./whitebox_tools -r=ElevPercentile -v --wd="/path/to/data/" ^
--dem=DEM.dep -o=output.dep --filter=25 


```

*Python function name*: ```elev_percentile```


#### 6.7.8 ElevRelativeToMinMax

Calculates the elevation of a location relative to the minimum and maximum elevations in a DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=ElevRelativeToMinMax -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=output.dep 


```

*Python function name*: ```elev_relative_to_min_max```


#### 6.7.9 ElevRelativeToWatershedMinMax

Calculates the elevation of a location relative to the minimum and maximum elevations in a watershed.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-\-watersheds        Input raster watersheds file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=ElevRelativeToWatershedMinMax -v ^
--wd="/path/to/data/" --dem=DEM.dep --watersheds=watershed.dep ^
-o=output.dep 


```

*Python function name*: ```elev_relative_to_watershed_min_max```


#### 6.7.10 FeaturePreservingDenoise

Reduces short-scale variation in an input DEM using a modified Sun et al. (2007) algorithm.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-filter            Size of the filter kernel
-\-norm_diff         Maximum difference in normal vectors, in degrees
-\-num_iter          Number of iterations
-\-zfactor           Optional multiplier for when the vertical and horizontal units are not the same

*Command-line Interface*:
```
>>./whitebox_tools -r=FeaturePreservingDenoise -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=output.dep 


```

*Python function name*: ```feature_preserving_denoise```


#### 6.7.11 FetchAnalysis

Performs an analysis of fetch or upwind distance to an obstacle.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-azimuth           Wind azimuth in degrees in degrees
-\-hgt_inc           Height increment value

*Command-line Interface*:
```
>>./whitebox_tools -r=FetchAnalysis -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep --azimuth=315.0 


```

*Python function name*: ```fetch_analysis```


#### 6.7.12 FillMissingData

Fills nodata holes in a DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-filter            Filter size (cells)

*Command-line Interface*:
```
>>./whitebox_tools -r=FillMissingData -v ^
--wd="/path/to/data/" -i=DEM.dep -o=output.dep --filter=25 


```

*Python function name*: ```fill_missing_data```


#### 6.7.13 FindRidges

Identifies potential ridge and peak grid cells.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-line_thin         Optional flag indicating whether post-processing line-thinning should be 
                     performed 

*Command-line Interface*:
```
>>./whitebox_tools -r=FindRidges -v --wd="/path/to/data/" ^
--dem=pointer.dep -o=out.dep --line_thin 


```

*Python function name*: ```find_ridges```


#### 6.7.14 Hillshade

Calculates a hillshade raster from an input DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-azimuth           Illumination source azimuth in degrees
-\-altitude          Illumination source altitude in degrees
-\-zfactor           Optional multiplier for when the vertical and horizontal units are not the same

*Command-line Interface*:
```
>>./whitebox_tools -r=Hillshade -v --wd="/path/to/data/" ^
-i=DEM.dep -o=output.dep --azimuth=315.0 --altitude=30.0 


```

*Python function name*: ```hillshade```


#### 6.7.15 HorizonAngle

Calculates horizon angle (maximum upwind slope) for each grid cell in an input DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-azimuth           Wind azimuth in degrees
-\-max_dist          Optional maximum search distance (unspecified if none; in xy units)

*Command-line Interface*:
```
>>./whitebox_tools -r=HorizonAngle -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep --azimuth=315.0 


```

*Python function name*: ```horizon_angle```


#### 6.7.16 HypsometricAnalysis

Calculates a hypsometric curve for one or more DEMs.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input DEM files
-\-watershed         Input watershed files (optional)
-o, -\-output        Output HTML file (default name will be based on input file if unspecified)

*Command-line Interface*:
```
>>./whitebox_tools -r=HypsometricAnalysis -v ^
--wd="/path/to/data/" -i="DEM1.tif;DEM2.tif" ^
--watershed="ws1.tif;ws2.tif" -o=outfile.html 


```

*Python function name*: ```hypsometric_analysis```


#### 6.7.17 MaxAnisotropyDev

Calculates the maximum anisotropy (directionality) in elevation deviation over a range of spatial scales.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-\-out_mag           Output raster DEVmax magnitude file
-\-out_scale         Output raster DEVmax scale file
-\-min_scale         Minimum search neighbourhood radius in grid cells
-\-max_scale         Maximum search neighbourhood radius in grid cells
-\-step              Step size as any positive non-zero integer

*Command-line Interface*:
```
>>./whitebox_tools -r=MaxAnisotropyDev -v ^
--wd="/path/to/data/" --dem=DEM.dep --out_mag=DEVmax_mag.dep ^
--out_scale=DEVmax_scale.dep --min_scale=1 --max_scale=1000 ^
--step=5 


```

*Python function name*: ```max_anisotropy_dev```


#### 6.7.18 MaxBranchLength

Lindsay and Seibert's (2013) branch length index is used to map drainage divides or ridge lines.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-log               Optional flag to request the output be log-transformed

*Command-line Interface*:
```
>>./whitebox_tools -r=MaxBranchLength -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=output.dep 


```

*Python function name*: ```max_branch_length```


#### 6.7.19 MaxDownslopeElevChange

Calculates the maximum downslope change in elevation between a grid cell and its eight downslope neighbors.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=MaxDownslopeElevChange -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=out.dep 


```

*Python function name*: ```max_downslope_elev_change```


#### 6.7.20 MaxElevDevSignature

Calculates the maximum elevation deviation over a range of spatial scales and for a set of points.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-\-points            Input vector points file
-o, -\-output        Output HTML file
-\-min_scale         Minimum search neighbourhood radius in grid cells
-\-max_scale         Maximum search neighbourhood radius in grid cells
-\-step              Step size as any positive non-zero integer

*Command-line Interface*:
```
>>./whitebox_tools -r=MaxElevDevSignature -v ^
--wd="/path/to/data/" --dem=DEM.dep --points=sites.shp ^
--output=topo_position.html --min_scale=1 --max_scale=1000 ^
--step=5 


```

*Python function name*: ```max_elev_dev_signature```


#### 6.7.21 MaxElevationDeviation

Calculates the maximum elevation deviation over a range of spatial scales.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-\-out_mag           Output raster DEVmax magnitude file
-\-out_scale         Output raster DEVmax scale file
-\-min_scale         Minimum search neighbourhood radius in grid cells
-\-max_scale         Maximum search neighbourhood radius in grid cells
-\-step              Step size as any positive non-zero integer

*Command-line Interface*:
```
>>./whitebox_tools -r=MaxElevationDeviation -v ^
--wd="/path/to/data/" --dem=DEM.dep --out_mag=DEVmax_mag.dep ^
--out_scale=DEVmax_scale.dep --min_scale=1 --max_scale=1000 ^
--step=5 


```

*Python function name*: ```max_elevation_deviation```


#### 6.7.22 MinDownslopeElevChange

Calculates the minimum downslope change in elevation between a grid cell and its eight downslope neighbors.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=MinDownslopeElevChange -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=out.dep 


```

*Python function name*: ```min_downslope_elev_change```


#### 6.7.23 MultiscaleRoughness

Calculates surface roughness over a range of spatial scales.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-\-out_mag           Output raster roughness magnitude file
-\-out_scale         Output raster roughness scale file
-\-min_scale         Minimum search neighbourhood radius in grid cells
-\-max_scale         Maximum search neighbourhood radius in grid cells
-\-step              Step size as any positive non-zero integer

*Command-line Interface*:
```
>>./whitebox_tools -r=MultiscaleRoughness -v ^
--wd="/path/to/data/" --dem=DEM.dep --out_mag=roughness_mag.dep ^
--out_scale=roughness_scale.dep --min_scale=1 --max_scale=1000 ^
--step=5 


```

*Python function name*: ```multiscale_roughness```


#### 6.7.24 MultiscaleRoughnessSignature

Calculates the surface roughness for points over a range of spatial scales.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-\-points            Input vector points file
-o, -\-output        Output HTML file
-\-min_scale         Minimum search neighbourhood radius in grid cells
-\-max_scale         Maximum search neighbourhood radius in grid cells
-\-step              Step size as any positive non-zero integer

*Command-line Interface*:
```
>>./whitebox_tools -r=MultiscaleRoughnessSignature -v ^
--wd="/path/to/data/" --dem=DEM.dep --points=sites.shp ^
--output=roughness.html --min_scale=1 --max_scale=1000 ^
--step=5 


```

*Python function name*: ```multiscale_roughness_signature```


#### 6.7.25 MultiscaleTopographicPositionImage

Creates a multiscale topographic position image from three DEVmax rasters of differing spatial scale ranges.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-local             Input local-scale topographic position (DEVmax) raster file
-\-meso              Input meso-scale topographic position (DEVmax) raster file
-\-broad             Input broad-scale topographic position (DEVmax) raster file
-o, -\-output        Output raster file
-\-lightness         Image lightness value (default is 1.2)

*Command-line Interface*:
```
>>./whitebox_tools -r=MultiscaleTopographicPositionImage -v ^
--wd="/path/to/data/" --local=DEV_local.dep --meso=DEV_meso.dep ^
--broad=DEV_broad.dep -o=output.dep --lightness=1.5 


```

*Python function name*: ```multiscale_topographic_position_image```


#### 6.7.26 NumDownslopeNeighbours

Calculates the number of downslope neighbours to each grid cell in a DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=NumDownslopeNeighbours -v ^
--wd="/path/to/data/" -i=DEM.dep -o=output.dep 


```

*Python function name*: ```num_downslope_neighbours```


#### 6.7.27 NumUpslopeNeighbours

Calculates the number of upslope neighbours to each grid cell in a DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=NumUpslopeNeighbours -v ^
--wd="/path/to/data/" -i=DEM.dep -o=output.dep 


```

*Python function name*: ```num_upslope_neighbours```


#### 6.7.28 PennockLandformClass

Classifies hillslope zones based on slope, profile curvature, and plan curvature.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-slope             Slope threshold value, in degrees (default is 3.0)
-\-prof              Profile curvature threshold value (default is 0.1)
-\-plan              Plan curvature threshold value (default is 0.0)
-\-zfactor           Optional multiplier for when the vertical and horizontal units are not the same

*Command-line Interface*:
```
>>./whitebox_tools -r=PennockLandformClass -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=output.dep --slope=3.0 ^
--prof=0.1 --plan=0.0 


```

*Python function name*: ```pennock_landform_class```


#### 6.7.29 PercentElevRange

Calculates percent of elevation range from a DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input, -\-dem Input raster DEM file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction

*Command-line Interface*:
```
>>./whitebox_tools -r=PercentElevRange -v ^
--wd="/path/to/data/" -i=DEM.dep -o=output.dep --filter=25 


```

*Python function name*: ```percent_elev_range```


#### 6.7.30 PlanCurvature

Calculates a plan (contour) curvature raster from an input DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-zfactor           Optional multiplier for when the vertical and horizontal units are not the same

*Command-line Interface*:
```
>>./whitebox_tools -r=PlanCurvature -v --wd="/path/to/data/" ^
--dem=DEM.dep -o=output.dep 


```

*Python function name*: ```plan_curvature```


#### 6.7.31 Profile

Plots profiles from digital surface models.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-lines             Input vector line file
-\-surface           Input raster surface file
-o, -\-output        Output HTML file

*Command-line Interface*:
```
>>./whitebox_tools -r=Profile -v --wd="/path/to/data/" ^
--lines=profile.shp --surface=dem.dep -o=profile.html 


```

*Python function name*: ```profile```


#### 6.7.32 ProfileCurvature

Calculates a profile curvature raster from an input DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-zfactor           Optional multiplier for when the vertical and horizontal units are not the same

*Command-line Interface*:
```
>>./whitebox_tools -r=ProfileCurvature -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=output.dep 


```

*Python function name*: ```profile_curvature```


#### 6.7.33 RelativeAspect

Calculates relative aspect (relative to a user-specified direction) from an input DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-azimuth           Illumination source azimuth
-\-zfactor           Optional multiplier for when the vertical and horizontal units are not the same

*Command-line Interface*:
```
>>./whitebox_tools -r=RelativeAspect -v --wd="/path/to/data/" ^
--dem=DEM.dep -o=output.dep --azimuth=180.0 


```

*Python function name*: ```relative_aspect```


#### 6.7.34 RelativeStreamPowerIndex

Calculates the relative stream power index.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-sca               Input raster specific contributing area (SCA) file
-\-slope             Input raster slope file
-o, -\-output        Output raster file
-\-exponent          SCA exponent value

*Command-line Interface*:
```
>>./whitebox_tools -r=RelativeStreamPowerIndex -v ^
--wd="/path/to/data/" --sca='flow_accum.dep' ^
--slope='slope.dep' -o=output.dep --exponent=1.1 


```

*Python function name*: ```relative_stream_power_index```


#### 6.7.35 RelativeTopographicPosition

Calculates the relative topographic position index from a DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction

*Command-line Interface*:
```
>>./whitebox_tools -r=RelativeTopographicPosition -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=output.dep ^
--filter=25 


```

*Python function name*: ```relative_topographic_position```


#### 6.7.36 RemoveOffTerrainObjects

Removes off-terrain objects from a raster digital elevation model (DEM).

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input, -\-dem Input raster DEM file
-o, -\-output        Output raster file
-\-filter            Filter size (cells)
-\-slope             Slope threshold value

*Command-line Interface*:
```
>>./whitebox_tools -r=RemoveOffTerrainObjects -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=bare_earth_DEM.dep ^
--filter=25 --slope=10.0 


```

*Python function name*: ```remove_off_terrain_objects```


#### 6.7.37 RuggednessIndex

Calculates the Riley et al.'s (1999) terrain ruggedness index from an input DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-zfactor           Optional multiplier for when the vertical and horizontal units are not the same

*Command-line Interface*:
```
>>./whitebox_tools -r=RuggednessIndex -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=output.dep 


```

*Python function name*: ```ruggedness_index```


#### 6.7.38 SedimentTransportIndex

Calculates the sediment transport index.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-sca               Input raster specific contributing area (SCA) file
-\-slope             Input raster slope file
-o, -\-output        Output raster file
-\-sca_exponent      SCA exponent value
-\-slope_exponent    Slope exponent value

*Command-line Interface*:
```
>>./whitebox_tools -r=SedimentTransportIndex -v ^
--wd="/path/to/data/" --sca='flow_accum.dep' ^
--slope='slope.dep' -o=output.dep --sca_exponent=0.5 ^
--slope_exponent=1.0 


```

*Python function name*: ```sediment_transport_index```


#### 6.7.39 Slope

Calculates a slope raster from an input DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-zfactor           Optional multiplier for when the vertical and horizontal units are not the same

*Command-line Interface*:
```
>>./whitebox_tools -r=Slope -v --wd="/path/to/data/" ^
--dem=DEM.dep -o=output.dep 


```

*Python function name*: ```slope```


#### 6.7.40 SlopeVsElevationPlot

Creates a slope vs. elevation plot for one or more DEMs.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input DEM files
-\-watershed         Input watershed files (optional)
-o, -\-output        Output HTML file (default name will be based on input file if unspecified)

*Command-line Interface*:
```
>>./whitebox_tools -r=SlopeVsElevationPlot -v ^
--wd="/path/to/data/" -i="DEM1.tif;DEM2.tif" ^
--watershed="ws1.tif;ws2.tif" -o=outfile.html 


```

*Python function name*: ```slope_vs_elevation_plot```


#### 6.7.41 TangentialCurvature

Calculates a tangential curvature raster from an input DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-zfactor           Optional multiplier for when the vertical and horizontal units are not the same

*Command-line Interface*:
```
>>./whitebox_tools -r=TangentialCurvature -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=output.dep 


```

*Python function name*: ```tangential_curvature```


#### 6.7.42 TotalCurvature

Calculates a total curvature raster from an input DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-zfactor           Optional multiplier for when the vertical and horizontal units are not the same

*Command-line Interface*:
```
>>./whitebox_tools -r=TotalCurvature -v --wd="/path/to/data/" ^
--dem=DEM.dep -o=output.dep 


```

*Python function name*: ```total_curvature```


#### 6.7.43 Viewshed

Identifies the viewshed for a point or set of points.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-dem               Input raster DEM file
-\-stations          Input viewing station vector file
-o, -\-output        Output raster file
-\-height            Viewing station height, in z units

*Command-line Interface*:
```
>>./whitebox_tools -r=Viewshed -v --wd="/path/to/data/" ^
--dem='dem.dep' --stations='stations.shp' -o=output.dep ^
--height=10.0 


```

*Python function name*: ```viewshed```


#### 6.7.44 WetnessIndex

Calculates the topographic wetness index, Ln(A / tan(slope)).

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-sca               Input raster specific contributing area (SCA) file
-\-slope             Input raster slope file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=WetnessIndex -v --wd="/path/to/data/" ^
--sca='flow_accum.dep' --slope='slope.dep' -o=output.dep 


```

*Python function name*: ```wetness_index```

### 6.8 Hydrological Analysis

#### 6.8.1 AverageFlowpathSlope

Measures the average slope gradient from each grid cell to all upslope divide cells.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=AverageFlowpathSlope -v ^
--wd="/path/to/data/" -i=DEM.dep -o=output.dep 


```

*Python function name*: ```average_flowpath_slope```


#### 6.8.2 AverageUpslopeFlowpathLength

Measures the average length of all upslope flowpaths draining each grid cell.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=AverageUpslopeFlowpathLength -v ^
--wd="/path/to/data/" -i=DEM.dep -o=output.dep 


```

*Python function name*: ```average_upslope_flowpath_length```


#### 6.8.3 Basins

Identifies drainage basins that drain to the DEM edge.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme

*Command-line Interface*:
```
>>./whitebox_tools -r=Basins -v --wd="/path/to/data/" ^
--d8_pntr='d8pntr.dep' -o='output.dep' 


```

*Python function name*: ```basins```


#### 6.8.4 BreachDepressions

Breaches all of the depressions in a DEM using Lindsay's (2016) algorithm. This should be preferred over depression filling in most cases.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-max_depth         Optional maximum breach depth (default is Inf)
-\-max_length        Optional maximum breach channel length (in grid cells; default is Inf)

*Command-line Interface*:
```
>>./whitebox_tools -r=BreachDepressions -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=output.dep 


```

*Python function name*: ```breach_depressions```


#### 6.8.5 BreachSingleCellPits

Removes single-cell pits from an input DEM by breaching.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=BreachSingleCellPits -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=output.dep 


```

*Python function name*: ```breach_single_cell_pits```


#### 6.8.6 D8FlowAccumulation

Calculates a D8 flow accumulation raster from an input DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-out_type          Output type; one of 'cells', 'specific contributing area' (default), and 
                     'catchment area' 
-\-log               Optional flag to request the output be log-transformed
-\-clip              Optional flag to request clipping the display max by 1%

*Command-line Interface*:
```
>>./whitebox_tools -r=D8FlowAccumulation -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=output.dep ^
--out_type='cells'
>>./whitebox_tools -r=D8FlowAccumulation -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=output.dep ^
--out_type='specific catchment area' --log --clip 


```

*Python function name*: ```d8_flow_accumulation```


#### 6.8.7 D8MassFlux

Performs a D8 mass flux calculation.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-dem               Input raster DEM file
-\-loading           Input loading raster file
-\-efficiency        Input efficiency raster file
-\-absorption        Input absorption raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=D8MassFlux -v --wd="/path/to/data/" ^
--dem=DEM.dep --loading=load.dep --efficiency=eff.dep ^
--absorption=abs.dep -o=output.dep 


```

*Python function name*: ```d8_mass_flux```


#### 6.8.8 D8Pointer

Calculates a D8 flow pointer raster from an input DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme

*Command-line Interface*:
```
>>./whitebox_tools -r=D8Pointer -v --wd="/path/to/data/" ^
--dem=DEM.dep -o=output.dep 


```

*Python function name*: ```d8_pointer```


#### 6.8.9 DInfFlowAccumulation

Calculates a D-infinity flow accumulation raster from an input DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-out_type          Output type; one of 'cells', 'sca' (default), and 'ca'
-\-threshold         Optional convergence threshold parameter, in grid cells; default is inifinity
-\-log               Optional flag to request the output be log-transformed
-\-clip              Optional flag to request clipping the display max by 1%

*Command-line Interface*:
```
>>./whitebox_tools -r=DInfFlowAccumulation -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=output.dep ^
--out_type=sca
>>./whitebox_tools -r=DInfFlowAccumulation -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=output.dep ^
--out_type=sca --threshold=10000 --log --clip 


```

*Python function name*: ```d_inf_flow_accumulation```


#### 6.8.10 DInfMassFlux

Performs a D-infinity mass flux calculation.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-dem               Input raster DEM file
-\-loading           Input loading raster file
-\-efficiency        Input efficiency raster file
-\-absorption        Input absorption raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=DInfMassFlux -v --wd="/path/to/data/" ^
--dem=DEM.dep --loading=load.dep --efficiency=eff.dep ^
--absorption=abs.dep -o=output.dep 


```

*Python function name*: ```d_inf_mass_flux```


#### 6.8.11 DInfPointer

Calculates a D-infinity flow pointer (flow direction) raster from an input DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=DInfPointer -v --wd="/path/to/data/" ^
--dem=DEM.dep 


```

*Python function name*: ```d_inf_pointer```


#### 6.8.12 DepthInSink

Measures the depth of sinks (depressions) in a DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-zero_background   Flag indicating whether the background value of zero should be used

*Command-line Interface*:
```
>>./whitebox_tools -r=DepthInSink -v --wd="/path/to/data/" ^
--dem=DEM.dep -o=output.dep --zero_background 


```

*Python function name*: ```depth_in_sink```


#### 6.8.13 DownslopeDistanceToStream

Measures distance to the nearest downslope stream cell.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-\-streams           Input raster streams file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=DownslopeDistanceToStream -v ^
--wd="/path/to/data/" --dem='dem.dep' --streams='streams.dep' ^
-o='output.dep' 


```

*Python function name*: ```downslope_distance_to_stream```


#### 6.8.14 DownslopeFlowpathLength

Calculates the downslope flowpath length from each cell to basin outlet.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input D8 pointer raster file
-\-watersheds        Optional input watershed raster file
-\-weights           Optional input weights raster file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme

*Command-line Interface*:
```
>>./whitebox_tools -r=DownslopeFlowpathLength -v ^
--wd="/path/to/data/" --d8_pntr=pointer.dep ^
-o=flowpath_len.dep
>>./whitebox_tools ^
-r=DownslopeFlowpathLength -v --wd="/path/to/data/" ^
--d8_pntr=pointer.flt --watersheds=basin.flt ^
--weights=weights.flt -o=flowpath_len.flt --esri_pntr 


```

*Python function name*: ```downslope_flowpath_length```


#### 6.8.15 ElevationAboveStream

Calculates the elevation of cells above the nearest downslope stream cell.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-\-streams           Input raster streams file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=ElevationAboveStream -v ^
--wd="/path/to/data/" --dem='dem.dep' --streams='streams.dep' ^
-o='output.dep' 


```

*Python function name*: ```elevation_above_stream```


#### 6.8.16 FD8FlowAccumulation

Calculates an FD8 flow accumulation raster from an input DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-out_type          Output type; one of 'cells', 'specific contributing area' (default), and 
                     'catchment area' 
-\-exponent          Optional exponent parameter; default is 1.1
-\-threshold         Optional convergence threshold parameter, in grid cells; default is inifinity
-\-log               Optional flag to request the output be log-transformed
-\-clip              Optional flag to request clipping the display max by 1%

*Command-line Interface*:
```
>>./whitebox_tools -r=FD8FlowAccumulation -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=output.dep ^
--out_type='cells'
>>./whitebox_tools -r=FD8FlowAccumulation -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=output.dep ^
--out_type='catchment area' --exponent=1.5 --threshold=10000 ^
--log --clip 


```

*Python function name*: ```fd8_flow_accumulation```


#### 6.8.17 FD8Pointer

Calculates an FD8 flow pointer raster from an input DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=FD8Pointer -v --wd="/path/to/data/" ^
--dem=DEM.dep -o=output.dep 


```

*Python function name*: ```fd8_pointer```


#### 6.8.18 FillDepressions

Fills all of the depressions in a DEM. Depression breaching should be preferred in most cases.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-fix_flats         Optional flag indicating whether flat areas should have a small gradient applied

*Command-line Interface*:
```
>>./whitebox_tools -r=FillDepressions -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=output.dep ^
--fix_flats 


```

*Python function name*: ```fill_depressions```


#### 6.8.19 FillSingleCellPits

Raises pit cells to the elevation of their lowest neighbour.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=FillSingleCellPits -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=NewRaster.dep 


```

*Python function name*: ```fill_single_cell_pits```


#### 6.8.20 FindNoFlowCells

Finds grid cells with no downslope neighbours.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=FindNoFlowCells -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=NewRaster.dep 


```

*Python function name*: ```find_no_flow_cells```


#### 6.8.21 FindParallelFlow

Finds areas of parallel flow in D8 flow direction rasters.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input D8 pointer raster file
-\-streams           Input raster streams file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=FindParallelFlow -v ^
--wd="/path/to/data/" --d8_pntr=pointer.dep ^
-o=out.dep
>>./whitebox_tools -r=FindParallelFlow -v ^
--wd="/path/to/data/" --d8_pntr=pointer.dep -o=out.dep ^
--streams='streams.dep' 


```

*Python function name*: ```find_parallel_flow```


#### 6.8.22 FloodOrder

Assigns each DEM grid cell its order in the sequence of inundations that are encountered during a search starting from the edges, moving inward at increasing elevations.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=FloodOrder -v --wd="/path/to/data/" ^
--dem=DEM.dep -o=output.dep 


```

*Python function name*: ```flood_order```


#### 6.8.23 FlowAccumulationFullWorkflow

Resolves all of the depressions in a DEM, outputting a breached DEM, an aspect-aligned non-divergent flow pointer, a flow accumulation raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-\-out_dem           Output raster DEM file
-\-out_pntr          Output raster flow pointer file
-\-out_accum         Output raster flow accumulation file
-\-out_type          Output type; one of 'cells', 'sca' (default), and 'ca'
-\-log               Optional flag to request the output be log-transformed
-\-clip              Optional flag to request clipping the display max by 1%
-\-esri_pntr         D8 pointer uses the ESRI style scheme

*Command-line Interface*:
```
>>./whitebox_tools -r=FlowAccumulationFullWorkflow -v ^
--wd="/path/to/data/" --dem='DEM.dep' ^
--out_dem='DEM_filled.dep' --out_pntr='pointer.dep' ^
--out_accum='accum.dep' --out_type=sca --log --clip 


```

*Python function name*: ```flow_accumulation_full_workflow```


#### 6.8.24 FlowLengthDiff

Calculates the local maximum absolute difference in downslope flowpath length, useful in mapping drainage divides and ridges.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input D8 pointer raster file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme

*Command-line Interface*:
```
>>./whitebox_tools -r=FlowLengthDiff -v --wd="/path/to/data/" ^
--d8_pntr=pointer.dep -o=output.dep 


```

*Python function name*: ```flow_length_diff```


#### 6.8.25 Hillslopes

Identifies the individual hillslopes draining to each link in a stream network.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-\-streams           Input raster streams file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme

*Command-line Interface*:
```
>>./whitebox_tools -r=Hillslopes -v --wd="/path/to/data/" ^
--d8_pntr='d8pntr.dep' --streams='streams.dep' ^
-o='output.dep' 


```

*Python function name*: ```hillslopes```


#### 6.8.26 Isobasins

Divides a landscape into nearly equal sized drainage basins (i.e. watersheds).

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-size              Target basin size, in grid cells

*Command-line Interface*:
```
>>./whitebox_tools -r=Isobasins -v --wd="/path/to/data/" ^
--dem=DEM.dep -o=output.dep --size=1000 


```

*Python function name*: ```isobasins```


#### 6.8.27 JensonSnapPourPoints

Moves outlet points used to specify points of interest in a watershedding operation to the nearest stream cell.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-pour_pts          Input raster pour points (outlet) file
-\-streams           Input raster streams file
-o, -\-output        Output raster file
-\-snap_dist         Maximum snap distance in map units

*Command-line Interface*:
```
>>./whitebox_tools -r=JensonSnapPourPoints -v ^
--wd="/path/to/data/" --pour_pts='pour_pts.dep' ^
--streams='streams.dep' -o='output.dep' --snap_dist=15.0 


```

*Python function name*: ```jenson_snap_pour_points```


#### 6.8.28 MaxUpslopeFlowpathLength

Measures the maximum length of all upslope flowpaths draining each grid cell.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=MaxUpslopeFlowpathLength -v ^
--wd="/path/to/data/" -i=DEM.dep ^
-o=output.dep
>>./whitebox_tools -r=MaxUpslopeFlowpathLength -v ^
--wd="/path/to/data/" --dem=DEM.dep -o=output.dep --log ^
--clip 


```

*Python function name*: ```max_upslope_flowpath_length```


#### 6.8.29 NumInflowingNeighbours

Computes the number of inflowing neighbours to each cell in an input DEM based on the D8 algorithm.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=NumInflowingNeighbours -v ^
--wd="/path/to/data/" -i=DEM.dep -o=output.dep 


```

*Python function name*: ```num_inflowing_neighbours```


#### 6.8.30 Rho8Pointer

Calculates a stochastic Rho8 flow pointer raster from an input DEM.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme

*Command-line Interface*:
```
>>./whitebox_tools -r=Rho8Pointer -v --wd="/path/to/data/" ^
--dem=DEM.dep -o=output.dep 


```

*Python function name*: ```rho8_pointer```


#### 6.8.31 Sink

Identifies the depressions in a DEM, giving each feature a unique identifier.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-zero_background   Flag indicating whether a background value of zero should be used

*Command-line Interface*:
```
>>./whitebox_tools -r=Sink -v --wd="/path/to/data/" ^
--dem=DEM.dep -o=output.dep --zero_background 


```

*Python function name*: ```sink```


#### 6.8.32 SnapPourPoints

Moves outlet points used to specify points of interest in a watershedding operation to the cell with the highest flow accumulation in its neighbourhood.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-pour_pts          Input raster pour points (outlet) file
-\-flow_accum        Input raster D8 flow accumulation file
-o, -\-output        Output raster file
-\-snap_dist         Maximum snap distance in map units

*Command-line Interface*:
```
>>./whitebox_tools -r=SnapPourPoints -v --wd="/path/to/data/" ^
--pour_pts='pour_pts.dep' --flow_accum='d8accum.dep' ^
-o='output.dep' --snap_dist=15.0 


```

*Python function name*: ```snap_pour_points```


#### 6.8.33 StrahlerOrderBasins

Identifies Strahler-order basins from an input stream network.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-\-streams           Input raster streams file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme

*Command-line Interface*:
```
>>./whitebox_tools -r=StrahlerOrderBasins -v ^
--wd="/path/to/data/" --d8_pntr='d8pntr.dep' ^
--streams='streams.dep' -o='output.dep' 


```

*Python function name*: ```strahler_order_basins```


#### 6.8.34 Subbasins

Identifies the catchments, or sub-basin, draining to each link in a stream network.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input D8 pointer raster file
-\-streams           Input raster streams file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme

*Command-line Interface*:
```
>>./whitebox_tools -r=Subbasins -v --wd="/path/to/data/" ^
--d8_pntr='d8pntr.dep' --streams='streams.dep' ^
-o='output.dep' 


```

*Python function name*: ```subbasins```


#### 6.8.35 TraceDownslopeFlowpaths

Traces downslope flowpaths from one or more target sites (i.e. seed points).

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-seed_pts          Input raster seed points file
-\-d8_pntr           Input D8 pointer raster file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme
-\-zero_background   Flag indicating whether a background value of zero should be used

*Command-line Interface*:
```
>>./whitebox_tools -r=TraceDownslopeFlowpaths -v ^
--wd="/path/to/data/" --seed_pts=seeds.dep ^
--flow_dir=flow_directions.dep --output=flow_paths.dep 


```

*Python function name*: ```trace_downslope_flowpaths```


#### 6.8.36 Watershed

Identifies the watershed, or drainage basin, draining to a set of target cells.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input D8 pointer raster file
-\-pour_pts          Input vector pour points (outlet) file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme

*Command-line Interface*:
```
>>./whitebox_tools -r=Watershed -v --wd="/path/to/data/" ^
--d8_pntr='d8pntr.dep' --pour_pts='pour_pts.shp' ^
-o='output.dep' 


```

*Python function name*: ```watershed```

### 6.9 Image Processing Tools

#### 6.9.1 Closing

A closing is a mathematical morphology operating involving an erosion (min filter) of a dilation (max filter) set.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction

*Command-line Interface*:
```
>>./whitebox_tools -r=Closing -v --wd="/path/to/data/" ^
-i=image.dep -o=output.dep --filter=25 


```

*Python function name*: ```closing```


#### 6.9.2 CreateColourComposite

Creates a colour-composite image from three bands of multispectral imagery.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-red               Input red band image file
-\-green             Input green band image file
-\-blue              Input blue band image file
-\-opacity           Input opacity band image file (optional)
-o, -\-output        Output colour composite file
-\-enhance           Optional flag indicating whether a balance contrast enhancement is performed

*Command-line Interface*:
```
>>./whitebox_tools -r=CreateColourComposite -v ^
--wd="/path/to/data/" --red=band3.dep --green=band2.dep ^
--blue=band1.dep -o=output.dep
>>./whitebox_tools ^
-r=CreateColourComposite -v --wd="/path/to/data/" ^
--red=band3.dep --green=band2.dep --blue=band1.dep ^
--opacity=a.dep -o=output.dep 


```

*Python function name*: ```create_colour_composite```


#### 6.9.3 FlipImage

Reflects an image in the vertical or horizontal axis.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-direction         Direction of reflection; options include 'v' (vertical), 'h' (horizontal), and 
                     'b' (both) 

*Command-line Interface*:
```
>>./whitebox_tools -r=FlipImage -v --wd="/path/to/data/" ^
--input=in.dep -o=out.dep --direction=h 


```

*Python function name*: ```flip_image```


#### 6.9.4 IntegralImage

Transforms an input image (summed area table) into its integral image equivalent.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=IntegralImage -v --wd="/path/to/data/" ^
-i=image.dep -o=output.dep 


```

*Python function name*: ```integral_image```


#### 6.9.5 KMeansClustering

Performs a k-means clustering operation on a multi-spectral dataset.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input raster files
-o, -\-output        Output raster file
-\-out_html          Output HTML report file
-\-classes           Number of classes
-\-max_iterations    Maximum number of iterations
-\-class_change      Minimum percent of cells changed between iterations before completion
-\-initialize        How to initialize cluster centres?
-\-min_class_size    Minimum class size, in pixels

*Command-line Interface*:
```
>>./whitebox_tools -r=KMeansClustering -v ^
--wd='/path/to/data/' -i='image1.tif;image2.tif;image3.tif' ^
-o=output.tif --out_html=report.html --classes=15 ^
--max_iterations=25 --class_change=1.5 --initialize='random' ^
--min_class_size=500 


```

*Python function name*: ```k_means_clustering```


#### 6.9.6 LineThinning

Performs line thinning a on Boolean raster image; intended to be used with the RemoveSpurs tool.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=LineThinning -v --wd="/path/to/data/" ^
--input=DEM.dep -o=output.dep 


```

*Python function name*: ```line_thinning```


#### 6.9.7 ModifiedKMeansClustering

Performs a modified k-means clustering operation on a multi-spectral dataset.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input raster files
-o, -\-output        Output raster file
-\-out_html          Output HTML report file
-\-start_clusters    Initial number of clusters
-\-merger_dist       Cluster merger distance
-\-max_iterations    Maximum number of iterations
-\-class_change      Minimum percent of cells changed between iterations before completion

*Command-line Interface*:
```
>>./whitebox_tools -r=ModifiedKMeansClustering -v ^
--wd='/path/to/data/' -i='image1.tif;image2.tif;image3.tif' ^
-o=output.tif --out_html=report.html --start_clusters=100 ^
--merger_dist=30.0 --max_iterations=25 --class_change=1.5 


```

*Python function name*: ```modified_k_means_clustering```


#### 6.9.8 Mosaic

Mosaics two or more images together.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input raster files
-o, -\-output        Output raster file
-\-method            Resampling method

*Command-line Interface*:
```
>>./whitebox_tools -r=Mosaic -v --wd='/path/to/data/' ^
-i='image1.dep;image2.dep;image3.dep' -o=dest.dep ^
--method='cc 


```

*Python function name*: ```mosaic```


#### 6.9.9 NormalizedDifferenceVegetationIndex

Calculates the normalized difference vegetation index (NDVI) from near-infrared and red imagery.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-nir               Input near-infrared band image
-\-red               Input red band image
-o, -\-output        Output raster file
-\-clip              Optional amount to clip the distribution tails by, in percent
-\-osavi             Optional flag indicating whether the optimized soil-adjusted veg index (OSAVI) 
                     should be used 

*Command-line Interface*:
```
>>./whitebox_tools -r=NormalizedDifferenceVegetationIndex -v ^
--wd="/path/to/data/" --nir=band4.dep --red=band3.dep ^
-o=output.dep
>>./whitebox_tools ^
-r=NormalizedDifferenceVegetationIndex -v --wd="/path/to/data/" ^
--nir=band4.dep --red=band3.dep -o=output.dep --clip=1.0 ^
--osavi 


```

*Python function name*: ```normalized_difference_vegetation_index```


#### 6.9.10 Opening

An opening is a mathematical morphology operating involving a dilation (max filter) of an erosion (min filter) set.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction

*Command-line Interface*:
```
>>./whitebox_tools -r=Opening -v --wd="/path/to/data/" ^
-i=image.dep -o=output.dep --filter=25 


```

*Python function name*: ```opening```


#### 6.9.11 RemoveSpurs

Removes the spurs (pruning operation) from a Boolean line image.; intended to be used on the output of the LineThinning tool.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-iterations        Maximum number of iterations

*Command-line Interface*:
```
>>./whitebox_tools -r=RemoveSpurs -v --wd="/path/to/data/" ^
--input=DEM.dep -o=output.dep --iterations=10 


```

*Python function name*: ```remove_spurs```


#### 6.9.12 Resample

Resamples one or more input images into a destination image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input raster files
-\-destination       Destination raster file
-\-method            Resampling method

*Command-line Interface*:
```
>>./whitebox_tools -r=Resample -v --wd='/path/to/data/' ^
-i='image1.dep;image2.dep;image3.dep' --destination=dest.dep ^
--method='cc 


```

*Python function name*: ```resample```


#### 6.9.13 RgbToIhs

Converts red, green, and blue (RGB) images into intensity, hue, and saturation (IHS) images.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-red               Input red band image file. Optionally specified if colour-composite not 
                     specified 
-\-green             Input green band image file. Optionally specified if colour-composite not 
                     specified 
-\-blue              Input blue band image file. Optionally specified if colour-composite not 
                     specified 
-\-composite         Input colour-composite image file. Only used if individual bands are not 
                     specified 
-\-intensity         Output intensity raster file
-\-hue               Output hue raster file
-\-saturation        Output saturation raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=RgbToIhs -v --wd="/path/to/data/" ^
--red=band3.dep --green=band2.dep --blue=band1.dep ^
--intensity=intensity.dep --hue=hue.dep ^
--saturation=saturation.dep
>>./whitebox_tools -r=RgbToIhs -v ^
--wd="/path/to/data/" --composite=image.dep ^
--intensity=intensity.dep --hue=hue.dep ^
--saturation=saturation.dep 


```

*Python function name*: ```rgb_to_ihs```


#### 6.9.14 SplitColourComposite

This tool splits an RGB colour composite image into seperate multispectral images.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input colour composite image file
-o, -\-output        Output raster file (suffixes of '_r', '_g', and '_b' will be appended)

*Command-line Interface*:
```
>>./whitebox_tools -r=SplitColourComposite -v ^
--wd="/path/to/data/" -i=input.dep -o=output.dep 


```

*Python function name*: ```split_colour_composite```


#### 6.9.15 ThickenRasterLine

Thickens single-cell wide lines within a raster image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=ThickenRasterLine -v ^
--wd="/path/to/data/" --input=DEM.dep -o=output.dep 


```

*Python function name*: ```thicken_raster_line```


#### 6.9.16 TophatTransform

Performs either a white or black top-hat transform on an input image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction
-\-variant           Optional variant value. Options include 'white' and 'black'

*Command-line Interface*:
```
>>./whitebox_tools -r=TophatTransform -v ^
--wd="/path/to/data/" -i=image.dep -o=output.dep --filter=25 


```

*Python function name*: ```tophat_transform```


#### 6.9.17 WriteFunctionMemoryInsertion

Performs a write function memory insertion for single-band multi-date change detection.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-i1, -\-input1     Input raster file associated with the first date
-\-i2, -\-input2     Input raster file associated with the second date
-\-i3, -\-input3     Optional input raster file associated with the third date
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=WriteFunctionMemoryInsertion -v ^
--wd="/path/to/data/" -i1=input1.dep -i2=input2.dep ^
-o=output.dep 


```

*Python function name*: ```write_function_memory_insertion```

### 6.10 Image Processing Tools => Filters

#### 6.10.1 AdaptiveFilter

Performs an adaptive filter on an image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction
-\-threshold         Difference from mean threshold, in standard deviations

*Command-line Interface*:
```
>>./whitebox_tools -r=AdaptiveFilter -v --wd="/path/to/data/" ^
-i=DEM.dep -o=output.dep --filter=25 --threshold = 2.0 


```

*Python function name*: ```adaptive_filter```


#### 6.10.2 BilateralFilter

A bilateral filter is an edge-preserving smoothing filter introduced by Tomasi and Manduchi (1998).

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-sigma_dist        Standard deviation in distance in pixels
-\-sigma_int         Standard deviation in intensity in pixels

*Command-line Interface*:
```
>>./whitebox_tools -r=BilateralFilter -v ^
--wd="/path/to/data/" -i=image.dep -o=output.dep ^
--sigma_dist=2.5 --sigma_int=4.0 


```

*Python function name*: ```bilateral_filter```


#### 6.10.3 ConservativeSmoothingFilter

Performs a conservative-smoothing filter on an image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction

*Command-line Interface*:
```
>>./whitebox_tools -r=ConservativeSmoothingFilter -v ^
--wd="/path/to/data/" -i=image.dep -o=output.dep --filter=25 


```

*Python function name*: ```conservative_smoothing_filter```


#### 6.10.4 DiffOfGaussianFilter

Performs a Difference of Gaussian (DoG) filter on an image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-sigma1            Standard deviation distance in pixels
-\-sigma2            Standard deviation distance in pixels

*Command-line Interface*:
```
>>./whitebox_tools -r=DiffOfGaussianFilter -v ^
--wd="/path/to/data/" -i=image.dep -o=output.dep --sigma1=2.0 ^
--sigma2=4.0 


```

*Python function name*: ```diff_of_gaussian_filter```


#### 6.10.5 DiversityFilter

Assigns each cell in the output grid the number of different values in a moving window centred on each grid cell in the input raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction

*Command-line Interface*:
```
>>./whitebox_tools -r=DiversityFilter -v ^
--wd="/path/to/data/" -i=image.dep -o=output.dep --filter=25 


```

*Python function name*: ```diversity_filter```


#### 6.10.6 EmbossFilter

Performs an emboss filter on an image, similar to a hillshade operation.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-direction         Direction of reflection; options include 'n', 's', 'e', 'w', 'ne', 'se', 'nw', 
                     'sw' 
-\-clip              Optional amount to clip the distribution tails by, in percent

*Command-line Interface*:
```
>>./whitebox_tools -r=EmbossFilter -v --wd="/path/to/data/" ^
-i=image.dep -o=output.dep --direction='s' --clip=1.0 


```

*Python function name*: ```emboss_filter```


#### 6.10.7 GaussianFilter

Performs a Gaussian filter on an image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-sigma             Standard deviation distance in pixels

*Command-line Interface*:
```
>>./whitebox_tools -r=GaussianFilter -v --wd="/path/to/data/" ^
-i=image.dep -o=output.dep --sigma=2.0 


```

*Python function name*: ```gaussian_filter```


#### 6.10.8 HighPassFilter

Performs a high-pass filter on an input image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction

*Command-line Interface*:
```
>>./whitebox_tools -r=HighPassFilter -v --wd="/path/to/data/" ^
-i=image.dep -o=output.dep --filter=25 


```

*Python function name*: ```high_pass_filter```


#### 6.10.9 KNearestMeanFilter

A k-nearest mean filter is a type of edge-preserving smoothing filter.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction
-k                   k-value in pixels; this is the number of nearest-valued neighbours to use

*Command-line Interface*:
```
>>./whitebox_tools -r=KNearestMeanFilter -v ^
--wd="/path/to/data/" -i=image.dep -o=output.dep --filter=9 ^
-k=5
>>./whitebox_tools -r=KNearestMeanFilter -v ^
--wd="/path/to/data/" -i=image.dep -o=output.dep --filtery=7 ^
--filtery=9 -k=5 


```

*Python function name*: ```k_nearest_mean_filter```


#### 6.10.10 LaplacianFilter

Performs a Laplacian filter on an image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-variant           Optional variant value. Options include 3x3(1), 3x3(2), 3x3(3), 3x3(4), 5x5(1), 
                     and 5x5(2) (default is 3x3(1)) 
-\-clip              Optional amount to clip the distribution tails by, in percent

*Command-line Interface*:
```
>>./whitebox_tools -r=LaplacianFilter -v ^
--wd="/path/to/data/" -i=image.dep -o=output.dep ^
--variant='3x3(1)' --clip=1.0 


```

*Python function name*: ```laplacian_filter```


#### 6.10.11 LaplacianOfGaussianFilter

Performs a Laplacian-of-Gaussian (LoG) filter on an image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-sigma             Standard deviation in pixels

*Command-line Interface*:
```
>>./whitebox_tools -r=LaplacianOfGaussianFilter -v ^
--wd="/path/to/data/" -i=image.dep -o=output.dep --sigma=2.0 


```

*Python function name*: ```laplacian_of_gaussian_filter```


#### 6.10.12 LeeFilter

Performs a Lee (Sigma) smoothing filter on an image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction
-\-sigma             Sigma value should be related to the standarad deviation of the distribution of 
                     image speckle noise 
-m                   M-threshold value the minimum allowable number of pixels within the intensity 
                     range 

*Command-line Interface*:
```
>>./whitebox_tools -r=LeeFilter -v --wd="/path/to/data/" ^
-i=image.dep -o=output.dep --filter=9 --sigma=10.0 ^
-m=5
>>./whitebox_tools -r=LeeFilter -v --wd="/path/to/data/" ^
-i=image.dep -o=output.dep --filtery=7 --filtery=9 ^
--sigma=10.0 -m=5 


```

*Python function name*: ```lee_filter```


#### 6.10.13 LineDetectionFilter

Performs a line-detection filter on an image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-variant           Optional variant value. Options include 'v' (vertical), 'h' (horizontal), '45', 
                     and '135' (default is 'v') 
-\-absvals           Optional flag indicating whether outputs should be absolute values
-\-clip              Optional amount to clip the distribution tails by, in percent

*Command-line Interface*:
```
>>./whitebox_tools -r=LineDetectionFilter -v ^
--wd="/path/to/data/" -i=image.dep -o=output.dep --variant=h ^
--clip=1.0 


```

*Python function name*: ```line_detection_filter```


#### 6.10.14 MajorityFilter

Assigns each cell in the output grid the most frequently occurring value (mode) in a moving window centred on each grid cell in the input raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction

*Command-line Interface*:
```
>>./whitebox_tools -r=MajorityFilter -v --wd="/path/to/data/" ^
-i=image.dep -o=output.dep --filter=25 


```

*Python function name*: ```majority_filter```


#### 6.10.15 MaximumFilter

Assigns each cell in the output grid the maximum value in a moving window centred on each grid cell in the input raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction

*Command-line Interface*:
```
>>./whitebox_tools -r=MaximumFilter -v --wd="/path/to/data/" ^
-i=image.dep -o=output.dep --filter=25 


```

*Python function name*: ```maximum_filter```


#### 6.10.16 MeanFilter

Performs a mean filter (low-pass filter) on an input image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction

*Command-line Interface*:
```
>>./whitebox_tools -r=MeanFilter -v --wd="/path/to/data/" ^
-i=image.dep -o=output.dep --filterx=25 --filtery=25 


```

*Python function name*: ```mean_filter```


#### 6.10.17 MedianFilter

Performs a median filter on an input image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction
-\-sig_digits        Number of significant digits

*Command-line Interface*:
```
>>./whitebox_tools -r=MedianFilter -v --wd="/path/to/data/" ^
-i=input.dep -o=output.dep --filter=25 


```

*Python function name*: ```median_filter```


#### 6.10.18 MinimumFilter

Assigns each cell in the output grid the minimum value in a moving window centred on each grid cell in the input raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction

*Command-line Interface*:
```
>>./whitebox_tools -r=MinimumFilter -v --wd="/path/to/data/" ^
-i=image.dep -o=output.dep --filter=25 


```

*Python function name*: ```minimum_filter```


#### 6.10.19 OlympicFilter

Performs an olympic smoothing filter on an image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction

*Command-line Interface*:
```
>>./whitebox_tools -r=OlympicFilter -v --wd="/path/to/data/" ^
-i=image.dep -o=output.dep --filter=25 


```

*Python function name*: ```olympic_filter```


#### 6.10.20 PercentileFilter

Performs a percentile filter on an input image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction
-\-sig_digits        Number of significant digits

*Command-line Interface*:
```
>>./whitebox_tools -r=PercentileFilter -v ^
--wd="/path/to/data/" -i=input.dep -o=output.dep --filter=25 


```

*Python function name*: ```percentile_filter```


#### 6.10.21 PrewittFilter

Performs a Prewitt edge-detection filter on an image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-clip              Optional amount to clip the distribution tails by, in percent

*Command-line Interface*:
```
>>./whitebox_tools -r=PrewittFilter -v --wd="/path/to/data/" ^
-i=image.dep -o=output.dep --clip=1.0 


```

*Python function name*: ```prewitt_filter```


#### 6.10.22 RangeFilter

Assigns each cell in the output grid the range of values in a moving window centred on each grid cell in the input raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction

*Command-line Interface*:
```
>>./whitebox_tools -r=RangeFilter -v --wd="/path/to/data/" ^
-i=image.dep -o=output.dep --filter=25 


```

*Python function name*: ```range_filter```


#### 6.10.23 RobertsCrossFilter

Performs a Robert's cross edge-detection filter on an image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-clip              Optional amount to clip the distribution tails by, in percent

*Command-line Interface*:
```
>>./whitebox_tools -r=RobertsCrossFilter -v ^
--wd="/path/to/data/" -i=image.dep -o=output.dep --clip=1.0 


```

*Python function name*: ```roberts_cross_filter```


#### 6.10.24 ScharrFilter

Performs a Scharr edge-detection filter on an image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-clip              Optional amount to clip the distribution tails by, in percent

*Command-line Interface*:
```
>>./whitebox_tools -r=ScharrFilter -v --wd="/path/to/data/" ^
-i=image.dep -o=output.dep --clip=1.0 


```

*Python function name*: ```scharr_filter```


#### 6.10.25 SobelFilter

Performs a Sobel edge-detection filter on an image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-variant           Optional variant value. Options include 3x3 and 5x5 (default is 3x3)
-\-clip              Optional amount to clip the distribution tails by, in percent (default is 0.0)

*Command-line Interface*:
```
>>./whitebox_tools -r=SobelFilter -v --wd="/path/to/data/" ^
-i=image.dep -o=output.dep --variant=5x5 --clip=1.0 


```

*Python function name*: ```sobel_filter```


#### 6.10.26 StandardDeviationFilter

Assigns each cell in the output grid the standard deviation of values in a moving window centred on each grid cell in the input raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction

*Command-line Interface*:
```
>>./whitebox_tools -r=StandardDeviationFilter -v ^
--wd="/path/to/data/" -i=image.dep -o=output.dep --filter=25 


```

*Python function name*: ```standard_deviation_filter```


#### 6.10.27 TotalFilter

Performs a total filter on an input image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-filterx           Size of the filter kernel in the x-direction
-\-filtery           Size of the filter kernel in the y-direction

*Command-line Interface*:
```
>>./whitebox_tools -r=TotalFilter -v --wd="/path/to/data/" ^
-i=image.dep -o=output.dep --filter=25 


```

*Python function name*: ```total_filter```

### 6.11 Image Processing Tools => Image Enhancement

#### 6.11.1 BalanceContrastEnhancement

Performs a balance contrast enhancement on a colour-composite image of multispectral data.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input colour composite image file
-o, -\-output        Output raster file
-\-band_mean         Band mean value

*Command-line Interface*:
```
>>./whitebox_tools -r=BalanceContrastEnhancement -v ^
--wd="/path/to/data/" --input=image.dep -o=output.dep ^
--band_mean=120 


```

*Python function name*: ```balance_contrast_enhancement```


#### 6.11.2 DirectDecorrelationStretch

Performs a direct decorrelation stretch enhancement on a colour-composite image of multispectral data.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input colour composite image file
-o, -\-output        Output raster file
-k                   Achromatic factor (k) ranges between 0 (no effect) and 1 (full saturation 
                     stretch), although typical values range from 0.3 to 0.7 
-\-clip              Optional percent to clip the upper tail by during the stretch

*Command-line Interface*:
```
>>./whitebox_tools -r=DirectDecorrelationStretch -v ^
--wd="/path/to/data/" --input=image.dep -o=output.dep -k=0.4 


```

*Python function name*: ```direct_decorrelation_stretch```


#### 6.11.3 GammaCorrection

Performs a sigmoidal contrast stretch on input images.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-gamma             Gamma value

*Command-line Interface*:
```
>>./whitebox_tools -r=GammaCorrection -v ^
--wd="/path/to/data/" -i=input.dep -o=output.dep --gamma=0.5 


```

*Python function name*: ```gamma_correction```


#### 6.11.4 HistogramEqualization

Performs a histogram equalization contrast enhancment on an image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-num_tones         Number of tones in the output image

*Command-line Interface*:
```
>>./whitebox_tools -r=HistogramEqualization -v ^
--wd="/path/to/data/" -i=input.dep -o=output.dep ^
--num_tones=1024 


```

*Python function name*: ```histogram_equalization```


#### 6.11.5 HistogramMatching

Alters the statistical distribution of a raster image matching it to a specified PDF.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-\-histo_file        Input reference probability distribution function (pdf) text file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=HistogramMatching -v ^
--wd="/path/to/data/" -i=input1.dep --histo_file=histo.txt ^
-o=output.dep 


```

*Python function name*: ```histogram_matching```


#### 6.11.6 HistogramMatchingTwoImages

This tool alters the cumulative distribution function of a raster image to that of another image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-i1, -\-input1     Input raster file to modify
-\-i2, -\-input2     Input reference raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=HistogramMatchingTwoImages -v ^
--wd="/path/to/data/" --i1=input1.dep --i2=input2.dep ^
-o=output.dep 


```

*Python function name*: ```histogram_matching_two_images```


#### 6.11.7 MinMaxContrastStretch

Performs a min-max contrast stretch on an input greytone image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-min_val           Lower tail clip value
-\-max_val           Upper tail clip value
-\-num_tones         Number of tones in the output image

*Command-line Interface*:
```
>>./whitebox_tools -r=MinMaxContrastStretch -v ^
--wd="/path/to/data/" -i=input.dep -o=output.dep ^
--min_val=45.0 --max_val=200.0 --num_tones=1024 


```

*Python function name*: ```min_max_contrast_stretch```


#### 6.11.8 PanchromaticSharpening

Increases the spatial resolution of image data by combining multispectral bands with panchromatic data.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-red               Input red band image file. Optionally specified if colour-composite not 
                     specified 
-\-green             Input green band image file. Optionally specified if colour-composite not 
                     specified 
-\-blue              Input blue band image file. Optionally specified if colour-composite not 
                     specified 
-\-composite         Input colour-composite image file. Only used if individual bands are not 
                     specified 
-\-pan               Input panchromatic band file
-o, -\-output        Output colour composite file
-\-method            Options include 'brovey' (default) and 'ihs'

*Command-line Interface*:
```
>>./whitebox_tools -r=PanchromaticSharpening -v ^
--wd="/path/to/data/" --red=red.dep --green=green.dep ^
--blue=blue.dep --pan=pan.dep --output=pan_sharp.dep ^
--method='brovey'
>>./whitebox_tools -r=PanchromaticSharpening ^
-v --wd="/path/to/data/" --composite=image.dep --pan=pan.dep ^
--output=pan_sharp.dep --method='ihs' 


```

*Python function name*: ```panchromatic_sharpening```


#### 6.11.9 PercentageContrastStretch

Performs a percentage linear contrast stretch on input images.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-clip              Optional amount to clip the distribution tails by, in percent
-\-tail              Specified which tails to clip; options include 'upper', 'lower', and 'both' 
                     (default is 'both') 
-\-num_tones         Number of tones in the output image

*Command-line Interface*:
```
>>./whitebox_tools -r=PercentageContrastStretch -v ^
--wd="/path/to/data/" -i=input.dep -o=output.dep --clip=2.0 ^
--tail='both' --num_tones=1024 


```

*Python function name*: ```percentage_contrast_stretch```


#### 6.11.10 SigmoidalContrastStretch

Performs a sigmoidal contrast stretch on input images.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-cutoff            Cutoff value between 0.0 and 0.95
-\-gain              Gain value
-\-num_tones         Number of tones in the output image

*Command-line Interface*:
```
>>./whitebox_tools -r=SigmoidalContrastStretch -v ^
--wd="/path/to/data/" -i=input.dep -o=output.dep --cutoff=0.1 ^
--gain=2.0 --num_tones=1024 


```

*Python function name*: ```sigmoidal_contrast_stretch```


#### 6.11.11 StandardDeviationContrastStretch

Performs a standard-deviation contrast stretch on input images.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-clip, -\-stdev    Standard deviation clip value
-\-num_tones         Number of tones in the output image

*Command-line Interface*:
```
>>./whitebox_tools -r=StandardDeviationContrastStretch -v ^
--wd="/path/to/data/" -i=input.dep -o=output.dep --stdev=2.0 ^
--num_tones=1024 


```

*Python function name*: ```standard_deviation_contrast_stretch```

### 6.12 LiDAR Tools

#### 6.12.1 BlockMaximum

Creates a block-maximum raster from an input LAS file.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file
-o, -\-output        Output file
-\-resolution        Output raster's grid resolution

*Command-line Interface*:
```
>>./whitebox_tools -r=BlockMaximum -v --wd="/path/to/data/" ^
-i=file.las -o=outfile.dep --resolution=2.0"
./whitebox_tools ^
-r=BlockMaximum -v --wd="/path/to/data/" -i=file.las ^
-o=outfile.dep --resolution=5.0 --palette=light_quant.plt 


```

*Python function name*: ```block_maximum```


#### 6.12.2 BlockMinimum

Creates a block-minimum raster from an input LAS file.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file
-o, -\-output        Output file
-\-resolution        Output raster's grid resolution

*Command-line Interface*:
```
>>./whitebox_tools -r=BlockMinimum -v --wd="/path/to/data/" ^
-i=file.las -o=outfile.dep --resolution=2.0"
./whitebox_tools ^
-r=BlockMinimum -v --wd="/path/to/data/" -i=file.las ^
-o=outfile.dep --resolution=5.0 --palette=light_quant.plt 


```

*Python function name*: ```block_minimum```


#### 6.12.3 FilterLidarScanAngles

Removes points in a LAS file with scan angles greater than a threshold.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file
-o, -\-output        Output LiDAR file
-\-threshold         Scan angle threshold

*Command-line Interface*:
```
>>./whitebox_tools -r=FilterLidarScanAngles -v ^
--wd="/path/to/data/" -i="input.las" -o="output.las" ^
--threshold=10.0 


```

*Python function name*: ```filter_lidar_scan_angles```


#### 6.12.4 FindFlightlineEdgePoints

Identifies points along a flightline's edge in a LAS file.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file
-o, -\-output        Output file

*Command-line Interface*:
```
>>./whitebox_tools -r=FindFlightlineEdgePoints -v ^
--wd="/path/to/data/" -i="input.las" -o="output.las" 


```

*Python function name*: ```find_flightline_edge_points```


#### 6.12.5 FlightlineOverlap

Reads a LiDAR (LAS) point file and outputs a raster containing the number of overlapping flight lines in each grid cell.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file
-o, -\-output        Output file
-\-resolution        Output raster's grid resolution

*Command-line Interface*:
```
>>./whitebox_tools -r=FlightlineOverlap -v ^
--wd="/path/to/data/" -i=file.las -o=outfile.dep ^
--resolution=2.0"
./whitebox_tools -r=FlightlineOverlap -v ^
--wd="/path/to/data/" -i=file.las -o=outfile.dep ^
--resolution=5.0 --palette=light_quant.plt 


```

*Python function name*: ```flightline_overlap```


#### 6.12.6 LasToAscii

Converts one or more LAS files into ASCII text files.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input LiDAR files

*Command-line Interface*:
```
>>./whitebox_tools -r=LasToAscii -v --wd="/path/to/data/" ^
-i="file1.las, file2.las, file3.las" -o=outfile.las" 


```

*Python function name*: ```las_to_ascii```


#### 6.12.7 LidarColourize

Adds the red-green-blue colour fields of a LiDAR (LAS) file based on an input image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-in_lidar          Input LiDAR file
-\-in_image          Input colour image file
-o, -\-output        Output LiDAR file

*Command-line Interface*:
```
>>./whitebox_tools -r=LidarColourize -v --wd="/path/to/data/" ^
--in_lidar="input.las" --in_image="image.dep" ^
-o="output.las" 


```

*Python function name*: ```lidar_colourize```


#### 6.12.8 LidarElevationSlice

Outputs all of the points within a LiDAR (LAS) point file that lie between a specified elevation range.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file
-o, -\-output        Output LiDAR file
-\-minz              Minimum elevation value (optional)
-\-maxz              Maximum elevation value (optional)
-\-class             Optional boolean flag indicating whether points outside the range should be 
                     retained in output but reclassified 
-\-inclassval        Optional parameter specifying the class value assigned to points within the 
                     slice 
-\-outclassval       Optional parameter specifying the class value assigned to points within the 
                     slice 

*Command-line Interface*:
```
>>./whitebox_tools -r=LidarElevationSlice -v ^
--wd="/path/to/data/" -i="input.las" -o="output.las" ^
--minz=100.0 --maxz=250.0
>>./whitebox_tools ^
-r=LidarElevationSlice -v -i="/path/to/data/input.las" ^
-o="/path/to/data/output.las" --minz=100.0 --maxz=250.0 ^
--class
>>./whitebox_tools -r=LidarElevationSlice -v ^
-i="/path/to/data/input.las" -o="/path/to/data/output.las" ^
--minz=100.0 --maxz=250.0 --inclassval=1 --outclassval=0 


```

*Python function name*: ```lidar_elevation_slice```


#### 6.12.9 LidarGroundPointFilter

Identifies ground points within LiDAR dataset using a slope-based method.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file
-o, -\-output        Output LiDAR file
-\-radius            Search Radius
-\-slope_threshold   Maximum inter-point slope to be considered an off-terrain point
-\-height_threshold  Inter-point height difference to be considered an off-terrain point

*Command-line Interface*:
```
>>./whitebox_tools -r=LidarGroundPointFilter -v ^
--wd="/path/to/data/" -i="input.las" -o="output.las" ^
--radius=10.0 


```

*Python function name*: ```lidar_ground_point_filter```


#### 6.12.10 LidarHillshade

Calculates a hillshade value for points within a LAS file and stores these data in the RGB field.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file
-o, -\-output        Output file
-\-azimuth           Illumination source azimuth in degrees
-\-altitude          Illumination source altitude in degrees
-\-radius            Search Radius

*Command-line Interface*:
```
>>./whitebox_tools -r=LidarHillshade -v --wd="/path/to/data/" ^
-i="input.las" -o="output.las" --radius=10.0
>>./whitebox_tools ^
-r=LidarHillshade -v --wd="/path/to/data/" -i="input.las" ^
-o="output.las" --azimuth=180.0 --altitude=20.0 --radius=1.0 


```

*Python function name*: ```lidar_hillshade```


#### 6.12.11 LidarHistogram

Creates a histogram from LiDAR data.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file
-o, -\-output        Output HTML file (default name will be based on input file if unspecified)
-\-parameter         Parameter; options are 'elevation' (default), 'intensity', 'scan angle', 'class
-\-clip              Amount to clip distribution tails (in percent)

*Command-line Interface*:
```
>>./whitebox_tools -r=LidarHistogram -v --wd="/path/to/data/" ^
-i="file1.tif, file2.tif, file3.tif" -o=outfile.htm ^
--contiguity=Bishopsl 


```

*Python function name*: ```lidar_histogram```


#### 6.12.12 LidarIdwInterpolation

Interpolates LAS files using an inverse-distance weighted (IDW) scheme.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file (including extension)
-o, -\-output        Output raster file (including extension)
-\-parameter         Interpolation parameter; options are 'elevation' (default), 'intensity', 
                     'class', 'scan angle', 'user data' 
-\-returns           Point return types to include; options are 'all' (default), 'last', 'first'
-\-resolution        Output raster's grid resolution
-\-weight            IDW weight value
-\-radius            Search Radius
-\-exclude_cls       Optional exclude classes from interpolation; Valid class values range from 0 to 
                     18, based on LAS specifications. Example, --exclude_cls='3,4,5,6,7,18' 
-\-minz              Optional minimum elevation for inclusion in interpolation
-\-maxz              Optional maximum elevation for inclusion in interpolation

*Command-line Interface*:
```
>>./whitebox_tools -r=LidarIdwInterpolation -v ^
--wd="/path/to/data/" -i=file.las -o=outfile.dep ^
--resolution=2.0 --radius=5.0"
./whitebox_tools ^
-r=LidarIdwInterpolation --wd="/path/to/data/" -i=file.las ^
-o=outfile.dep --resolution=5.0 --weight=2.0 --radius=2.0 ^
--exclude_cls='3,4,5,6,7,18' --palette=light_quant.plt 


```

*Python function name*: ```lidar_idw_interpolation```


#### 6.12.13 LidarInfo

Prints information about a LiDAR (LAS) dataset, including header, point return frequency, and classification data and information about the variable length records (VLRs) and geokeys.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file
-o, -\-output        Output HTML file for regression summary report
-\-vlr               Flag indicating whether or not to print the variable length records (VLRs)
-\-geokeys           Flag indicating whether or not to print the geokeys

*Command-line Interface*:
```
>>./whitebox_tools -r=LidarInfo -v --wd="/path/to/data/" ^
-i=file.las --vlr --geokeys"
./whitebox_tools -r=LidarInfo ^
--wd="/path/to/data/" -i=file.las 


```

*Python function name*: ```lidar_info```


#### 6.12.14 LidarJoin

Joins multiple LiDAR (LAS) files into a single LAS file.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input LiDAR files
-o, -\-output        Output LiDAR file

*Command-line Interface*:
```
>>./whitebox_tools -r=LidarJoin -v --wd="/path/to/data/" ^
-i="file1.las, file2.las, file3.las" -o=outfile.las" 


```

*Python function name*: ```lidar_join```


#### 6.12.15 LidarKappaIndex

Performs a kappa index of agreement (KIA) analysis on the classifications of two LAS files.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-i1, -\-input1     Input LiDAR classification file
-\-i2, -\-input2     Input LiDAR reference file
-o, -\-output        Output HTML file

*Command-line Interface*:
```
>>./whitebox_tools -r=LidarKappaIndex -v ^
--wd="/path/to/data/" --i1=class.tif --i2=reference.tif ^
-o=kia.html 


```

*Python function name*: ```lidar_kappa_index```


#### 6.12.16 LidarNearestNeighbourGridding

Grids LAS files using nearest-neighbour scheme.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file (including extension)
-o, -\-output        Output raster file (including extension)
-\-parameter         Interpolation parameter; options are 'elevation' (default), 'intensity', 
                     'class', 'scan angle', 'user data' 
-\-returns           Point return types to include; options are 'all' (default), 'last', 'first'
-\-resolution        Output raster's grid resolution
-\-radius            Search Radius
-\-exclude_cls       Optional exclude classes from interpolation; Valid class values range from 0 to 
                     18, based on LAS specifications. Example, --exclude_cls='3,4,5,6,7,18' 
-\-minz              Optional minimum elevation for inclusion in interpolation
-\-maxz              Optional maximum elevation for inclusion in interpolation

*Command-line Interface*:
```
>>./whitebox_tools -r=LidarNearestNeighbourGridding -v ^
--wd="/path/to/data/" -i=file.las -o=outfile.dep ^
--resolution=2.0 --radius=5.0"
./whitebox_tools ^
-r=LidarNearestNeighbourGridding --wd="/path/to/data/" ^
-i=file.las -o=outfile.dep --resolution=5.0 --radius=2.0 ^
--exclude_cls='3,4,5,6,7,18' --palette=light_quant.plt 


```

*Python function name*: ```lidar_nearest_neighbour_gridding```


#### 6.12.17 LidarPointDensity

Calculates the spatial pattern of point density for a LiDAR data set.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file (including extension)
-o, -\-output        Output raster file (including extension)
-\-returns           Point return types to include; options are 'all' (default), 'last', 'first'
-\-resolution        Output raster's grid resolution
-\-radius            Search Radius
-\-exclude_cls       Optional exclude classes from interpolation; Valid class values range from 0 to 
                     18, based on LAS specifications. Example, --exclude_cls='3,4,5,6,7,18' 
-\-minz              Optional minimum elevation for inclusion in interpolation
-\-maxz              Optional maximum elevation for inclusion in interpolation

*Command-line Interface*:
```
>>./whitebox_tools -r=LidarPointDensity -v ^
--wd="/path/to/data/" -i=file.las -o=outfile.dep ^
--resolution=2.0 --radius=5.0"
./whitebox_tools ^
-r=LidarPointDensity -v --wd="/path/to/data/" -i=file.las ^
-o=outfile.dep --resolution=5.0 --radius=2.0 ^
--exclude_cls='3,4,5,6,7,18' --palette=light_quant.plt 


```

*Python function name*: ```lidar_point_density```


#### 6.12.18 LidarPointStats

Creates several rasters summarizing the distribution of LAS point data.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file
-\-resolution        Output raster's grid resolution
-\-num_points        Flag indicating whether or not to output the number of points raster
-\-num_pulses        Flag indicating whether or not to output the number of pulses raster
-\-z_range           Flag indicating whether or not to output the elevation range raster
-\-intensity_range   Flag indicating whether or not to output the intensity range raster
-\-predom_class      Flag indicating whether or not to output the predominant classification raster

*Command-line Interface*:
```
>>./whitebox_tools -r=LidarPointStats -v ^
--wd="/path/to/data/" -i=file.las --resolution=1.0 ^
--num_points 


```

*Python function name*: ```lidar_point_stats```


#### 6.12.19 LidarRemoveDuplicates

Removes duplicate points from a LiDAR data set.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file
-o, -\-output        Output LiDAR file
-\-include_z         Include z-values in point comparison?

*Command-line Interface*:
```
>>./whitebox_tools -r=LidarRemoveDuplicates -v ^
--wd="/path/to/data/" -i="input.las" -o="output.las" 


```

*Python function name*: ```lidar_remove_duplicates```


#### 6.12.20 LidarRemoveOutliers

Removes outliers (high and low points) in a LiDAR point cloud.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file
-o, -\-output        Output LiDAR file
-\-radius            Search Radius
-\-elev_diff         Max. elevation difference

*Command-line Interface*:
```
>>./whitebox_tools -r=LidarRemoveOutliers -v ^
--wd="/path/to/data/" -i="input.las" -o="output.las" ^
--radius=10.0 --elev_diff=25.0 


```

*Python function name*: ```lidar_remove_outliers```


#### 6.12.21 LidarSegmentation

Segments a LiDAR point cloud based on normal vectors.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file
-o, -\-output        Output file
-\-dist, -\-radius   Search Radius
-\-norm_diff         Maximum difference in normal vectors, in degrees
-\-maxzdiff          Maximum difference in elevation (z units) between neighbouring points of the 
                     same segment 

*Command-line Interface*:
```
>>./whitebox_tools -r=LidarSegmentation -v ^
--wd="/path/to/data/" -i="input.las" -o="output.las" ^
--radius=10.0 --norm_diff=2.5 --maxzdiff=0.75 


```

*Python function name*: ```lidar_segmentation```


#### 6.12.22 LidarSegmentationBasedFilter

Identifies ground points within LiDAR point clouds using a segmentation based approach.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file
-o, -\-output        Output file
-\-dist, -\-radius   Search Radius
-\-norm_diff         Maximum difference in normal vectors, in degrees
-\-maxzdiff          Maximum difference in elevation (z units) between neighbouring points of the 
                     same segment 
-\-classify          Classify points as ground (2) or off-ground (1)

*Command-line Interface*:
```
>>./whitebox_tools -r=LidarSegmentationBasedFilter -v ^
--wd="/path/to/data/" -i="input.las" -o="output.las" ^
--radius=10.0 --norm_diff=2.5 --maxzdiff=0.75 --classify 


```

*Python function name*: ```lidar_segmentation_based_filter```


#### 6.12.23 LidarTile

Tiles a LiDAR LAS file into multiple LAS files.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file
-\-width_x           Width of tiles in the X dimension; default 1000.0
-\-width_y           Width of tiles in the Y dimension
-\-origin_x          Origin point X coordinate for tile grid
-\-origin_y          Origin point Y coordinate for tile grid
-\-min_points        Minimum number of points contained in a tile for it to be saved

*Command-line Interface*:
```
>>./whitebox_tools -r=LidarTile -v -i=/path/to/data/input.las ^
--width_x=1000.0 --width_y=2500.0 -=min_points=100 


```

*Python function name*: ```lidar_tile```


#### 6.12.24 LidarTophatTransform

Performs a white top-hat transform on a Lidar dataset; as an estimate of height above ground, this is useful for modelling the vegetation canopy.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file
-o, -\-output        Output LiDAR file
-\-radius            Search Radius

*Command-line Interface*:
```
>>./whitebox_tools -r=LidarTophatTransform -v ^
--wd="/path/to/data/" -i="input.las" -o="output.las" ^
--radius=10.0 


```

*Python function name*: ```lidar_tophat_transform```


#### 6.12.25 NormalVectors

Calculates normal vectors for points within a LAS file and stores these data (XYZ vector components) in the RGB field.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input LiDAR file
-o, -\-output        Output LiDAR file
-\-radius            Search Radius

*Command-line Interface*:
```
>>./whitebox_tools -r=NormalVectors -v --wd="/path/to/data/" ^
-i="input.las" -o="output.las" --radius=10.0 


```

*Python function name*: ```normal_vectors```

### 6.13 Math and Stats Tools

#### 6.13.1 AbsoluteValue

Calculates the absolute value of every cell in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=AbsoluteValue -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```absolute_value```


#### 6.13.2 Add

Performs an addition operation on two rasters or a raster and a constant value.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-input1            Input raster file or constant value
-\-input2            Input raster file or constant value
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Add -v --wd="/path/to/data/" ^
--input1='in1.dep' --input2='in2.dep' -o=output.dep 


```

*Python function name*: ```add```


#### 6.13.3 And

Performs a logical AND operator on two Boolean raster images.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-input1            Input raster file
-\-input2            Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=And -v --wd="/path/to/data/" ^
--input1='in1.dep' --input2='in2.dep' -o=output.dep 


```

*Python function name*: ```And```


#### 6.13.4 Anova

Performs an analysis of variance (ANOVA) test on a raster dataset.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-\-features          Feature definition (or class) raster
-o, -\-output        Output HTML file

*Command-line Interface*:
```
>>./whitebox_tools -r=Anova -v --wd="/path/to/data/" ^
-i=data.tif --features=classes.tif -o=anova.html 


```

*Python function name*: ```anova```


#### 6.13.5 ArcCos

Returns the inverse cosine (arccos) of each values in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=ArcCos -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```arc_cos```


#### 6.13.6 ArcSin

Returns the inverse sine (arcsin) of each values in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=ArcSin -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```arc_sin```


#### 6.13.7 ArcTan

Returns the inverse tangent (arctan) of each values in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=ArcTan -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```arc_tan```


#### 6.13.8 Atan2

Returns the 2-argument inverse tangent (atan2).

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-input_y           Input y raster file or constant value (rise)
-\-input_x           Input x raster file or constant value (run)
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Atan2 -v --wd="/path/to/data/" ^
--input_y='in1.dep' --input_x='in2.dep' -o=output.dep 


```

*Python function name*: ```atan2```


#### 6.13.9 Ceil

Returns the smallest (closest to negative infinity) value that is greater than or equal to the values in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Ceil -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```ceil```


#### 6.13.10 Cos

Returns the cosine (cos) of each values in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Cos -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```cos```


#### 6.13.11 Cosh

Returns the hyperbolic cosine (cosh) of each values in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Cosh -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```cosh```


#### 6.13.12 CrispnessIndex

Calculates the Crispness Index, which is used to quantify how crisp (or conversely how fuzzy) a probability image is.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Optional output html file (default name will be based on input file if 
                     unspecified) 

*Command-line Interface*:
```
>>./whitebox_tools -r=CrispnessIndex -v --wd="/path/to/data/" ^
-i=input.dep
>>./whitebox_tools -r=CrispnessIndex -v ^
--wd="/path/to/data/" -o=crispness.html 


```

*Python function name*: ```crispness_index```


#### 6.13.13 CrossTabulation

Performs a cross-tabulation on two categorical images.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-i1, -\-input1     Input raster file 1
-\-i2, -\-input2     Input raster file 1
-o, -\-output        Output HTML file (default name will be based on input file if unspecified)

*Command-line Interface*:
```
>>./whitebox_tools -r=CrossTabulation -v ^
--wd="/path/to/data/" --i1="file1.tif" --i2="file2.tif" ^
-o=outfile.html 


```

*Python function name*: ```cross_tabulation```


#### 6.13.14 CumulativeDistribution

Converts a raster image to its cumulative distribution function.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=CumulativeDistribution -v ^
--wd="/path/to/data/" -i=DEM.dep -o=output.dep 


```

*Python function name*: ```cumulative_distribution```


#### 6.13.15 Decrement

Decreases the values of each grid cell in an input raster by 1.0.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Decrement -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```decrement```


#### 6.13.16 Divide

Performs a division operation on two rasters or a raster and a constant value.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-input1            Input raster file or constant value
-\-input2            Input raster file or constant value
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Divide -v --wd="/path/to/data/" ^
--input1='in1.dep' --input2='in2.dep' -o=output.dep 


```

*Python function name*: ```divide```


#### 6.13.17 EqualTo

Performs a equal-to comparison operation on two rasters or a raster and a constant value.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-input1            Input raster file or constant value
-\-input2            Input raster file or constant value
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=EqualTo -v --wd="/path/to/data/" ^
--input1='in1.dep' --input2='in2.dep' -o=output.dep 


```

*Python function name*: ```equal_to```


#### 6.13.18 Exp

Returns the exponential (base e) of values in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Exp -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```exp```


#### 6.13.19 Exp2

Returns the exponential (base 2) of values in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Exp2 -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```exp2```


#### 6.13.20 ExtractRasterStatistics

Extracts descriptive statistics for a group of patches in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input data raster file
-\-features          Input feature definition raster file
-o, -\-output        Output raster file
-\-stat              Statistic to extract
-\-out_table         Output HTML Table file

*Command-line Interface*:
```
>>./whitebox_tools -r=ExtractRasterStatistics -v ^
--wd="/path/to/data/" -i='input.dep' --features='groups.dep' ^
-o='output.dep' --stat='minimum'
>>./whitebox_tools ^
-r=ExtractRasterStatistics -v --wd="/path/to/data/" ^
-i='input.dep' --features='groups.dep' ^
--out_table='output.html' 


```

*Python function name*: ```extract_raster_statistics```


#### 6.13.21 Floor

Returns the largest (closest to positive infinity) value that is less than or equal to the values in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Floor -v --wd="/path/to/data/" ^
-i='input.dep' -o='output.dep' 


```

*Python function name*: ```floor```


#### 6.13.22 GreaterThan

Performs a greater-than comparison operation on two rasters or a raster and a constant value.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-input1            Input raster file or constant value
-\-input2            Input raster file or constant value
-o, -\-output        Output raster file
-\-incl_equals       Perform a greater-than-or-equal-to operation

*Command-line Interface*:
```
>>./whitebox_tools -r=GreaterThan -v --wd="/path/to/data/" ^
--input1='in1.dep' --input2='in2.dep' -o=output.dep ^
--incl_equals 


```

*Python function name*: ```greater_than```


#### 6.13.23 ImageAutocorrelation

Performs Moran's I analysis on two or more input images.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input raster files
-\-contiguity        Contiguity type
-o, -\-output        Output HTML file (default name will be based on input file if unspecified)

*Command-line Interface*:
```
>>./whitebox_tools -r=ImageAutocorrelation -v ^
--wd="/path/to/data/" -i="file1.tif, file2.tif, file3.tif" ^
-o=outfile.html --contiguity=Bishops 


```

*Python function name*: ```image_autocorrelation```


#### 6.13.24 ImageCorrelation

Performs image correlation on two or more input images.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-inputs        Input raster files
-o, -\-output        Output HTML file (default name will be based on input file if unspecified)

*Command-line Interface*:
```
>>./whitebox_tools -r=ImageCorrelation -v ^
--wd="/path/to/data/" -i="file1.tif, file2.tif, file3.tif" ^
-o=outfile.html 


```

*Python function name*: ```image_correlation```


#### 6.13.25 ImageRegression

Performs image regression analysis on two input images.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-i1, -\-input1     Input raster file (independent variable, X)
-\-i2, -\-input2     Input raster file (dependent variable, Y)
-o, -\-output        Output HTML file for regression summary report
-\-out_residuals     Output raster regression resdidual file
-\-standardize       Optional flag indicating whether to standardize the residuals map

*Command-line Interface*:
```
>>./whitebox_tools -r=ImageRegression -v ^
--wd="/path/to/data/" --i1='file1.tif' --i2='file2.tif' ^
-o='outfile.html' --out_residuals='residuals.tif' ^
--standardize 


```

*Python function name*: ```image_regression```


#### 6.13.26 Increment

Increases the values of each grid cell in an input raster by 1.0.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Increment -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```increment```


#### 6.13.27 IntegerDivision

Performs an integer division operation on two rasters or a raster and a constant value.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-input1            Input raster file or constant value
-\-input2            Input raster file or constant value
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=IntegerDivision -v ^
--wd="/path/to/data/" --input1='in1.dep' --input2='in2.dep' ^
-o=output.dep 


```

*Python function name*: ```integer_division```


#### 6.13.28 IsNoData

Identifies NoData valued pixels in an image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=IsNoData -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```is_no_data```


#### 6.13.29 KSTestForNormality

Evaluates whether the values in a raster are normally distributed.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output HTML file
-\-num_samples       Number of samples. Leave blank to use whole image

*Command-line Interface*:
```
>>./whitebox_tools -r=KSTestForNormality -v ^
--wd="/path/to/data/" -i=input.dep -o=output.html ^
--num_samples=1000
>>./whitebox_tools -r=KSTestForNormality -v ^
--wd="/path/to/data/" -i=input.dep -o=output.html 


```

*Python function name*: ```ks_test_for_normality```


#### 6.13.30 KappaIndex

Performs a kappa index of agreement (KIA) analysis on two categorical raster files.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-i1, -\-input1     Input classification raster file
-\-i2, -\-input2     Input reference raster file
-o, -\-output        Output HTML file

*Command-line Interface*:
```
>>./whitebox_tools -r=KappaIndex -v --wd="/path/to/data/" ^
--i1=class.tif --i2=reference.tif -o=kia.html 


```

*Python function name*: ```kappa_index```


#### 6.13.31 LessThan

Performs a less-than comparison operation on two rasters or a raster and a constant value.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-input1            Input raster file or constant value
-\-input2            Input raster file or constant value
-o, -\-output        Output raster file
-\-incl_equals       Perform a less-than-or-equal-to operation

*Command-line Interface*:
```
>>./whitebox_tools -r=LessThan -v --wd="/path/to/data/" ^
--input1='in1.dep' --input2='in2.dep' -o=output.dep ^
--incl_equals 


```

*Python function name*: ```less_than```


#### 6.13.32 Ln

Returns the natural logarithm of values in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Ln -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```ln```


#### 6.13.33 Log10

Returns the base-10 logarithm of values in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Log10 -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```log10```


#### 6.13.34 Log2

Returns the base-2 logarithm of values in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Log2 -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```log2```


#### 6.13.35 Max

Performs a MAX operation on two rasters or a raster and a constant value.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-input1            Input raster file or constant value
-\-input2            Input raster file or constant value
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Max -v --wd="/path/to/data/" ^
--input1='in1.dep' --input2='in2.dep' -o=output.dep 


```

*Python function name*: ```max```


#### 6.13.36 Min

Performs a MIN operation on two rasters or a raster and a constant value.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-input1            Input raster file or constant value
-\-input2            Input raster file or constant value
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Min -v --wd="/path/to/data/" ^
--input1='in1.dep' --input2='in2.dep' -o=output.dep 


```

*Python function name*: ```min```


#### 6.13.37 Modulo

Performs a modulo operation on two rasters or a raster and a constant value.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-input1            Input raster file or constant value
-\-input2            Input raster file or constant value
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Modulo -v --wd="/path/to/data/" ^
--input1='in1.dep' --input2='in2.dep' -o=output.dep 


```

*Python function name*: ```modulo```


#### 6.13.38 Multiply

Performs a multiplication operation on two rasters or a raster and a constant value.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-input1            Input raster file or constant value
-\-input2            Input raster file or constant value
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Multiply -v --wd="/path/to/data/" ^
--input1='in1.dep' --input2='in2.dep' -o=output.dep 


```

*Python function name*: ```multiply```


#### 6.13.39 Negate

Changes the sign of values in a raster or the 0-1 values of a Boolean raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Negate -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```negate```


#### 6.13.40 Not

Performs a logical NOT operator on two Boolean raster images.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-input1            Input raster file
-\-input2            Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Not -v --wd="/path/to/data/" ^
--input1='in1.dep' --input2='in2.dep' -o=output.dep 


```

*Python function name*: ```Not```


#### 6.13.41 NotEqualTo

Performs a not-equal-to comparison operation on two rasters or a raster and a constant value.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-input1            Input raster file or constant value
-\-input2            Input raster file or constant value
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=NotEqualTo -v --wd="/path/to/data/" ^
--input1='in1.dep' --input2='in2.dep' -o=output.dep 


```

*Python function name*: ```not_equal_to```


#### 6.13.42 Or

Performs a logical OR operator on two Boolean raster images.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-input1            Input raster file
-\-input2            Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Or -v --wd="/path/to/data/" ^
--input1='in1.dep' --input2='in2.dep' -o=output.dep 


```

*Python function name*: ```Or```


#### 6.13.43 Power

Raises the values in grid cells of one rasters, or a constant value, by values in another raster or constant value.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-input1            Input raster file or constant value
-\-input2            Input raster file or constant value
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Power -v --wd="/path/to/data/" ^
--input1='in1.dep' --input2='in2.dep' -o=output.dep 


```

*Python function name*: ```power```


#### 6.13.44 Quantiles

Transforms raster values into quantiles.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-num_quantiles     Number of quantiles

*Command-line Interface*:
```
>>./whitebox_tools -r=Quantiles -v --wd="/path/to/data/" ^
-i=DEM.dep -o=output.dep --num_quantiles=5 


```

*Python function name*: ```quantiles```


#### 6.13.45 RandomField

Creates an image containing random values.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-base          Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=RandomField -v --wd="/path/to/data/" ^
--base=in.dep -o=out.dep 


```

*Python function name*: ```random_field```


#### 6.13.46 RandomSample

Creates an image containing randomly located sample grid cells with unique IDs.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-base          Input raster file
-o, -\-output        Output raster file
-\-num_samples       Number of samples

*Command-line Interface*:
```
>>./whitebox_tools -r=RandomSample -v --wd="/path/to/data/" ^
--base=in.dep -o=out.dep --num_samples=1000 


```

*Python function name*: ```random_sample```


#### 6.13.47 RasterHistogram

Creates a histogram from raster values.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output HTML file (default name will be based on input file if unspecified)

*Command-line Interface*:
```
>>./whitebox_tools -r=RasterHistogram -v ^
--wd="/path/to/data/" -i="file1.tif" -o=outfile.html 


```

*Python function name*: ```raster_histogram```


#### 6.13.48 RasterSummaryStats

Measures a rasters average, standard deviation, num. non-nodata cells, and total.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=RasterSummaryStats -v ^
--wd="/path/to/data/" -i=DEM.dep 


```

*Python function name*: ```raster_summary_stats```


#### 6.13.49 Reciprocal

Returns the reciprocal (i.e. 1 / z) of values in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Reciprocal -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```reciprocal```


#### 6.13.50 RescaleValueRange

Performs a min-max contrast stretch on an input greytone image.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-out_min_val       New minimum value in output image
-\-out_max_val       New maximum value in output image
-\-clip_min          Optional lower tail clip value
-\-clip_max          Optional upper tail clip value

*Command-line Interface*:
```
>>./whitebox_tools -r=RescaleValueRange -v ^
--wd="/path/to/data/" -i=input.dep -o=output.dep ^
--out_min_val=0.0 --out_max_val=1.0
>>./whitebox_tools ^
-r=RescaleValueRange -v --wd="/path/to/data/" -i=input.dep ^
-o=output.dep --out_min_val=0.0 --out_max_val=1.0 ^
--clip_min=45.0 --clip_max=200.0 


```

*Python function name*: ```rescale_value_range```


#### 6.13.51 RootMeanSquareError

Calculates the RMSE and other accuracy statistics.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-\-base              Input base raster file used for comparison

*Command-line Interface*:
```
>>./whitebox_tools -r=RootMeanSquareError -v ^
--wd="/path/to/data/" -i=DEM.dep 


```

*Python function name*: ```root_mean_square_error```


#### 6.13.52 Round

Rounds the values in an input raster to the nearest integer value.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Round -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```round```


#### 6.13.53 Sin

Returns the sine (sin) of each values in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Sin -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```sin```


#### 6.13.54 Sinh

Returns the hyperbolic sine (sinh) of each values in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Sinh -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```sinh```


#### 6.13.55 Square

Squares the values in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Square -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```square```


#### 6.13.56 SquareRoot

Returns the square root of the values in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=SquareRoot -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```square_root```


#### 6.13.57 Subtract

Performs a differencing operation on two rasters or a raster and a constant value.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-input1            Input raster file or constant value
-\-input2            Input raster file or constant value
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Subtract -v --wd="/path/to/data/" ^
--input1='in1.dep' --input2='in2.dep' -o=output.dep 


```

*Python function name*: ```subtract```


#### 6.13.58 Tan

Returns the tangent (tan) of each values in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Tan -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```tan```


#### 6.13.59 Tanh

Returns the hyperbolic tangent (tanh) of each values in a raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Tanh -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```tanh```


#### 6.13.60 ToDegrees

Converts a raster from radians to degrees.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=ToDegrees -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```to_degrees```


#### 6.13.61 ToRadians

Converts a raster from degrees to radians.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=ToRadians -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep 


```

*Python function name*: ```to_radians```


#### 6.13.62 Truncate

Truncates the values in a raster to the desired number of decimal places.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file
-\-num_decimals      Number of decimals left after truncation (default is zero)

*Command-line Interface*:
```
>>./whitebox_tools -r=Truncate -v --wd="/path/to/data/" ^
-i='input.dep' -o=output.dep --num_decimals=2 


```

*Python function name*: ```truncate```


#### 6.13.63 TurningBandsSimulation

Creates an image containing random values based on a turning-bands simulation.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-base          Input base raster file
-o, -\-output        Output file
-\-range             The field's range, in xy-units, related to the extent of spatial autocorrelation
-\-iterations        The number of iterations

*Command-line Interface*:
```
>>./whitebox_tools -r=TurningBandsSimulation -v ^
--wd="/path/to/data/" --base=in.dep -o=out.dep --range=850.0 ^
--iterations=2500 


```

*Python function name*: ```turning_bands_simulation```


#### 6.13.64 Xor

Performs a logical XOR operator on two Boolean raster images.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-input1            Input raster file
-\-input2            Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=Xor -v --wd="/path/to/data/" ^
--input1='in1.dep' --input2='in2.dep' -o=output.dep 


```

*Python function name*: ```xor```


#### 6.13.65 ZScores

Standardizes the values in an input raster by converting to z-scores.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-input         Input raster file
-o, -\-output        Output raster file

*Command-line Interface*:
```
>>./whitebox_tools -r=ZScores -v --wd="/path/to/data/" ^
-i=DEM.dep -o=output.dep 


```

*Python function name*: ```z_scores```

### 6.14 Stream Network Analysis

#### 6.14.1 DistanceToOutlet

Calculates the distance of stream grid cells to the channel network outlet cell.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-\-streams           Input raster streams file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme
-\-zero_background   Flag indicating whether a background value of zero should be used

*Command-line Interface*:
```
>>./whitebox_tools -r=DistanceToOutlet -v ^
--wd="/path/to/data/" --d8_pntr=D8.dep --streams=streams.dep ^
-o=output.dep
>>./whitebox_tools -r=DistanceToOutlet -v ^
--wd="/path/to/data/" --d8_pntr=D8.flt --streams=streams.flt ^
-o=output.flt --esri_pntr --zero_background 


```

*Python function name*: ```distance_to_outlet```


#### 6.14.2 ExtractStreams

Extracts stream grid cells from a flow accumulation raster.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-flow_accum        Input raster D8 flow accumulation file
-o, -\-output        Output raster file
-\-threshold         Threshold in flow accumulation values for channelization
-\-zero_background   Flag indicating whether a background value of zero should be used

*Command-line Interface*:
```
>>./whitebox_tools -r=ExtractStreams -v --wd="/path/to/data/" ^
--flow_accum='d8accum.dep' -o='output.dep' --threshold=100.0 ^
--zero_background 


```

*Python function name*: ```extract_streams```


#### 6.14.3 ExtractValleys

Identifies potential valley bottom grid cells based on local topolography alone.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-variant           Options include 'lq' (lower quartile), 'JandR' (Johnston and Rosenfeld), and 
                     'PandD' (Peucker and Douglas); default is 'lq' 
-\-line_thin         Optional flag indicating whether post-processing line-thinning should be 
                     performed 
-\-filter            Optional argument (only used when variant='lq') providing the filter size, in 
                     grid cells, used for lq-filtering (default is 5) 

*Command-line Interface*:
```
>>./whitebox_tools -r=ExtractValleys -v --wd="/path/to/data/" ^
--dem=pointer.dep -o=out.dep --variant='JandR' ^
--line_thin
>>./whitebox_tools -r=ExtractValleys -v ^
--wd="/path/to/data/" --dem=pointer.dep -o=out.dep ^
--variant='lq' --filter=7 --line_thin 


```

*Python function name*: ```extract_valleys```


#### 6.14.4 FarthestChannelHead

Calculates the distance to the furthest upstream channel head for each stream cell.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-\-streams           Input raster streams file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme
-\-zero_background   Flag indicating whether a background value of zero should be used

*Command-line Interface*:
```
>>./whitebox_tools -r=FarthestChannelHead -v ^
--wd="/path/to/data/" --d8_pntr=D8.dep --streams=streams.dep ^
-o=output.dep
>>./whitebox_tools -r=FarthestChannelHead -v ^
--wd="/path/to/data/" --d8_pntr=D8.flt --streams=streams.flt ^
-o=output.flt --esri_pntr --zero_background 


```

*Python function name*: ```farthest_channel_head```


#### 6.14.5 FindMainStem

Finds the main stem, based on stream lengths, of each stream network.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-\-streams           Input raster streams file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme
-\-zero_background   Flag indicating whether a background value of zero should be used

*Command-line Interface*:
```
>>./whitebox_tools -r=FindMainStem -v --wd="/path/to/data/" ^
--d8_pntr=D8.dep --streams=streams.dep ^
-o=output.dep
>>./whitebox_tools -r=FindMainStem -v ^
--wd="/path/to/data/" --d8_pntr=D8.flt --streams=streams.flt ^
-o=output.flt --esri_pntr --zero_background 


```

*Python function name*: ```find_main_stem```


#### 6.14.6 HackStreamOrder

Assigns the Hack stream order to each tributary in a stream network.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-\-streams           Input raster streams file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme
-\-zero_background   Flag indicating whether a background value of zero should be used

*Command-line Interface*:
```
>>./whitebox_tools -r=HackStreamOrder -v ^
--wd="/path/to/data/" --d8_pntr=D8.dep --streams=streams.dep ^
-o=output.dep
>>./whitebox_tools -r=HackStreamOrder -v ^
--wd="/path/to/data/" --d8_pntr=D8.flt --streams=streams.flt ^
-o=output.flt --esri_pntr --zero_background 


```

*Python function name*: ```hack_stream_order```


#### 6.14.7 HortonStreamOrder

Assigns the Horton stream order to each tributary in a stream network.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-\-streams           Input raster streams file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme
-\-zero_background   Flag indicating whether a background value of zero should be used

*Command-line Interface*:
```
>>./whitebox_tools -r=HortonStreamOrder -v ^
--wd="/path/to/data/" --d8_pntr=D8.dep --streams=streams.dep ^
-o=output.dep
>>./whitebox_tools -r=HortonStreamOrder -v ^
--wd="/path/to/data/" --d8_pntr=D8.flt --streams=streams.flt ^
-o=output.flt --esri_pntr --zero_background 


```

*Python function name*: ```horton_stream_order```


#### 6.14.8 LengthOfUpstreamChannels

Calculates the total length of channels upstream.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-\-streams           Input raster streams file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme
-\-zero_background   Flag indicating whether a background value of zero should be used

*Command-line Interface*:
```
>>./whitebox_tools -r=LengthOfUpstreamChannels -v ^
--wd="/path/to/data/" --d8_pntr=D8.dep --streams=streams.dep ^
-o=output.dep
>>./whitebox_tools -r=LengthOfUpstreamChannels -v ^
--wd="/path/to/data/" --d8_pntr=D8.flt --streams=streams.flt ^
-o=output.flt --esri_pntr --zero_background 


```

*Python function name*: ```length_of_upstream_channels```


#### 6.14.9 LongProfile

Plots the stream longitudinal profiles for one or more rivers.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-\-streams           Input raster streams file
-\-dem               Input raster DEM file
-o, -\-output        Output HTML file
-\-esri_pntr         D8 pointer uses the ESRI style scheme

*Command-line Interface*:
```
>>./whitebox_tools -r=LongProfile -v --wd="/path/to/data/" ^
--d8_pntr=D8.dep --streams=streams.dep --dem=dem.dep ^
-o=output.html --esri_pntr 


```

*Python function name*: ```long_profile```


#### 6.14.10 LongProfileFromPoints

Plots the longitudinal profiles from flow-paths initiating from a set of vector points.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-\-points            Input vector points file
-\-dem               Input raster DEM file
-o, -\-output        Output HTML file
-\-esri_pntr         D8 pointer uses the ESRI style scheme

*Command-line Interface*:
```
>>./whitebox_tools -r=LongProfileFromPoints -v ^
--wd="/path/to/data/" --d8_pntr=D8.dep --points=stream_head.shp ^
--dem=dem.dep -o=output.html --esri_pntr 


```

*Python function name*: ```long_profile_from_points```


#### 6.14.11 RemoveShortStreams

Removes short first-order streams from a stream network.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-\-streams           Input raster streams file
-o, -\-output        Output raster file
-\-min_length        Minimum tributary length (in map units) used for network prunning
-\-esri_pntr         D8 pointer uses the ESRI style scheme

*Command-line Interface*:
```
>>./whitebox_tools -r=RemoveShortStreams -v ^
--wd="/path/to/data/" --d8_pntr=D8.dep --streams=streams.dep ^
-o=output.dep 


```

*Python function name*: ```remove_short_streams```


#### 6.14.12 ShreveStreamMagnitude

Assigns the Shreve stream magnitude to each link in a stream network.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-\-streams           Input raster streams file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme
-\-zero_background   Flag indicating whether a background value of zero should be used

*Command-line Interface*:
```
>>./whitebox_tools -r=ShreveStreamMagnitude -v ^
--wd="/path/to/data/" --d8_pntr=D8.dep --streams=streams.dep ^
-o=output.dep
>>./whitebox_tools -r=ShreveStreamMagnitude -v ^
--wd="/path/to/data/" --d8_pntr=D8.flt --streams=streams.flt ^
-o=output.flt --esri_pntr --zero_background 


```

*Python function name*: ```shreve_stream_magnitude```


#### 6.14.13 StrahlerStreamOrder

Assigns the Strahler stream order to each link in a stream network.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-\-streams           Input raster streams file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme
-\-zero_background   Flag indicating whether a background value of zero should be used

*Command-line Interface*:
```
>>./whitebox_tools -r=StrahlerStreamOrder -v ^
--wd="/path/to/data/" --d8_pntr=D8.dep --streams=streams.dep ^
-o=output.dep
>>./whitebox_tools -r=StrahlerStreamOrder -v ^
--wd="/path/to/data/" --d8_pntr=D8.flt --streams=streams.flt ^
-o=output.flt --esri_pntr --zero_background 


```

*Python function name*: ```strahler_stream_order```


#### 6.14.14 StreamLinkClass

Identifies the exterior/interior links and nodes in a stream network.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-\-streams           Input raster streams file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme
-\-zero_background   Flag indicating whether a background value of zero should be used

*Command-line Interface*:
```
>>./whitebox_tools -r=StreamLinkClass -v ^
--wd="/path/to/data/" --d8_pntr=D8.dep --streams=streams.dep ^
-o=output.dep
>>./whitebox_tools -r=StreamLinkClass -v ^
--wd="/path/to/data/" --d8_pntr=D8.flt --streams=streams.flt ^
-o=output.flt --esri_pntr --zero_background 


```

*Python function name*: ```stream_link_class```


#### 6.14.15 StreamLinkIdentifier

Assigns a unique identifier to each link in a stream network.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-\-streams           Input raster streams file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme
-\-zero_background   Flag indicating whether a background value of zero should be used

*Command-line Interface*:
```
>>./whitebox_tools -r=StreamLinkIdentifier -v ^
--wd="/path/to/data/" --d8_pntr=D8.dep --streams=streams.dep ^
-o=output.dep
>>./whitebox_tools -r=StreamLinkIdentifier -v ^
--wd="/path/to/data/" --d8_pntr=D8.flt --streams=streams.flt ^
-o=output.flt --esri_pntr --zero_background 


```

*Python function name*: ```stream_link_identifier```


#### 6.14.16 StreamLinkLength

Estimates the length of each link (or tributary) in a stream network.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-\-linkid            Input raster streams link ID (or tributary ID) file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme
-\-zero_background   Flag indicating whether a background value of zero should be used

*Command-line Interface*:
```
>>./whitebox_tools -r=StreamLinkLength -v ^
--wd="/path/to/data/" --d8_pntr=D8.dep --linkid=streamsID.dep ^
--dem=dem.dep -o=output.dep
>>./whitebox_tools ^
-r=StreamLinkLength -v --wd="/path/to/data/" --d8_pntr=D8.flt ^
--linkid=streamsID.flt --dem=dem.flt -o=output.flt --esri_pntr ^
--zero_background 


```

*Python function name*: ```stream_link_length```


#### 6.14.17 StreamLinkSlope

Estimates the average slope of each link (or tributary) in a stream network.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-\-linkid            Input raster streams link ID (or tributary ID) file
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme
-\-zero_background   Flag indicating whether a background value of zero should be used

*Command-line Interface*:
```
>>./whitebox_tools -r=StreamLinkSlope -v ^
--wd="/path/to/data/" --d8_pntr=D8.dep --linkid=streamsID.dep ^
--dem=dem.dep -o=output.dep
>>./whitebox_tools ^
-r=StreamLinkSlope -v --wd="/path/to/data/" --d8_pntr=D8.flt ^
--linkid=streamsID.flt --dem=dem.flt -o=output.flt --esri_pntr ^
--zero_background 


```

*Python function name*: ```stream_link_slope```


#### 6.14.18 StreamSlopeContinuous

Estimates the slope of each grid cell in a stream network.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-\-streams           Input raster streams file
-i, -\-dem           Input raster DEM file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme
-\-zero_background   Flag indicating whether a background value of zero should be used

*Command-line Interface*:
```
>>./whitebox_tools -r=StreamSlopeContinuous -v ^
--wd="/path/to/data/" --d8_pntr=D8.dep --linkid=streamsID.dep ^
--dem=dem.dep -o=output.dep
>>./whitebox_tools ^
-r=StreamSlopeContinuous -v --wd="/path/to/data/" ^
--d8_pntr=D8.flt --streams=streamsID.flt --dem=dem.flt ^
-o=output.flt --esri_pntr --zero_background 


```

*Python function name*: ```stream_slope_continuous```


#### 6.14.19 TopologicalStreamOrder

Assigns each link in a stream network its topological order.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-\-streams           Input raster streams file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme
-\-zero_background   Flag indicating whether a background value of zero should be used

*Command-line Interface*:
```
>>./whitebox_tools -r=TopologicalStreamOrder -v ^
--wd="/path/to/data/" --d8_pntr=D8.dep --streams=streams.dep ^
-o=output.dep
>>./whitebox_tools -r=TopologicalStreamOrder -v ^
--wd="/path/to/data/" --d8_pntr=D8.flt --streams=streams.flt ^
-o=output.flt --esri_pntr --zero_background 


```

*Python function name*: ```topological_stream_order```


#### 6.14.20 TributaryIdentifier

Assigns a unique identifier to each tributary in a stream network.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
-\-d8_pntr           Input raster D8 pointer file
-\-streams           Input raster streams file
-o, -\-output        Output raster file
-\-esri_pntr         D8 pointer uses the ESRI style scheme
-\-zero_background   Flag indicating whether a background value of zero should be used

*Command-line Interface*:
```
>>./whitebox_tools -r=TributaryIdentifier -v ^
--wd="/path/to/data/" --d8_pntr=D8.dep --streams=streams.dep ^
-o=output.dep
>>./whitebox_tools -r=TributaryIdentifier -v ^
--wd="/path/to/data/" --d8_pntr=D8.flt --streams=streams.flt ^
-o=output.flt --esri_pntr --zero_background 


```

*Python function name*: ```tributary_identifier```















## 7. Supported Data Formats
The **WhiteboxTools** library can currently support reading/writing raster data in [*Whitebox GAT*](http://www.uoguelph.ca/~hydrogeo/Whitebox/), GeoTIFF, ESRI (ArcGIS) ASCII and binary (.flt & .hdr), GRASS GIS, Idrisi, SAGA GIS (binary and ASCII), and Surfer 7 data formats. The library is primarily tested using Whitebox raster data sets and if you encounter issues when reading/writing data in other formats, you should report the [issue](#reporting-bugs). Please note that there are no plans to incorporate third-party libraries, like [GDAL](http://www.gdal.org), in the project given the design goal of keeping a pure (or as close as possible) Rust codebase. 

At present, there is limited ability in *WhiteboxTools* to read vector geospatial data. Support for Shapefile (and other common vector formats) will be enhanced within the library soon. 

LiDAR data can be read/written in the common [LAS](https://www.asprs.org/committee-general/laser-las-file-format-exchange-activities.html) data format. *WhiteboxTools* can read and write LAS files that have been compressed (zipped with a .zip extension) using the common DEFLATE algorithm. Note that only LAS file should be contained within a zipped archive file. The compressed LiDAR format LAZ and ESRI LiDAR format are not currently supported by the library. The following is an example of running a LiDAR tool using zipped input/output files:

```
>>./whitebox_tools -r=LidarTophatTransform -v --wd="/path/to/data/" 
-i="input.las.zip" -o="output.las.zip" --radius=10.0
```

Note that the double extensions (.las.zip) in the above command are not necessary and are only used for convenience of keeping track of LiDAR data sets (i.e. .zip extensions work too). The extra work of decoding/encoding compressed files does add additional processing time, although the Rust compression library that is used is highly efficient and usually only adds a few seconds to tool run times. Zipping LAS files frequently results 40-60% smaller binary files, making the additional processing time worthwhile for larger LAS file data sets with massive storage requirements. 

## 8. Contributing

If you would like to contribute to the project as a developer, follow these instructions to get started:

1. Fork the larger Whitebox project (in which whitebox-tools exists) ( https://github.com/jblindsay/whitebox-geospatial-analysis-tools )
2. Create your feature branch (git checkout -b my-new-feature)
3. Commit your changes (git commit -am 'Add some feature')
4. Push to the branch (git push origin my-new-feature)
5. Create a new Pull Request

Unless explicitly stated otherwise, any contribution intentionally submitted for inclusion in the work shall be licensed [as above](#license) without any additional terms or conditions.

If you would like to contribute financial support for the project, please contact [John Lindsay](http://www.uoguelph.ca/~hydrogeo/index.html). We also welcome contributions in the form of media exposure. If you have written an article or blog about *WhiteboxTools* please let us know about it.

## 9. Reporting Bugs

WhiteboxTools is distributed as is and without warranty of suitability for application. If you encounter flaws with the software (i.e. bugs) please report the issue. Providing a detailed description of the conditions under which the bug occurred will help to identify the bug. *Use the Issues tracker on GitHub to report issues with the software and to request feature enchancements.* Please do not email Dr. Lindsay directly with bugs.

## 10. Known Issues and Limitations

- There is limited support for reading, writing, or analyzing vector data yet. Plans include native support for the ESRI Shapefile format and possibly GeoJSON data.
- The LAZ compressed LiDAR data format is currently unsupported although zipped LAS files (.zip) are.
- There is no support for reading waveform data contained within or associated with LAS files.
- File directories cannot contain apostrophes (', e.g. /John's data/) as they will be interpreted in the arguments array as single quoted strings.
- The Python scripts included with **WhiteboxTools** require Python 3. They will not work with Python 2, which is frequently the default Python version installed on many systems.

## 11. License

The **WhiteboxTools** library is distributed under the [MIT license](LICENSE.txt), a permissive open-source (free software) license.

> The MIT License (MIT)
> 
> Copyright (c) 2017-2018 John Lindsay
> 
> Permission is hereby granted, free of charge, to any person obtaining a copy
> of this software and associated documentation files (the "Software"), to deal
> in the Software without restriction, including without limitation the rights
> to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
> copies of the Software, and to permit persons to whom the Software is
> furnished to do so, subject to the following conditions:
> 
> The above copyright notice and this permission notice shall be included in all
> copies or substantial portions of the Software.
> 
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
> IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
> FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
> AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
> LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
> SOFTWARE.

## 12. Frequently Asked Questions

### 12.1 Do I need Whitebox GAT to use WhiteboxTools?

No you do not. You can call the tools contained within *WhiteboxTools* completely independent from the *Whitebox GAT* user interface using a Remote Procedure Call (RPC) approach. In fact, you can interact with the tools using Python scripting or directly, using a terminal application (command prompt). See [*Interacting With *WhiteboxTools* From the Command Prompt*](#interacting-with-whiteboxtools-from-the-command-prompt) for further details.

### 12.2 How do I request a tool be added?

Eventually most of the tools in *Whitebox GAT* will be ported over to *WhiteboxTools* and all new tools will be added to this library as well. Naturally, this will take time. The order by which tools are ported is partly a function of ease of porting, existing infrastructure (i.e. raster and LiDAR tools will be ported first since their is currently no support in the library for vector I/O), and interest. If you are interested in making a tool a higher priority for porting, email [John Lindsay](http://www.uoguelph.ca/~hydrogeo/index.html).

### 12.3 Can WhiteboxTools be incorporated into other software and open-source GIS projects?

*WhiteboxTools* was developed with the open-source GIS [Whitebox GAT](http://www.uoguelph.ca/~hydrogeo/Whitebox/index.html) in mind. That said, the tools can be accessed independently and so long as you abide by the terms of the [MIT license](#license), there is no reason why other software and GIS projects cannot use *WhiteboxTools* as well. In fact, this was one of the motivating factors for creating the library in the first place. Feel free to use *WhiteboxTools* as the geospatial analysis engine in your open-source software project.

### 12.4 What platforms does WhiteboxTools support?

*WhiteboxTools* is developed using the Rust programming language, which supports a [wide variety of platforms](https://forge.rust-lang.org/platform-support.html) including MS Windows, MacOS, and Linux operating systems and common chip architectures. Interestingly, Rust also supports mobile platforms, and *WhiteboxTools* should therefore be capable of targeting (although no testing has been completed in this regard to date). Nearly all development and testing of the software is currently carried out on MacOS and we cannot guarantee a bug-free performance on other platforms. In particularly, MS Windows is the most different from the other platforms and is therefore the most likely to encounter platform-specific bugs. If you encounter bugs in the software, please consider reporting an issue using the GitHub support for issue-tracking.

### 12.5 What are the system requirements?

The answer to this question depends strongly on the type of analysis and data that you intend to process. However, generally we find performance to be optimal with a recommended minimum of 8-16GB of memory (RAM), a modern multi-core processor (e.g. 64-bit i5 or i7), and an solid-state-drive (SSD). It is likely that *WhiteboxTools* will have satisfactory performance on lower-spec systems if smaller datasets are being processed. Because *WhiteboxTools* reads entire raster datasets into system memory (for optimal performance, and in recognition that modern systems have increasingly larger amounts of fast RAM), this tends to be the limiting factor for the upper-end of data size successfully processed by the library. 64-bit operating systems are recommended and extensive testing has not been carried out on 32-bit OSs. See [**"What platforms does WhiteboxTools support?"**](#what-platforms-does-whiteboxtools-support) for further details on supported platforms.

### 12.6 Are pre-compiled executables of WhiteboxTools available?

Pre-compiled binaries for *WhiteboxTools* can be downloaded from the [*Geomorphometry and Hydrogeomatics Research Group*](http://www.uoguelph.ca/~hydrogeo/software.shtml#WhiteboxTools) software web site for various supported operating systems. If you need binaries for other operating systems/system architectures, you will need to compile the executable from source files. See [Installation](#installation) for details.

### 12.7 Why is WhiteboxTools programmed in Rust?

I spent a long time evaluating potential programming language for future development efforts for the *Whitebox GAT* project. My most important criterion for a language was that it compile to native code, rather than target the Java virtual machine (JVM). I have been keen to move Whitebox GAT away from Java because of some of the challenges that supporting the JVM has included for many Whitebox users. The language should be fast and productive--Java is already quite fast, but if I am going to change development languages, I would like a performance boost. Furthermore, given that many, though not all, of the algorithms used for geospatial analysis scale well with concurrent (parallel) implementations, I favoured languages that offered easy and safe concurrent programming. Although many would consider C/C++ for this work, I was looking for a modern and safe language. Fortunately, we are living through a renaissance period in programming language development and there are many newer languages that fit the bill nicely. Over the past two years, I considered each of Go, Rust, D, Nim, and Crystal for Whitebox development and ultimately decided on Rust. [See [*GoSpatial*](https://github.com/jblindsay/go-spatial) and [*lidario*](https://github.com/jblindsay/lidario).]

Each of the languages I examined has its own advantages of disadvantages, so why Rust? It's a combination of factors that made it a compelling option for this project. Compared with many on the list, Rust is a mature language with a vibrant user community. Like C/C++, it's a high-performance and low-level language that allows for complete control of the system. However, Rust is also one of the safest languages, meaning that I can be confident that *WhiteboxTools* will not contain common bugs, such as memory use-after-release, memory leaks and race conditions within concurrent code. Importantly, and quite uniquely, this safety is achieved in the Rust language without the use of a garbage collector (automatic memory management). Garbage collectors can be great, but they do generally come with a certain efficiency trade-off that Rust does not have. The other main advantage of Rust's approach to memory management is that it allows for  a level of interaction with scripting languages (e.g. Python) that is quite difficult to do in garbage collected languages. Although **WhiteboxTools** is currently set up to use an automation approach to interacting with Python code that calls it, I like the fact that I have the option to create a *WhiteboxTools* shared library. 

Not everything with Rust is perfect however. It is still a very young language and there are many pieces still missing from its ecosystem. Furthermore, it is not the easiest language to learn, particularly for people who are inexperienced with programming. This may limit my ability to attract other programers to the Whitebox project, which would be unfortunate. However, overall, Rust was the best option for this particular application.

### 12.8 Do I need Rust installed on my computer to run WhiteboxTools?

No, you would only need Rust installed if you were compiling the *WhiteboxTools* codebase from source files.

### 12.9 How does WhiteboxTools' design philosophy differ?

*Whitebox GAT* is frequently praised for its consistent design and ease of use. Like *Whitebox GAT*, *WhiteboxTools* follows the convention of *one tool for one function*. For example, in *WhiteboxTools* assigning the links in a stream channel network their Horton, Strahler, Shreve, or Hack stream ordering numbers requires running separate tools (i.e. *HortonStreamOrder*, *StrahlerStreamOrder*, *ShreveStreamMagnitude*, and *HackStreamOrder*). By contrast, in GRASS GIS\textsuperscript{1} and ArcGIS single tools (i.e. the *r.stream.order* and *Stream Order* tools respectively) can be configured to output different channel ordering schemes. The *WhiteboxTools* design is intended to simplify the user experience and to make it easier to find the right tool for a task. With more specific tool names that are reflective of their specific purposes, users are not as reliant on reading help documentation to identify the tool for the task at hand. Similarly, it is not uncommon for tools in other GIS to have multiple outputs. For example, in GRASS GIS the *r.slope.aspect* tool can be configured to output slope, aspect, profile curvature, plan curvature, and several other common terrain surface derivatives. Based on the *one tool for one function* design approach of *WhiteboxTools*, multiple outputs are indicative that a tool should be split into different, more specific tools. Are you more likely to go to a tool named *r.slope.aspect* or *TangentialCurvature* when you want to create a tangential curvature raster from a DEM? If you're new to the software and are unfamiliar with it, probably the later is more obvious. The *WhiteboxTools* design approach also has the added benefit of simplifying the documentation for tools. The one downside to this design approach, however, is that it results (or will result) in a large number of tools, often with signifcant overlap in function. 

\textsuperscript{1} NOTE: It's not my intent to criticize GRASS GIS, as I deeply respect the work that the GRASS developers have contributed. Rather, I am contrasting the consequences of *WhiteboxTools'* design philosophy to that of other GIS.
