#!/usr/bin/env python
import os
import sys
import whitebox_tools as wbt

def main():
    try:
        # Set the whitebox-tools executable directory
        wb_dir = os.path.dirname(os.path.abspath(__file__)) + "/target/release/"
        wbt.set_whitebox_dir(wb_dir)

        # Sets the working directory. If the working dir is set, you don't
        # need to specify complete file names (with paths) to tools that you run.
        # wbt.set_working_dir("/Users/johnlindsay/Documents/data/Rondeau/")
        # wbt.set_working_dir("/Users/johnlindsay/Documents/data/Rondeau/")
        wbt.set_working_dir("/Users/johnlindsay/Documents/data/GarveyGlenWatershed/RGB_6_529_150502_1733__0_282528_2912.las")

        # Run the tool, specifying the arguments.
        # name = "remove_off_terrain_objects"
        # args = [
        #     "--input=\"StudyData_Rondeau2_NN_filled.dep\"",
        #     # "--input=\"tmp10.dep\"",
        #     "--output=\"tmp13.dep\"",
        #     "--filter=25",
        #     "--slope=15.0"
        # ]

        name = "remove_off_terrain_objects"
        args = [
            "--input=\"StudyData_Rondeau2_NN_filled.dep\"",
            # "--input=\"tmp10.dep\"",
            "--output=\"tmp13.dep\"",
            "--filter=25",
            "--slope=15.0"
        ]

        # Run the tool and check the return value
        if wbt.run_tool(name, args) != 0:
            print("ERROR running {}".format(name))

        # args = [
        #     "--input=\"tmp9.dep\"",
        #     "--output=\"tmp10.dep\"",
        #     "--filter=75",
        #     "--slope=12.0"
        # ]
        #
        # # Run the tool and check the return value
        # if wbt.run_tool(name, args) != 0:
        #     print("ERROR running {}".format(name))

    except:
        print("Unexpected error:", sys.exc_info()[0])
        raise

main()
