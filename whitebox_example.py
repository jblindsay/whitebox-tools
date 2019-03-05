#!/usr/bin/env python3
''' This module provides examples of how to call the whitebox_tool script and the
whitebox-tools geospatial analysis library using Python code.
'''

# This script is part of the WhiteboxTools geospatial library.
# Authors: Dr. John Lindsay
# Created: November 28, 2017
# Last Modified: Feb. 17, 2018
# License: MIT

from __future__ import print_function
import os
import sys
from whitebox_tools import WhiteboxTools
import urllib.request


def main():
    ''' main function
    '''
    try:
        wbt = WhiteboxTools()

        # Get the root directory of WhiteboxTools source code or executable file
        root_dir = os.path.dirname(os.path.abspath(__file__))
        # WhiteboxTools executable file name for MS Windows
        wbt_win_bin = os.path.join(root_dir, "whitebox_tools.exe")
        # WhiteboxTools executable file name for MacOS/Linux
        wbt_linux_bin = os.path.join(root_dir, "whitebox_tools")

        # If the WhiteboxTools executable file (whitbox_tools.exe) is in the same
        # directory as this script, set wbt path to the current directory
        # otherwise, set wbt path to (root_dir + "/target/release/")
        if os.path.isfile(wbt_win_bin) or os.path.isfile(wbt_linux_bin):
            wbt.set_whitebox_dir(root_dir)
        else:
            wbt.set_whitebox_dir(root_dir + "/target/release/")  # or simply wbt.exe_path = ...

        # Set the working directory. This is the path to the folder containing the data,
        # i.e. files sent to tools as input/output parameters. You don't need to set
        # the working directory if you specify full path names as tool parameters.
        wbt.work_dir = os.path.dirname(
            os.path.abspath(__file__)) + "/testdata/"

        # If test datasets do not exist, download them from the WhiteboxTools repo
        if not os.path.exists(wbt.work_dir):
            os.mkdir(wbt.work_dir)
            dem_url = "https://github.com/jblindsay/whitebox-tools/raw/master/testdata/DEM.tif"
            dep_url = "https://github.com/jblindsay/whitebox-tools/raw/master/testdata/DEM.dep"
            urllib.request.urlretrieve(dem_url, "testdata/DEM.tif")
            urllib.request.urlretrieve(dep_url, "testdata/DEM.dep")

        # Sets verbose mode (True or False). Most tools will suppress output (e.g. updating
        # progress) when verbose mode is False. The default is True
        # wbt.set_verbose_mode(False) # or simply, wbt.verbose = False

        # The most convenient way to run a tool is to use its associated method, e.g.:
        if wbt.elev_percentile("DEM.tif", "output.tif", 15, 15) != 0:
            print("ERROR running tool")

        # You may also provide an optional custom callback for processing output from the
        # tool. If you don't provide a callback, and verbose is set to True, tool output
        # will simply be printed to the standard output. Also, notice that each tool has a
        # convenience method. While internally, whitebox_tools.exe uses CamelCase (MeanFilter)
        # to denote tool names, but the Python interface of whitebox_tools.py uses
        # snake_case (mean_filter), according to Python style conventions.

        # All of the convenience methods just call the 'run_tool' method, feeding it an
        # args array. This is an alternative way of calling tools:
        tool_name = "elev_percentile"
        args = ["--dem=\"DEM.dep\"",
                "--output=\"DEV_101.dep\"",
                "--filterx=101"]

        if wbt.run_tool(tool_name, args, my_callback) != 0:
            print("ERROR running {}".format(tool_name))

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

    except:
        print("Unexpected error:", sys.exc_info()[0])
        raise


def my_callback(out_str):
    ''' Create a custom callback to process the text coming out of the tool.
    If a callback is not provided, it will simply print the output stream.
    A custom callback allows for processing of the output stream.
    '''
    try:
        if not hasattr(my_callback, 'prev_line_progress'):
            my_callback.prev_line_progress = False
        if "%" in out_str:
            str_array = out_str.split(" ")
            label = out_str.replace(str_array[len(str_array) - 1], "").strip()
            progress = int(
                str_array[len(str_array) - 1].replace("%", "").strip())
            if my_callback.prev_line_progress:
                print('{0} {1}%'.format(label, progress), end="\r")
            else:
                my_callback.prev_line_progress = True
                print(out_str)
        elif "error" in out_str.lower():
            print("ERROR: {}".format(out_str))
            my_callback.prev_line_progress = False
        elif "elapsed time (excluding i/o):" in out_str.lower():
            elapsed_time = ''.join(
                ele for ele in out_str if ele.isdigit() or ele == '.')
            units = out_str.lower().replace("elapsed time (excluding i/o):",
                                            "").replace(elapsed_time, "").strip()
            print("Elapsed time: {0}{1}".format(elapsed_time, units))
            my_callback.prev_line_progress = False
        else:
            if callback.prev_line_progress:
                print('\n{0}'.format(out_str))
                my_callback.prev_line_progress = False
            else:
                print(out_str)

    except:
        print(out_str)


main()
