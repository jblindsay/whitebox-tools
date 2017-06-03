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
        print("Version information: {}".format(wbt.version()))

        # List all available tools in whitebox-tools
        print(wbt.list_tools())

        # Print the help documentation (description, input parameters, example usage) for various tools
        print(wbt.tool_help("lidar_elevation_slice"))
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
        # wbt.set_working_dir("/Users/johnlindsay/Documents/data/GarveyGlenWatershed/")
        # name = "lidar_ground_point_separation"
        # Run the remove_off_terrain_objects tool, specifying the arguments.
        # name = "remove_off_terrain_objects"
        # args = [
        #     "--input=\"small DEM.dep\"",
        #     "--output=\"tmp2.dep\"",
        #     "--filter=49",
        #     "--slope=15.0"
        # ]

        name = "lidar_elevation_slice"
        args = [
            "--input=\"1km183270487302008GROUPEALTA.las\"",
            "--output=\"deleteme.las\"",
            "--minz=90.0",
            "--maxz=120.0",
            "--inclassval=1",
            "--outclassval=0"
        ]

        # args = [
        #     "-i=\"RGB_5_529_150502_1754__0_282400_2848.las\"",
        #     "-o=\"out3.las\"",
        #     "-dist=5.0",
        #     "-slope=12.0",
        #     "-minz=0.0"
        # ]

        # cmd = "." + os.path.sep  + "lidar_elevation_slice"
        # argslist = [
        #     cmd,
        #     '-wd', # working directory
        #     # '/Users/johnlindsay/Documents/research/OTOpaper/Take3/data/PEC/Picton data/',
        #     # "/Users/johnlindsay/Documents/data/Rondeau/",
        #     # "/Users/johnlindsay/Documents/data/GullyCreek/LiDAR/1_LiDAR_OMAFRA_PointCloud/LAS_tiles_25m/",
        #     "/Users/johnlindsay/Documents/teaching/GEOG3420/W17/Labs/Lab2/NewLab/data/",
        #     '-i', # input file
        #     # 'StudyData_EAG.las',
        #     # "448000_4827000.las",
        #     # "447000_4828000.las",
        #     # "446000_4829000.las",
        #     "1km183270487302008GROUPEALTA.las",
        #     '-o', # output file,
        #     'test_tile4.las',
        #     '-minz', # minimum elevation
        #     '75.0',
        #     '-maxz', # maximum elevaiton
        #     '155.0',
        #     '-v' # verbose mode; progress will be updated to output stream
        # ]

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
