#!/usr/bin/env python
''' This script is used to test individual tools in the whitebox-tools library.
'''

from __future__ import print_function
import os
import sys
import whitebox_tools as wbt


def main():
    """ main function
    """
    try:
        # Tool name and arguments
        # name = "DevFromMeanElev"
        # name = "RelativeTopographicPosition"
        # args = ["--input=\"DEM no OTOs.dep\"",
        #         "--output=\"tmp33.dep\"",
        #         "--filterx=101",
        #         "--filtery=101"]

        # name = "RemoveOffTerrainObjects"
        # args = ["--input=\"DEM no OTOs.dep\"",
        #         "--output=\"tmp34.dep\"",
        #         "--filter=10",
        #         "--slope=10.0"]

        name = "LidarInfo"
        args = ["--input=\"points.las\"",
                "--vlr",
                "--geokeys"]

        wb_dir = os.path.dirname(
            os.path.abspath(__file__)) + "/target/release/"
        wbt.set_whitebox_dir(wb_dir)

        # needed to specify complete file names (with paths) to tools that you run.
        # wbt.set_working_dir(
        #     "/Users/johnlindsay/Documents/data/JayStateForest/")
        wbt.set_working_dir(
            "/Users/johnlindsay/Documents/data/")

        # Print the tool's help
        print(wbt.tool_help(name))

        # Run the tool and check the return value
        if wbt.run_tool(name, args) != 0:
            print("ERROR running {}".format(name))

    except:
        print("Unexpected error:", sys.exc_info()[0])
        raise


main()
