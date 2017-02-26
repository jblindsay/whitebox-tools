#!/usr/bin/env python
import os
import sys
import whitebox_tools as wbt

def main():
    try:
        # Set the whitebox-tools executable directory
        wb_dir = os.path.dirname(os.path.abspath(__file__)) + "/target/release/"
        wbt.set_whitebox_dir(wb_dir)

        # Prints the whitebox-tools help...a listing of available commands
        print(wbt.help())

        # Prints the whitebox-tools license
        print(wbt.license())

        # Prints the whitebox-tools version
        print("Version Info: {}".format(wbt.version()))

        # List all available tools in whitebox-tools
        print(wbt.list_tools())

        # Print the help documentation (description, input parameters, example usage) for various tools
        print(wbt.tool_help("lidar_flightline_overlap"))
        print(wbt.tool_help("lidar_info"))
        print(wbt.tool_help("lidar_join"))
        print(wbt.tool_help("remove_off_terrain_objects"))

        # Sets verbose mode (True or False). Most tools will suppress output (e.g. updating
        # progress) when verbose mode is False. The default is True
        # wbt.set_verbose_mode(False)

        # Sets the working directory. If the working dir is set, you don't
        # need to specify complete file names (with paths) to tools that you run.
        wbt.set_working_dir("/Users/johnlindsay/Documents/research/OTOpaper/Take3/data/PEC/Picton data/")

        # Run the remove_off_terrain_objects tool, specifying the arguments.
        name = "remove_off_terrain_objects"
        args = [
            "--input=\"small DEM.dep\"",
            "--output=\"tmp2.dep\"",
            "--filter=49",
            "--slope=15.0"
        ]

        # Run the tool and check the return value
        if wbt.run_tool(name, args, callback) != 0:
            print("ERROR running {}".format(name))

    except:
        print("Unexpected error:", sys.exc_info()[0])
        raise

# Create a custom callback to process the text coming out of the tool.
# If a callback is not provided, it will simply print the output stream.
# A provided callback allows for custom processing of the output stream.
def callback(s):
    try:
        # print("{}".format(s))
        if "%" in s:
            str_array = s.split(" ")
            label = s.replace(str_array[len(str_array)-1], "")
            progress = int(str_array[len(str_array)-1].replace("%", "").strip())
            print("Progress: {}%".format(progress))
        else:
            if "error" in s.lower():
                print("ERROR: {}".format(s))
            else:
                print("{}".format(s))
    except:
        print(s)

main()
