#!/usr/bin/env python
''' This script is used to test individual tools in the whitebox-tools library.
It is not intended for general use.
'''

from __future__ import print_function
import os
import sys
# import Tkinter as tk
# import ttk
from whitebox_tools import WhiteboxTools


# class SampleApp(tk.Tk):

#     def __init__(self, *args, **kwargs):
#         tk.Tk.__init__(self, *args, **kwargs)
#         self.wm_title("whitebox-tools")
#         self.button = ttk.Button(text="start", command=self.start)
#         self.button.pack()
#         self.button = ttk.Button(text="cancel", command=self.cancel)
#         self.button.pack()
#         self.progress = ttk.Progressbar(self, orient="horizontal",
#                                         length=200, mode="determinate")
#         self.progress.pack()

#         self.bytes = 0
#         self.maxbytes = 100

#         # setup WhiteboxTools
#         self.wbt = WhiteboxTools()

#     def cancel(self):
#         print("Cancelling...")

#     def start(self):
#         self.wb_dir = os.path.dirname(
#             os.path.abspath(__file__)) + "/target/release/"
#         self.wbt.set_whitebox_dir(self.wb_dir)

#         # needed to specify complete file names (with paths) to tools that you run.
#         # self.wbt.set_working_dir(
#         #     "/Users/johnlindsay/Documents/data/JayStateForest/")
#         # wbt.set_working_dir(
#         #     "/Users/johnlindsay/Documents/data/")
#         self.wbt.set_working_dir(
#             "/Users/johnlindsay/Documents/research/VectorStreamNetworkAnalysis/data/NewBrunswick/")

#         # Tool name and arguments
#         # name = "DevFromMeanElev"
#         name = "RelativeTopographicPosition"
#         # args = ["--input=\"DEM no OTOs.dep\"",
#         #         "--output=\"tmp33.dep\"",
#         #         "--filterx=101",
#         #         "--filtery=101"]

#         args = ["--input=\"alosDEM1_clipped.dep\"",
#                 "--output=\"tmp30.dep\"",
#                 "--filterx=101",
#                 "--filtery=101"]

#         # name = "RemoveOffTerrainObjects"
#         # args = ["--input=\"DEM no OTOs.dep\"",
#         #         "--output=\"tmp34.dep\"",
#         #         "--filter=10",
#         #         "--slope=10.0"]

#         # name = "LidarInfo"
#         # args = ["--input=\"points.las\"",
#         #         "--vlr",
#         #         "--geokeys"]

#         # Print the tool's help
#         # print(wbt.tool_help(name))

#         # Run the tool and check the return value
#         def custom_callback(value):
#             ''' A custom callback for dealing with tool output.
#             '''
#             if "%" in value:
#                 str_array = value.split(" ")
#                 # label = out_str.replace(str_array[len(str_array) - 1], "").strip()
#                 progress = int(
#                     str_array[len(str_array) - 1].replace("%", "").strip())
#                 self.progress["value"] = progress
#             # if "85" in value:
#             #     self.wbt.cancel_op = True

#             print(value)

#         ret = self.wbt.run_tool(name, args, custom_callback)
#         if ret == 1:
#             print("ERROR running {}".format(name))
#         elif ret == 2:
#             print("Operation cancelled while running {}".format(name))

#         # self.progress["value"] = 0
#         # self.maxbytes = 100
#         # self.progress["maximum"] = 100
#         # self.read_bytes()

#     def read_bytes(self):
#         '''simulate reading 500 bytes; update progress bar'''
#         self.bytes += 1
#         self.progress["value"] = self.bytes
#         if self.bytes < self.maxbytes:
#             self.read_bytes()
#         #     # read more bytes after 100 ms
#         #     self.after(100, self.read_bytes)


