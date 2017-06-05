#!/usr/bin/env python
import os
import sys
import whitebox_tools as wbt


def main():
    try:
        # Tool name and arguments
        #name = "dev_from_mean_elev"
        name = "elev_percentile"
        args = ["--input=\"DEM no OTOs.dep\"",
                "--output=\"tmp32.dep\"",
                "--filterx=101",
                "--filtery=101"]

        wb_dir = os.path.dirname(
            os.path.abspath(__file__)) + "/target/release/"
        wbt.set_whitebox_dir(wb_dir)

        # needed to specify complete file names (with paths) to tools that you run.
        wbt.set_working_dir(
            "/Users/johnlindsay/Documents/data/JayStateForest/")

        # Print the tool's help
        print(wbt.tool_help(name))

        # Run the tool and check the return value
        if wbt.run_tool(name, args) != 0:
            print("ERROR running {}".format(name))

    except:
        print("Unexpected error:", sys.exc_info()[0])
        raise


main()
