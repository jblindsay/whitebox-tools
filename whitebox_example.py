#!/usr/bin/env python
''' This module provides examples of how to call the whitebox_tool script and the
whitebox-tools geospatial analysis library using Python code.
'''
from __future__ import print_function
import os
import sys
from whitebox_tools import WhiteboxTools


def main():
    ''' main function
    '''
    try:
        # Set the whitebox-tools executable directory
        # (change this to point to where you have the whitebox-tools.exe file)
        wb_dir = os.path.dirname(
            os.path.abspath(__file__)) + "/target/release/"
        wbt = WhiteboxTools()
        wbt.set_whitebox_dir(wb_dir)

        # Prints the whitebox-tools help...a listing of available commands
        print(wbt.help())

        # Prints the whitebox-tools license
        print(wbt.license())

        # Prints the whitebox-tools version
        print("Version information: {}".format(wbt.version()))

        # List all available tools in whitebox-tools
        print(wbt.list_tools())

        # print(wbt.tool_help("dev_from_mean_elev"))
        print(wbt.tool_help("elev_percentile"))

        # Sets verbose mode (True or False). Most tools will suppress output (e.g. updating
        # progress) when verbose mode is False. The default is True
        # wbt.set_verbose_mode(False)

        # needed to specify complete file names (with paths) to tools that you run.
        wbt.set_working_dir(os.path.dirname(
            os.path.abspath(__file__)) + "/testdata/")

        name = "elev_percentile"
        args = ["--input=\"DEM.dep\"",
                "--output=\"DEV_101.dep\"",
                "--filter=101"]

        # Run the tool and check the return value
        if wbt.run_tool(name, args, callback) != 0:
            print("ERROR running {}".format(name))

    except:
        print("Unexpected error:", sys.exc_info()[0])
        raise


def callback(out_str):
    ''' Create a custom callback to process the text coming out of the tool.
    If a callback is not provided, it will simply print the output stream.
    A custom callback allows for processing of the output stream.
    '''
    try:
        if not hasattr(callback, 'prev_line_progress'):
            callback.prev_line_progress = False
        if "%" in out_str:
            str_array = out_str.split(" ")
            label = out_str.replace(str_array[len(str_array) - 1], "").strip()
            progress = int(
                str_array[len(str_array) - 1].replace("%", "").strip())
            if callback.prev_line_progress:
                print('{0} {1}%'.format(label, progress), end="\r")
            else:
                callback.prev_line_progress = True
                print(out_str)
        elif "error" in out_str.lower():
            print("ERROR: {}".format(out_str))
            callback.prev_line_progress = False
        elif "elapsed time (excluding i/o):" in out_str.lower():
            elapsed_time = ''.join(
                ele for ele in out_str if ele.isdigit() or ele == '.')
            units = out_str.lower().replace("elapsed time (excluding i/o):",
                                            "").replace(elapsed_time, "").strip()
            print("Elapsed time: {0}{1}".format(elapsed_time, units))
            callback.prev_line_progress = False
        else:
            if callback.prev_line_progress:
                print('\n{0}'.format(out_str))
                callback.prev_line_progress = False
            else:
                print(out_str)

    except:
        print(out_str)


main()