def main():
    """ main function
    """
    try:
        # name = input("Tool name: ") or 'DevFromMeanElev'
        # input_file = input("Input file: ") or 'DEM no OTOs.dep'
        # output_file = input("Output file: ") or 'tmp.dep'
        # filter_size = int(input("Filter size: ") or "101")

        # setup WhiteboxTools
        wbt = WhiteboxTools()

        wb_dir = os.path.dirname(
            os.path.abspath(__file__)) + "/target/release/"
        wbt.set_whitebox_dir(wb_dir)

        wkdir = "/Users/johnlindsay/Documents/data/GarveyGlenWatershed/"
        # wkdir = "/Users/johnlindsay/Desktop/WhiteboxGAT-mac/resources/samples/Vermont DEM/"
        wbt.set_working_dir(wkdir)

        # Tool name and arguments
        # name = "DevFromMeanElev"
        # name = "ElevsPercentile"
        # name = "RelativeTopographicPosition"

        # filter_size = 101
        # args = ["--input=\"DEM no OTOs.dep\"",
        #         "--output=\"tmp33.dep\"",
        #         "--filterx={0}".format(filter_size),
        #         "--filtery={0}".format(filter_size)]

        # args = ["--input=\"{0}\"".format(input_file),
        #         "--output=\"{0}\"".format(output_file),
        #         "--filterx={0}".format(filter_size),
        #         "--filtery={0}".format(filter_size)]

        # args = ["--input=\"DEM.dep\"",
        #         "--output=\"tmp33.dep\"",
        #         "--filterx=101",
        #         "--filtery=101"]

        # name = "RemoveOffTerrainObjects"
        # args = ["--input=\"DEM no OTOs.dep\"",
        #         "--output=\"tmp34.dep\"",
        #         "--filter=10",
        #         "--slope=10.0"]

        # name = "LidarInfo"
        # args = ["--input=\"points.las\"",
        #         "--vlr",
        #         "--geokeys"]

        # name = "LidarHillshade"
        # args = ["--input=\"RGB_5_529_150502_1754__0_270112_2848.las\"",
        #         "--output=\"hillshade2.las\"",
        #         "--azimuth=315.0",
        #         "--altitude=30.0",
        #         "--radius=2.5"]

        # name = "LidarTophatTransform"
        # args = ["--input=\"RGB_5_529_150502_1754__0_270112_2848.las\"",
        #         "--output=\"filtered12m_RGB_5_529_150502_1754__0_270112_2848.las\"",
        #         "--radius=12.0"]

        def custom_callback(value):
            ''' A custom callback for dealing with tool output.
            '''
            if not hasattr(custom_callback, 'prev_line_progress'):
                custom_callback.prev_line_progress = False
            if not hasattr(custom_callback, 'prev_line_len'):
                custom_callback.prev_line_len = -1
            if "%" in value:
                # wbt.cancel_op = True
                if custom_callback.prev_line_progress:
                    if len(value) < custom_callback.prev_line_len:
                        print('                                   ', end="\r")
                    print('{0}'.format(value), end="\r")
                else:
                    custom_callback.prev_line_progress = True
                    print(value)
            else:
                if custom_callback.prev_line_progress:
                    print('\n{0}'.format(value))
                    custom_callback.prev_line_progress = False
                else:
                    print(value)

            custom_callback.prev_line_len = len(value)

        # file_names = ["RGB_3_529_150514_1353__0_294688_2784.las",
        #               "RGB_3_529_150514_1353__0_276256_2784.las"]

        # filter_size = 18.0

        # for file_name in file_names:
        #     name = "LidarGroundPointFilter"
        #     args = ["--input=\"{0}\"".format(file_name),
        #             "--output=\"filtered{0}m_{1}\"".format(
        #                 str(filter_size).replace(".", "_"), file_name),
        #             "--radius={0}".format(filter_size),
        #             "--otoheight=0.75"]

        #     # Run the tool and check the return value
        #     ret = wbt.run_tool(name, args, custom_callback)
        #     if ret == 1:
        #         print("ERROR running {}".format(name))
        #     elif ret == 2:
        #         print("Operation cancelled while running {}".format(name))

        # name = "FillMissingData"
        # args = ["--input=\"mosaic_FR.dep\"",
        #         "--output=\"tmp1.dep\"",
        #         "--filter=41"]

        # name = "RemoveOffTerrainObjects"
        # args = ["--input=\"tmp1_clipped.dep\"",
        #         "--output=\"mosaic_FR no OTOs4.dep\"",
        #         "--filter=251",
        #         "--slope=10.0"]

        # name = "Hillshade"
        # args = ["--input=\"tmp10.dep\"",
        #         "--output=\"tmp11.dep\"",
        #         "--azimuth=315.0",
        #         "--altitude=20.0"]

        # name = "DInfFlowAccumulation"
        # args = ["--input=\"DEM final.dep\"",
        #         "--output=\"tmp13.dep\"",
        #         "--out_type=\"cells\"",
        #         "--threshold=10000",
        #         "--log"]

        # name = "BufferRaster"
        # args = ["--input=\"tmp11.dep\"",
        #         "--output=\"tmp12.dep\"",
        #         "--size=450",
        #         "--gridcells"]

        # name = "Quantiles"
        # args = ["--input=\"DEM final.dep\"",
        #         "--output=\"tmp10.dep\"",
        #         "--num_quantiles=15"]

        # name = "Clump"
        # args = ["--input=\"tmp10.dep\"",
        #         "--output=\"tmp15.dep\""]

        # name = "Clump"
        # args = ["--input=\"tmp12.dep\"",
        #         "--output=\"tmp16.dep\"",
        #         "--diag",
        #         "--zero_back"]

        name = "DiffFromMeanElev"
        args = ["--input=\"DEM final.dep\"",
                "--output=\"tmp7.dep\"",
                "--filter=250"]

        # Run the tool and check the return value
        ret = wbt.run_tool(name, args, custom_callback)
        if ret == 1:
            print("ERROR running {}".format(name))
        elif ret == 2:
            print("Operation cancelled while running {}".format(name))

        # app = SampleApp()
        # app.mainloop()

    except:
        print("Unexpected error:", sys.exc_info()[0])
        raise


main()
