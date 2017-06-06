#!/usr/bin/env python
''' This modual provides examples of how to call the whitebox_tool script and the
whitebox-tools geospatial analysis library using Python code.
'''
import os
import sys
import whitebox_tools as wbt


def main():
    ''' main function
    '''
    try:
        # Set the whitebox-tools executable directory
        # (change this to point to where you have the whitebox-tools.exe file)
        wb_dir = os.path.dirname(
            os.path.abspath(__file__)) + "/target/release/"
        wbt.set_whitebox_dir(wb_dir)

        # Prints the whitebox-tools help...a listing of available commands
        print(wbt.help())

        # Prints the whitebox-tools license
        print(wbt.license())

        # Prints the whitebox-tools version
        print("Version information: {}".format(wbt.version()))

        # List all available tools in whitebox-tools
        print(wbt.list_tools())

        print(wbt.tool_help("dev_from_mean_elev"))

        # Sets verbose mode (True or False). Most tools will suppress output (e.g. updating
        # progress) when verbose mode is False. The default is True
        # wbt.set_verbose_mode(False)

        # needed to specify complete file names (with paths) to tools that you run.
        wbt.set_working_dir(os.path.dirname(
            os.path.abspath(__file__)) + "/testdata/")

        name = "dev_from_mean_elev"
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
    A provided callback allows for custom processing of the output stream.
    '''
    try:
        if "%" in out_str:
            str_array = out_str.split(" ")
            label = out_str.replace(str_array[len(str_array) - 1], "").strip()
            progress = int(
                str_array[len(str_array) - 1].replace("%", "").strip())
            print("{0} {1}%".format(label, progress))
        elif "error" in out_str.lower():
            print("ERROR: {}".format(out_str))
        elif "elapsed time (excluding i/o):" in out_str.lower():
            elapsed_time = ''.join(
                ele for ele in out_str if ele.isdigit() or ele == '.')
            units = out_str.lower().replace("elapsed time (excluding i/o):",
                                            "").replace(elapsed_time, "").strip()
            print("Elapsed time: {0}{1}".format(elapsed_time, units))
        else:
            print("{}".format(out_str))
    except:
        print(out_str)


main()
