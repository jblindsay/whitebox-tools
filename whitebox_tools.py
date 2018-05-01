#!/usr/bin/env python
''' This file is intended to be a helper for running whitebox-tools plugins from a Python script.
See whitebox_example.py for an example of how to use it.
'''

# This script is part of the WhiteboxTools geospatial library.
# Authors: Dr. John Lindsay
# Created: 28/11/2017
# Last Modified: 22/04/2018
# License: MIT

from __future__ import print_function
import os
from os import path
import sys
from sys import platform
from subprocess import CalledProcessError, Popen, PIPE, STDOUT


def default_callback(value):
    ''' 
    A simple default callback that outputs using the print function. When
    tools are called without providing a custom callback, this function
    will be used to print to standard output.
    '''
    print(value)


class WhiteboxTools(object):
    ''' 
    An object for interfacing with the WhiteboxTools executable.
    '''

    def __init__(self):
        self.exe_path = path.dirname(path.abspath(__file__))
        self.work_dir = ""
        self.verbose = True
        self.cancel_op = False

    if platform == 'win32':
        ext = '.exe'
    else:
        ext = ''

    exe_name = "whitebox_tools{}".format(ext)

    def set_whitebox_dir(self, path_str):
        ''' 
        Sets the directory to the WhiteboxTools executable file.
        '''
        self.exe_path = path_str

    def set_working_dir(self, path_str):
        ''' 
        Sets the working directory, i.e. the directory in which
        the data files are located. By setting the working 
        directory, tool input parameters that are files need only
        specify the file name rather than the complete file path.
        '''
        self.work_dir = path.normpath(path_str)

    def set_verbose_mode(self, val=True):
        ''' 
        Sets verbose mode. If verbose mode is False, tools will not
        print output messages. Tools will frequently provide substantial
        feedback while they are operating, e.g. updating progress for 
        various sub-routines. When the user has scripted a workflow
        that ties many tools in sequence, this level of tool output
        can be problematic. By setting verbose mode to False, these
        messages are suppressed and tools run as background processes.
        '''
        self.verbose = val

    def run_tool(self, tool_name, args, callback=default_callback):
        ''' 
        Runs a tool and specifies tool arguments.
        Returns 0 if completes without error.
        Returns 1 if error encountered (details are sent to callback).
        Returns 2 if process is cancelled by user.
        '''
        try:
            os.chdir(self.exe_path)
            args2 = []
            args2.append("." + path.sep + self.exe_name)
            args2.append("--run=\"{}\"".format(tool_name))

            if self.work_dir.strip() != "":
                args2.append("--wd=\"{}\"".format(self.work_dir))

            for arg in args:
                args2.append(arg)

            # args_str = args_str[:-1]
            # a.append("--args=\"{}\"".format(args_str))

            if self.verbose:
                args2.append("-v")

            if self.verbose:
                cl = ""
                for v in args2:
                    cl += v + " "
                callback(cl.strip() + "\n")

            proc = Popen(args2, shell=False, stdout=PIPE,
                         stderr=STDOUT, bufsize=1, universal_newlines=True)

            while True:
                line = proc.stdout.readline()
                sys.stdout.flush()
                if line != '':
                    if not self.cancel_op:
                        callback(line.strip())
                    else:
                        self.cancel_op = False
                        proc.terminate()
                        return 2

                else:
                    break

            return 0
        except (OSError, ValueError, CalledProcessError) as err:
            callback(str(err))
            return 1

    def help(self):
        ''' 
        Retrieves the help description for WhiteboxTools.
        '''
        try:
            os.chdir(self.exe_path)
            args = []
            args.append("." + os.path.sep + self.exe_name)
            args.append("-h")

            proc = Popen(args, shell=False, stdout=PIPE,
                         stderr=STDOUT, bufsize=1, universal_newlines=True)
            ret = ""
            while True:
                line = proc.stdout.readline()
                if line != '':
                    ret += line
                else:
                    break

            return ret
        except (OSError, ValueError, CalledProcessError) as err:
            return err

    def license(self):
        ''' 
        Retrieves the license information for WhiteboxTools.
        '''
        try:
            os.chdir(self.exe_path)
            args = []
            args.append("." + os.path.sep + self.exe_name)
            args.append("--license")

            proc = Popen(args, shell=False, stdout=PIPE,
                         stderr=STDOUT, bufsize=1, universal_newlines=True)
            ret = ""
            while True:
                line = proc.stdout.readline()
                if line != '':
                    ret += line
                else:
                    break

            return ret
        except (OSError, ValueError, CalledProcessError) as err:
            return err

    def version(self):
        ''' 
        Retrieves the version information for WhiteboxTools.
        '''
        try:
            os.chdir(self.exe_path)
            args = []
            args.append("." + os.path.sep + self.exe_name)
            args.append("--version")

            proc = Popen(args, shell=False, stdout=PIPE,
                         stderr=STDOUT, bufsize=1, universal_newlines=True)
            ret = ""
            while True:
                line = proc.stdout.readline()
                if line != '':
                    ret += line
                else:
                    break

            return ret
        except (OSError, ValueError, CalledProcessError) as err:
            return err

    def tool_help(self, tool_name=''):
        ''' 
        Retrieves the help description for a specific tool.
        '''
        try:
            os.chdir(self.exe_path)
            args = []
            args.append("." + os.path.sep + self.exe_name)
            args.append("--toolhelp={}".format(tool_name))

            proc = Popen(args, shell=False, stdout=PIPE,
                         stderr=STDOUT, bufsize=1, universal_newlines=True)
            ret = ""
            while True:
                line = proc.stdout.readline()
                if line != '':
                    ret += line
                else:
                    break

            return ret
        except (OSError, ValueError, CalledProcessError) as err:
            return err

    def tool_parameters(self, tool_name):
        ''' 
        Retrieves the tool parameter descriptions for a specific tool.
        '''
        try:
            os.chdir(self.exe_path)
            args = []
            args.append("." + os.path.sep + self.exe_name)
            args.append("--toolparameters={}".format(tool_name))

            proc = Popen(args, shell=False, stdout=PIPE,
                         stderr=STDOUT, bufsize=1, universal_newlines=True)
            ret = ""
            while True:
                line = proc.stdout.readline()
                if line != '':
                    ret += line
                else:
                    break

            return ret
        except (OSError, ValueError, CalledProcessError) as err:
            return err

    def toolbox(self, tool_name=''):
        ''' 
        Retrieve the toolbox for a specific tool.
        '''
        try:
            os.chdir(self.exe_path)
            args = []
            args.append("." + os.path.sep + self.exe_name)
            args.append("--toolbox={}".format(tool_name))

            proc = Popen(args, shell=False, stdout=PIPE,
                         stderr=STDOUT, bufsize=1, universal_newlines=True)
            ret = ""
            while True:
                line = proc.stdout.readline()
                if line != '':
                    ret += line
                else:
                    break

            return ret
        except (OSError, ValueError, CalledProcessError) as err:
            return err

    def view_code(self, tool_name):
        ''' 
        Opens a web browser to view the source code for a specific tool
        on the projects source code repository.
        '''
        try:
            os.chdir(self.exe_path)
            args = []
            args.append("." + os.path.sep + self.exe_name)
            args.append("--viewcode={}".format(tool_name))

            proc = Popen(args, shell=False, stdout=PIPE,
                         stderr=STDOUT, bufsize=1, universal_newlines=True)
            ret = ""
            while True:
                line = proc.stdout.readline()
                if line != '':
                    ret += line
                else:
                    break

            return ret
        except (OSError, ValueError, CalledProcessError) as err:
            return err

    def list_tools(self, keywords=[]):
        ''' 
        Lists all available tools in WhiteboxTools.
        '''
        try:
            os.chdir(self.exe_path)
            args = []
            args.append("." + os.path.sep + self.exe_name)
            args.append("--listtools")
            if len(keywords) > 0:
                for kw in keywords:
                    args.append(kw)

            proc = Popen(args, shell=False, stdout=PIPE,
                         stderr=STDOUT, bufsize=1, universal_newlines=True)
            ret = ""
            while True:
                line = proc.stdout.readline()
                if line != '':
                    ret += line
                else:
                    break

            return ret
        except (OSError, ValueError, CalledProcessError) as err:
            return err

    ########################################################################
    # The following methods are convenience methods for each available tool.
    # This needs updating whenever new tools are added to the WhiteboxTools
    # library. They can be generated automatically using the
    # whitebox_plugin_generator.py script. It would also be possible to
    # discover plugins at runtime and monkey-patch their methods using
    # MethodType. However, this would not be as useful since it would
    # restrict the ability for text editors and IDEs to use autocomplete.
    ########################################################################

    
    
    
    
    ##############
    # Data Tools #
    ##############

    def convert_nodata_to_zero(self, i, output, callback=default_callback):
        """ Converts nodata values in a raster to zero.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('ConvertNodataToZero', args, callback) # returns 1 if error

    def convert_raster_format(self, i, output, callback=default_callback):
        """ Converts raster data from one format to another.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('ConvertRasterFormat', args, callback) # returns 1 if error

    def export_table_to_csv(self, i, output, headers=True, callback=default_callback):
        """ Exports an attribute table to a CSV text file.

        Keyword arguments:

        i -- Input vector file. 
        output -- Output raster file. 
        headers -- Export field names as file header?. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if headers: args.append("--headers")
        return self.run_tool('ExportTableToCsv', args, callback) # returns 1 if error

    def new_raster_from_base(self, base, output, value="nodata", data_type="float", callback=default_callback):
        """ Creates a new raster using a base image.

        Keyword arguments:

        base -- Input base raster file. 
        output -- Output raster file. 
        value -- Constant value to fill raster with; either 'nodata' or numeric value. 
        data_type -- Output raster data type; options include 'double' (64-bit), 'float' (32-bit), and 'integer' (signed 16-bit) (default is 'float'). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--base='{}'".format(base))
        args.append("--output='{}'".format(output))
        args.append("--value={}".format(value))
        args.append("--data_type={}".format(data_type))
        return self.run_tool('NewRasterFromBase', args, callback) # returns 1 if error

    def print_geo_tiff_tags(self, i, callback=default_callback):
        """ Prints the tags within a GeoTIFF.

        Keyword arguments:

        i -- Input GeoTIFF file. 
callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        return self.run_tool('PrintGeoTiffTags', args, callback) # returns 1 if error

    def set_nodata_value(self, i, output, back_value=0.0, callback=default_callback):
        """ Assign a specified value in an input image to the NoData value.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        back_value -- Background value to set to nodata. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--back_value={}".format(back_value))
        return self.run_tool('SetNodataValue', args, callback) # returns 1 if error

    def vector_lines_to_raster(self, i, output, field="FID", nodata=True, cell_size=None, base=None, callback=default_callback):
        """ Converts a vector containing polylines into a raster.

        Keyword arguments:

        i -- Input vector lines file. 
        field -- Input field name in attribute table. 
        output -- Output raster file. 
        nodata -- Background value to set to NoData. Without this flag, it will be set to 0.0. 
        cell_size -- Optionally specified cell size of output raster. Not used when base raster is specified. 
        base -- Optionally specified input base raster file. Not used when a cell size is specified. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--field={}".format(field))
        args.append("--output='{}'".format(output))
        if nodata: args.append("--nodata")
        if cell_size is not None: args.append("--cell_size='{}'".format(cell_size))
        if base is not None: args.append("--base='{}'".format(base))
        return self.run_tool('VectorLinesToRaster', args, callback) # returns 1 if error

    def vector_points_to_raster(self, i, output, field="FID", assign="last", nodata=True, cell_size=None, base=None, callback=default_callback):
        """ Converts a vector containing points into a raster.

        Keyword arguments:

        i -- Input vector Points file. 
        field -- Input field name in attribute table. 
        output -- Output raster file. 
        assign -- Assignment operation, where multiple points are in the same grid cell; options include 'first', 'last' (default), 'min', 'max', 'sum'. 
        nodata -- Background value to set to NoData. Without this flag, it will be set to 0.0. 
        cell_size -- Optionally specified cell size of output raster. Not used when base raster is specified. 
        base -- Optionally specified input base raster file. Not used when a cell size is specified. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--field={}".format(field))
        args.append("--output='{}'".format(output))
        args.append("--assign={}".format(assign))
        if nodata: args.append("--nodata")
        if cell_size is not None: args.append("--cell_size='{}'".format(cell_size))
        if base is not None: args.append("--base='{}'".format(base))
        return self.run_tool('VectorPointsToRaster', args, callback) # returns 1 if error

    def vector_polygons_to_raster(self, i, output, field="FID", nodata=True, cell_size=None, base=None, callback=default_callback):
        """ Converts a vector containing polygons into a raster.

        Keyword arguments:

        i -- Input vector polygons file. 
        field -- Input field name in attribute table. 
        output -- Output raster file. 
        nodata -- Background value to set to NoData. Without this flag, it will be set to 0.0. 
        cell_size -- Optionally specified cell size of output raster. Not used when base raster is specified. 
        base -- Optionally specified input base raster file. Not used when a cell size is specified. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--field={}".format(field))
        args.append("--output='{}'".format(output))
        if nodata: args.append("--nodata")
        if cell_size is not None: args.append("--cell_size='{}'".format(cell_size))
        if base is not None: args.append("--base='{}'".format(base))
        return self.run_tool('VectorPolygonsToRaster', args, callback) # returns 1 if error

    ################
    # GIS Analysis #
    ################

    def aggregate_raster(self, i, output, agg_factor=2, type="mean", callback=default_callback):
        """ Aggregates a raster to a lower resolution.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        agg_factor -- Aggregation factor, in pixels. 
        type -- Statistic used to fill output pixels. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--agg_factor={}".format(agg_factor))
        args.append("--type={}".format(type))
        return self.run_tool('AggregateRaster', args, callback) # returns 1 if error

    def centroid(self, i, output, text_output=False, callback=default_callback):
        """ Calculates the centroid, or average location, of raster polygon objects.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        text_output -- Optional text output. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if text_output: args.append("--text_output")
        return self.run_tool('Centroid', args, callback) # returns 1 if error

    def clump(self, i, output, diag=True, zero_back=False, callback=default_callback):
        """ Groups cells that form physically discrete areas, assigning them unique identifiers.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        diag -- Flag indicating whether diagonal connections should be considered. 
        zero_back -- Flag indicating whether zero values should be treated as a background. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if diag: args.append("--diag")
        if zero_back: args.append("--zero_back")
        return self.run_tool('Clump', args, callback) # returns 1 if error

    def create_plane(self, base, output, gradient=15.0, aspect=90.0, constant=0.0, callback=default_callback):
        """ Creates a raster image based on the equation for a simple plane.

        Keyword arguments:

        base -- Input base raster file. 
        output -- Output raster file. 
        gradient -- Slope gradient in degrees (-85.0 to 85.0). 
        aspect -- Aspect (direction) in degrees clockwise from north (0.0-360.0). 
        constant -- Constant value. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--base='{}'".format(base))
        args.append("--output='{}'".format(output))
        args.append("--gradient={}".format(gradient))
        args.append("--aspect={}".format(aspect))
        args.append("--constant={}".format(constant))
        return self.run_tool('CreatePlane', args, callback) # returns 1 if error

    def raster_cell_assignment(self, i, output, assign="column", callback=default_callback):
        """ Assign row or column number to cells.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        assign -- Which variable would you like to assign to grid cells? Options include 'column', 'row', 'x', and 'y'. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--assign={}".format(assign))
        return self.run_tool('RasterCellAssignment', args, callback) # returns 1 if error

    def reclass(self, i, output, reclass_vals, assign_mode=False, callback=default_callback):
        """ Reclassifies the values in a raster image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        reclass_vals -- Reclassification triplet values (new value; from value; to less than), e.g. '0.0;0.0;1.0;1.0;1.0;2.0'. 
        assign_mode -- Optional Boolean flag indicating whether to operate in assign mode, reclass_vals values are interpreted as new value; old value pairs. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--reclass_vals='{}'".format(reclass_vals))
        if assign_mode: args.append("--assign_mode")
        return self.run_tool('Reclass', args, callback) # returns 1 if error

    def reclass_equal_interval(self, i, output, interval=10.0, start_val=None, end_val=None, callback=default_callback):
        """ Reclassifies the values in a raster image based on equal-ranges.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        interval -- Class interval size. 
        start_val -- Optional starting value (default is input minimum value). 
        end_val -- Optional ending value (default is input maximum value). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--interval={}".format(interval))
        if start_val is not None: args.append("--start_val='{}'".format(start_val))
        if end_val is not None: args.append("--end_val='{}'".format(end_val))
        return self.run_tool('ReclassEqualInterval', args, callback) # returns 1 if error

    def reclass_from_file(self, i, reclass_file, output, callback=default_callback):
        """ Reclassifies the values in a raster image using reclass ranges in a text file.

        Keyword arguments:

        i -- Input raster file. 
        reclass_file -- Input text file containing reclass ranges. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--reclass_file='{}'".format(reclass_file))
        args.append("--output='{}'".format(output))
        return self.run_tool('ReclassFromFile', args, callback) # returns 1 if error

    ###############################
    # GIS Analysis/Distance Tools #
    ###############################

    def buffer_raster(self, i, output, size, gridcells=False, callback=default_callback):
        """ Maps a distance-based buffer around each non-background (non-zero/non-nodata) grid cell in an input image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        size -- Buffer size. 
        gridcells -- Optional flag to indicate that the 'size' threshold should be measured in grid cells instead of the default map units. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--size='{}'".format(size))
        if gridcells: args.append("--gridcells")
        return self.run_tool('BufferRaster', args, callback) # returns 1 if error

    def cost_allocation(self, source, backlink, output, callback=default_callback):
        """ Identifies the source cell to which each grid cell is connected by a least-cost pathway in a cost-distance analysis.

        Keyword arguments:

        source -- Input source raster file. 
        backlink -- Input backlink raster file generated by the cost-distance tool. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--source='{}'".format(source))
        args.append("--backlink='{}'".format(backlink))
        args.append("--output='{}'".format(output))
        return self.run_tool('CostAllocation', args, callback) # returns 1 if error

    def cost_distance(self, source, cost, out_accum, out_backlink, callback=default_callback):
        """ Performs cost-distance accumulation on a cost surface and a group of source cells.

        Keyword arguments:

        source -- Input source raster file. 
        cost -- Input cost (friction) raster file. 
        out_accum -- Output cost accumulation raster file. 
        out_backlink -- Output backlink raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--source='{}'".format(source))
        args.append("--cost='{}'".format(cost))
        args.append("--out_accum='{}'".format(out_accum))
        args.append("--out_backlink='{}'".format(out_backlink))
        return self.run_tool('CostDistance', args, callback) # returns 1 if error

    def cost_pathway(self, destination, backlink, output, zero_background=False, callback=default_callback):
        """ Performs cost-distance pathway analysis using a series of destination grid cells.

        Keyword arguments:

        destination -- Input destination raster file. 
        backlink -- Input backlink raster file generated by the cost-distance tool. 
        output -- Output cost pathway raster file. 
        zero_background -- Flag indicating whether zero values should be treated as a background. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--destination='{}'".format(destination))
        args.append("--backlink='{}'".format(backlink))
        args.append("--output='{}'".format(output))
        if zero_background: args.append("--zero_background")
        return self.run_tool('CostPathway', args, callback) # returns 1 if error

    def euclidean_allocation(self, i, output, callback=default_callback):
        """ Assigns grid cells in the output raster the value of the nearest target cell in the input image, measured by the Shih and Wu (2004) Euclidean distance transform.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('EuclideanAllocation', args, callback) # returns 1 if error

    def euclidean_distance(self, i, output, callback=default_callback):
        """ Calculates the Shih and Wu (2004) Euclidean distance transform.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('EuclideanDistance', args, callback) # returns 1 if error

    ##############################
    # GIS Analysis/Overlay Tools #
    ##############################

    def average_overlay(self, inputs, output, callback=default_callback):
        """ Calculates the average for each grid cell from a group of raster images.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('AverageOverlay', args, callback) # returns 1 if error

    def clip_raster_to_polygon(self, i, polygons, output, maintain_dimensions=False, callback=default_callback):
        """ Clips a raster to a vector polygon.

        Keyword arguments:

        i -- Input raster file. 
        polygons -- Input vector polygons file. 
        output -- Output raster file. 
        maintain_dimensions -- Maintain input raster dimensions?. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--polygons='{}'".format(polygons))
        args.append("--output='{}'".format(output))
        if maintain_dimensions: args.append("--maintain_dimensions")
        return self.run_tool('ClipRasterToPolygon', args, callback) # returns 1 if error

    def count_if(self, inputs, output, value, callback=default_callback):
        """ Counts the number of occurrences of a specified value in a cell-stack of rasters.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        value -- Search value (e.g. countif value = 5.0). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        args.append("--value='{}'".format(value))
        return self.run_tool('CountIf', args, callback) # returns 1 if error

    def erase_polygon_from_raster(self, i, polygons, output, callback=default_callback):
        """ Erases (cuts out) a vector polygon from a raster.

        Keyword arguments:

        i -- Input raster file. 
        polygons -- Input vector polygons file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--polygons='{}'".format(polygons))
        args.append("--output='{}'".format(output))
        return self.run_tool('ErasePolygonFromRaster', args, callback) # returns 1 if error

    def highest_position(self, inputs, output, callback=default_callback):
        """ Identifies the stack position of the maximum value within a raster stack on a cell-by-cell basis.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('HighestPosition', args, callback) # returns 1 if error

    def lowest_position(self, inputs, output, callback=default_callback):
        """ Identifies the stack position of the minimum value within a raster stack on a cell-by-cell basis.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('LowestPosition', args, callback) # returns 1 if error

    def max_absolute_overlay(self, inputs, output, callback=default_callback):
        """ Evaluates the maximum absolute value for each grid cell from a stack of input rasters.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('MaxAbsoluteOverlay', args, callback) # returns 1 if error

    def max_overlay(self, inputs, output, callback=default_callback):
        """ Evaluates the maximum value for each grid cell from a stack of input rasters.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('MaxOverlay', args, callback) # returns 1 if error

    def min_absolute_overlay(self, inputs, output, callback=default_callback):
        """ Evaluates the minimum absolute value for each grid cell from a stack of input rasters.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('MinAbsoluteOverlay', args, callback) # returns 1 if error

    def min_overlay(self, inputs, output, callback=default_callback):
        """ Evaluates the minimum value for each grid cell from a stack of input rasters.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('MinOverlay', args, callback) # returns 1 if error

    def percent_equal_to(self, inputs, comparison, output, callback=default_callback):
        """ Calculates the percentage of a raster stack that have cell values equal to an input on a cell-by-cell basis.

        Keyword arguments:

        inputs -- Input raster files. 
        comparison -- Input comparison raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--comparison='{}'".format(comparison))
        args.append("--output='{}'".format(output))
        return self.run_tool('PercentEqualTo', args, callback) # returns 1 if error

    def percent_greater_than(self, inputs, comparison, output, callback=default_callback):
        """ Calculates the percentage of a raster stack that have cell values greather than an input on a cell-by-cell basis.

        Keyword arguments:

        inputs -- Input raster files. 
        comparison -- Input comparison raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--comparison='{}'".format(comparison))
        args.append("--output='{}'".format(output))
        return self.run_tool('PercentGreaterThan', args, callback) # returns 1 if error

    def percent_less_than(self, inputs, comparison, output, callback=default_callback):
        """ Calculates the percentage of a raster stack that have cell values less than an input on a cell-by-cell basis.

        Keyword arguments:

        inputs -- Input raster files. 
        comparison -- Input comparison raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--comparison='{}'".format(comparison))
        args.append("--output='{}'".format(output))
        return self.run_tool('PercentLessThan', args, callback) # returns 1 if error

    def pick_from_list(self, inputs, pos_input, output, callback=default_callback):
        """ Outputs the value from a raster stack specified by a position raster.

        Keyword arguments:

        inputs -- Input raster files. 
        pos_input -- Input position raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--pos_input='{}'".format(pos_input))
        args.append("--output='{}'".format(output))
        return self.run_tool('PickFromList', args, callback) # returns 1 if error

    def weighted_sum(self, inputs, output, weights, callback=default_callback):
        """ Performs a weighted-sum overlay on multiple input raster images.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        weights -- Weight values, contained in quotes and separated by commas or semicolons. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        args.append("--weights='{}'".format(weights))
        return self.run_tool('WeightedSum', args, callback) # returns 1 if error

    ##################################
    # GIS Analysis/Patch Shape Tools #
    ##################################

    def edge_proportion(self, i, output, output_text=False, callback=default_callback):
        """ Calculate the proportion of cells in a raster polygon that are edge cells.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        output_text -- flag indicating whether a text report should also be output. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if output_text: args.append("--output_text")
        return self.run_tool('EdgeProportion', args, callback) # returns 1 if error

    def find_patch_or_class_edge_cells(self, i, output, callback=default_callback):
        """ Finds all cells located on the edge of patch or class features.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('FindPatchOrClassEdgeCells', args, callback) # returns 1 if error

    def radius_of_gyration(self, i, output, text_output=False, callback=default_callback):
        """ Calculates the distance of cells from their polygon's centroid.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        text_output -- Optional text output. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if text_output: args.append("--text_output")
        return self.run_tool('RadiusOfGyration', args, callback) # returns 1 if error

    ############################
    # Geomorphometric Analysis #
    ############################

    def aspect(self, dem, output, zfactor=1.0, callback=default_callback):
        """ Calculates an aspect raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('Aspect', args, callback) # returns 1 if error

    def dev_from_mean_elev(self, dem, output, filterx=11, filtery=11, callback=default_callback):
        """ Calculates deviation from mean elevation.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('DevFromMeanElev', args, callback) # returns 1 if error

    def diff_from_mean_elev(self, dem, output, filterx=11, filtery=11, callback=default_callback):
        """ Calculates difference from mean elevation (equivalent to a high-pass filter).

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('DiffFromMeanElev', args, callback) # returns 1 if error

    def directional_relief(self, dem, output, azimuth=0.0, max_dist=None, callback=default_callback):
        """ Calculates relief for cells in an input DEM for a specified direction.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        azimuth -- Wind azimuth in degrees. 
        max_dist -- Optional maximum search distance (unspecified if none; in xy units). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--azimuth={}".format(azimuth))
        if max_dist is not None: args.append("--max_dist='{}'".format(max_dist))
        return self.run_tool('DirectionalRelief', args, callback) # returns 1 if error

    def downslope_index(self, dem, output, drop=2.0, out_type="tangent", callback=default_callback):
        """ Calculates the Hjerdt et al. (2004) downslope index.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        drop -- Vertical drop value (default is 2.0). 
        out_type -- Output type, options include 'tangent', 'degrees', 'radians', 'distance' (default is 'tangent'). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--drop={}".format(drop))
        args.append("--out_type={}".format(out_type))
        return self.run_tool('DownslopeIndex', args, callback) # returns 1 if error

    def elev_above_pit(self, dem, output, callback=default_callback):
        """ Calculate the elevation of each grid cell above the nearest downstream pit cell or grid edge cell.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('ElevAbovePit', args, callback) # returns 1 if error

    def elev_percentile(self, dem, output, filterx=11, filtery=11, sig_digits=2, callback=default_callback):
        """ Calculates the elevation percentile raster from a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        sig_digits -- Number of significant digits. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        args.append("--sig_digits={}".format(sig_digits))
        return self.run_tool('ElevPercentile', args, callback) # returns 1 if error

    def elev_relative_to_min_max(self, dem, output, callback=default_callback):
        """ Calculates the elevation of a location relative to the minimum and maximum elevations in a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('ElevRelativeToMinMax', args, callback) # returns 1 if error

    def elev_relative_to_watershed_min_max(self, dem, watersheds, output, callback=default_callback):
        """ Calculates the elevation of a location relative to the minimum and maximum elevations in a watershed.

        Keyword arguments:

        dem -- Input raster DEM file. 
        watersheds -- Input raster watersheds file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--watersheds='{}'".format(watersheds))
        args.append("--output='{}'".format(output))
        return self.run_tool('ElevRelativeToWatershedMinMax', args, callback) # returns 1 if error

    def feature_preserving_denoise(self, dem, output, filter=11, norm_diff=15.0, num_iter=5, zfactor=1.0, callback=default_callback):
        """ Reduces short-scale variation in an input DEM using a modified Sun et al. (2007) algorithm.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        filter -- Size of the filter kernel. 
        norm_diff -- Maximum difference in normal vectors, in degrees. 
        num_iter -- Number of iterations. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filter={}".format(filter))
        args.append("--norm_diff={}".format(norm_diff))
        args.append("--num_iter={}".format(num_iter))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('FeaturePreservingDenoise', args, callback) # returns 1 if error

    def fetch_analysis(self, dem, output, azimuth=0.0, hgt_inc=0.05, callback=default_callback):
        """ Performs an analysis of fetch or upwind distance to an obstacle.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        azimuth -- Wind azimuth in degrees in degrees. 
        hgt_inc -- Height increment value. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--azimuth={}".format(azimuth))
        args.append("--hgt_inc={}".format(hgt_inc))
        return self.run_tool('FetchAnalysis', args, callback) # returns 1 if error

    def fill_missing_data(self, i, output, filter=11, callback=default_callback):
        """ Fills nodata holes in a DEM.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filter -- Filter size (cells). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filter={}".format(filter))
        return self.run_tool('FillMissingData', args, callback) # returns 1 if error

    def find_ridges(self, dem, output, line_thin=True, callback=default_callback):
        """ Identifies potential ridge and peak grid cells.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        line_thin -- Optional flag indicating whether post-processing line-thinning should be performed. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if line_thin: args.append("--line_thin")
        return self.run_tool('FindRidges', args, callback) # returns 1 if error

    def hillshade(self, dem, output, azimuth=315.0, altitude=30.0, zfactor=1.0, callback=default_callback):
        """ Calculates a hillshade raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        azimuth -- Illumination source azimuth in degrees. 
        altitude -- Illumination source altitude in degrees. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--azimuth={}".format(azimuth))
        args.append("--altitude={}".format(altitude))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('Hillshade', args, callback) # returns 1 if error

    def horizon_angle(self, dem, output, azimuth=0.0, max_dist=None, callback=default_callback):
        """ Calculates horizon angle (maximum upwind slope) for each grid cell in an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        azimuth -- Wind azimuth in degrees. 
        max_dist -- Optional maximum search distance (unspecified if none; in xy units). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--azimuth={}".format(azimuth))
        if max_dist is not None: args.append("--max_dist='{}'".format(max_dist))
        return self.run_tool('HorizonAngle', args, callback) # returns 1 if error

    def hypsometric_analysis(self, inputs, output, watershed=None, callback=default_callback):
        """ Calculates a hypsometric curve for one or more DEMs.

        Keyword arguments:

        inputs -- Input DEM files. 
        watershed -- Input watershed files (optional). 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        if watershed is not None: args.append("--watershed='{}'".format(watershed))
        args.append("--output='{}'".format(output))
        return self.run_tool('HypsometricAnalysis', args, callback) # returns 1 if error

    def max_anisotropy_dev(self, dem, out_mag, out_scale, max_scale, min_scale=3, step=2, callback=default_callback):
        """ Calculates the maximum anisotropy (directionality) in elevation deviation over a range of spatial scales.

        Keyword arguments:

        dem -- Input raster DEM file. 
        out_mag -- Output raster DEVmax magnitude file. 
        out_scale -- Output raster DEVmax scale file. 
        min_scale -- Minimum search neighbourhood radius in grid cells. 
        max_scale -- Maximum search neighbourhood radius in grid cells. 
        step -- Step size as any positive non-zero integer. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--out_mag='{}'".format(out_mag))
        args.append("--out_scale='{}'".format(out_scale))
        args.append("--min_scale={}".format(min_scale))
        args.append("--max_scale='{}'".format(max_scale))
        args.append("--step={}".format(step))
        return self.run_tool('MaxAnisotropyDev', args, callback) # returns 1 if error

    def max_anisotropy_dev_signature(self, dem, points, output, max_scale, min_scale=1, step=1, callback=default_callback):
        """ Calculates the anisotropy in deviation from mean for points over a range of spatial scales.

        Keyword arguments:

        dem -- Input raster DEM file. 
        points -- Input vector points file. 
        output -- Output HTML file. 
        min_scale -- Minimum search neighbourhood radius in grid cells. 
        max_scale -- Maximum search neighbourhood radius in grid cells. 
        step -- Step size as any positive non-zero integer. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--points='{}'".format(points))
        args.append("--output='{}'".format(output))
        args.append("--min_scale={}".format(min_scale))
        args.append("--max_scale='{}'".format(max_scale))
        args.append("--step={}".format(step))
        return self.run_tool('MaxAnisotropyDevSignature', args, callback) # returns 1 if error

    def max_branch_length(self, dem, output, log=False, callback=default_callback):
        """ Lindsay and Seibert's (2013) branch length index is used to map drainage divides or ridge lines.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        log -- Optional flag to request the output be log-transformed. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if log: args.append("--log")
        return self.run_tool('MaxBranchLength', args, callback) # returns 1 if error

    def max_downslope_elev_change(self, dem, output, callback=default_callback):
        """ Calculates the maximum downslope change in elevation between a grid cell and its eight downslope neighbors.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('MaxDownslopeElevChange', args, callback) # returns 1 if error

    def max_elev_dev_signature(self, dem, points, output, min_scale, max_scale, step=10, callback=default_callback):
        """ Calculates the maximum elevation deviation over a range of spatial scales and for a set of points.

        Keyword arguments:

        dem -- Input raster DEM file. 
        points -- Input vector points file. 
        output -- Output HTML file. 
        min_scale -- Minimum search neighbourhood radius in grid cells. 
        max_scale -- Maximum search neighbourhood radius in grid cells. 
        step -- Step size as any positive non-zero integer. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--points='{}'".format(points))
        args.append("--output='{}'".format(output))
        args.append("--min_scale='{}'".format(min_scale))
        args.append("--max_scale='{}'".format(max_scale))
        args.append("--step={}".format(step))
        return self.run_tool('MaxElevDevSignature', args, callback) # returns 1 if error

    def max_elevation_deviation(self, dem, out_mag, out_scale, min_scale, max_scale, step=10, callback=default_callback):
        """ Calculates the maximum elevation deviation over a range of spatial scales.

        Keyword arguments:

        dem -- Input raster DEM file. 
        out_mag -- Output raster DEVmax magnitude file. 
        out_scale -- Output raster DEVmax scale file. 
        min_scale -- Minimum search neighbourhood radius in grid cells. 
        max_scale -- Maximum search neighbourhood radius in grid cells. 
        step -- Step size as any positive non-zero integer. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--out_mag='{}'".format(out_mag))
        args.append("--out_scale='{}'".format(out_scale))
        args.append("--min_scale='{}'".format(min_scale))
        args.append("--max_scale='{}'".format(max_scale))
        args.append("--step={}".format(step))
        return self.run_tool('MaxElevationDeviation', args, callback) # returns 1 if error

    def min_downslope_elev_change(self, dem, output, callback=default_callback):
        """ Calculates the minimum downslope change in elevation between a grid cell and its eight downslope neighbors.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('MinDownslopeElevChange', args, callback) # returns 1 if error

    def multiscale_roughness(self, dem, out_mag, out_scale, max_scale, min_scale=1, step=1, callback=default_callback):
        """ Calculates surface roughness over a range of spatial scales.

        Keyword arguments:

        dem -- Input raster DEM file. 
        out_mag -- Output raster roughness magnitude file. 
        out_scale -- Output raster roughness scale file. 
        min_scale -- Minimum search neighbourhood radius in grid cells. 
        max_scale -- Maximum search neighbourhood radius in grid cells. 
        step -- Step size as any positive non-zero integer. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--out_mag='{}'".format(out_mag))
        args.append("--out_scale='{}'".format(out_scale))
        args.append("--min_scale={}".format(min_scale))
        args.append("--max_scale='{}'".format(max_scale))
        args.append("--step={}".format(step))
        return self.run_tool('MultiscaleRoughness', args, callback) # returns 1 if error

    def multiscale_roughness_signature(self, dem, points, output, max_scale, min_scale=1, step=1, callback=default_callback):
        """ Calculates the surface roughness for points over a range of spatial scales.

        Keyword arguments:

        dem -- Input raster DEM file. 
        points -- Input vector points file. 
        output -- Output HTML file. 
        min_scale -- Minimum search neighbourhood radius in grid cells. 
        max_scale -- Maximum search neighbourhood radius in grid cells. 
        step -- Step size as any positive non-zero integer. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--points='{}'".format(points))
        args.append("--output='{}'".format(output))
        args.append("--min_scale={}".format(min_scale))
        args.append("--max_scale='{}'".format(max_scale))
        args.append("--step={}".format(step))
        return self.run_tool('MultiscaleRoughnessSignature', args, callback) # returns 1 if error

    def multiscale_topographic_position_image(self, local, meso, broad, output, lightness=1.2, callback=default_callback):
        """ Creates a multiscale topographic position image from three DEVmax rasters of differing spatial scale ranges.

        Keyword arguments:

        local -- Input local-scale topographic position (DEVmax) raster file. 
        meso -- Input meso-scale topographic position (DEVmax) raster file. 
        broad -- Input broad-scale topographic position (DEVmax) raster file. 
        output -- Output raster file. 
        lightness -- Image lightness value (default is 1.2). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--local='{}'".format(local))
        args.append("--meso='{}'".format(meso))
        args.append("--broad='{}'".format(broad))
        args.append("--output='{}'".format(output))
        args.append("--lightness={}".format(lightness))
        return self.run_tool('MultiscaleTopographicPositionImage', args, callback) # returns 1 if error

    def num_downslope_neighbours(self, dem, output, callback=default_callback):
        """ Calculates the number of downslope neighbours to each grid cell in a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('NumDownslopeNeighbours', args, callback) # returns 1 if error

    def num_upslope_neighbours(self, dem, output, callback=default_callback):
        """ Calculates the number of upslope neighbours to each grid cell in a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('NumUpslopeNeighbours', args, callback) # returns 1 if error

    def pennock_landform_class(self, dem, output, slope=3.0, prof=0.1, plan=0.0, zfactor=1.0, callback=default_callback):
        """ Classifies hillslope zones based on slope, profile curvature, and plan curvature.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        slope -- Slope threshold value, in degrees (default is 3.0). 
        prof -- Profile curvature threshold value (default is 0.1). 
        plan -- Plan curvature threshold value (default is 0.0). 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--slope={}".format(slope))
        args.append("--prof={}".format(prof))
        args.append("--plan={}".format(plan))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('PennockLandformClass', args, callback) # returns 1 if error

    def percent_elev_range(self, dem, output, filterx=3, filtery=3, callback=default_callback):
        """ Calculates percent of elevation range from a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('PercentElevRange', args, callback) # returns 1 if error

    def plan_curvature(self, dem, output, zfactor=1.0, callback=default_callback):
        """ Calculates a plan (contour) curvature raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('PlanCurvature', args, callback) # returns 1 if error

    def profile(self, lines, surface, output, callback=default_callback):
        """ Plots profiles from digital surface models.

        Keyword arguments:

        lines -- Input vector line file. 
        surface -- Input raster surface file. 
        output -- Output HTML file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--lines='{}'".format(lines))
        args.append("--surface='{}'".format(surface))
        args.append("--output='{}'".format(output))
        return self.run_tool('Profile', args, callback) # returns 1 if error

    def profile_curvature(self, dem, output, zfactor=1.0, callback=default_callback):
        """ Calculates a profile curvature raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('ProfileCurvature', args, callback) # returns 1 if error

    def relative_aspect(self, dem, output, azimuth=0.0, zfactor=1.0, callback=default_callback):
        """ Calculates relative aspect (relative to a user-specified direction) from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        azimuth -- Illumination source azimuth. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--azimuth={}".format(azimuth))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('RelativeAspect', args, callback) # returns 1 if error

    def relative_stream_power_index(self, sca, slope, output, exponent=1.0, callback=default_callback):
        """ Calculates the relative stream power index.

        Keyword arguments:

        sca -- Input raster specific contributing area (SCA) file. 
        slope -- Input raster slope file. 
        output -- Output raster file. 
        exponent -- SCA exponent value. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--sca='{}'".format(sca))
        args.append("--slope='{}'".format(slope))
        args.append("--output='{}'".format(output))
        args.append("--exponent={}".format(exponent))
        return self.run_tool('RelativeStreamPowerIndex', args, callback) # returns 1 if error

    def relative_topographic_position(self, dem, output, filterx=11, filtery=11, callback=default_callback):
        """ Calculates the relative topographic position index from a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('RelativeTopographicPosition', args, callback) # returns 1 if error

    def remove_off_terrain_objects(self, dem, output, filter=11, slope=15.0, callback=default_callback):
        """ Removes off-terrain objects from a raster digital elevation model (DEM).

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        filter -- Filter size (cells). 
        slope -- Slope threshold value. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filter={}".format(filter))
        args.append("--slope={}".format(slope))
        return self.run_tool('RemoveOffTerrainObjects', args, callback) # returns 1 if error

    def ruggedness_index(self, dem, output, zfactor=1.0, callback=default_callback):
        """ Calculates the Riley et al.'s (1999) terrain ruggedness index from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('RuggednessIndex', args, callback) # returns 1 if error

    def sediment_transport_index(self, sca, slope, output, sca_exponent=0.4, slope_exponent=1.3, callback=default_callback):
        """ Calculates the sediment transport index.

        Keyword arguments:

        sca -- Input raster specific contributing area (SCA) file. 
        slope -- Input raster slope file. 
        output -- Output raster file. 
        sca_exponent -- SCA exponent value. 
        slope_exponent -- Slope exponent value. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--sca='{}'".format(sca))
        args.append("--slope='{}'".format(slope))
        args.append("--output='{}'".format(output))
        args.append("--sca_exponent={}".format(sca_exponent))
        args.append("--slope_exponent={}".format(slope_exponent))
        return self.run_tool('SedimentTransportIndex', args, callback) # returns 1 if error

    def slope(self, dem, output, zfactor=1.0, callback=default_callback):
        """ Calculates a slope raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('Slope', args, callback) # returns 1 if error

    def slope_vs_elevation_plot(self, inputs, output, watershed=None, callback=default_callback):
        """ Creates a slope vs. elevation plot for one or more DEMs.

        Keyword arguments:

        inputs -- Input DEM files. 
        watershed -- Input watershed files (optional). 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        if watershed is not None: args.append("--watershed='{}'".format(watershed))
        args.append("--output='{}'".format(output))
        return self.run_tool('SlopeVsElevationPlot', args, callback) # returns 1 if error

    def tangential_curvature(self, dem, output, zfactor=1.0, callback=default_callback):
        """ Calculates a tangential curvature raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('TangentialCurvature', args, callback) # returns 1 if error

    def total_curvature(self, dem, output, zfactor=1.0, callback=default_callback):
        """ Calculates a total curvature raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('TotalCurvature', args, callback) # returns 1 if error

    def viewshed(self, dem, stations, output, height=2.0, callback=default_callback):
        """ Identifies the viewshed for a point or set of points.

        Keyword arguments:

        dem -- Input raster DEM file. 
        stations -- Input viewing station vector file. 
        output -- Output raster file. 
        height -- Viewing station height, in z units. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--stations='{}'".format(stations))
        args.append("--output='{}'".format(output))
        args.append("--height={}".format(height))
        return self.run_tool('Viewshed', args, callback) # returns 1 if error

    def visibility_index(self, dem, output, height=2.0, res_factor=2, callback=default_callback):
        """ Estimates the relative visibility of sites in a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        height -- Viewing station height, in z units. 
        res_factor -- The resolution factor determines the density of measured viewsheds. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--height={}".format(height))
        args.append("--res_factor={}".format(res_factor))
        return self.run_tool('VisibilityIndex', args, callback) # returns 1 if error

    def wetness_index(self, sca, slope, output, callback=default_callback):
        """ Calculates the topographic wetness index, Ln(A / tan(slope)).

        Keyword arguments:

        sca -- Input raster specific contributing area (SCA) file. 
        slope -- Input raster slope file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--sca='{}'".format(sca))
        args.append("--slope='{}'".format(slope))
        args.append("--output='{}'".format(output))
        return self.run_tool('WetnessIndex', args, callback) # returns 1 if error

    #########################
    # Hydrological Analysis #
    #########################

    def average_flowpath_slope(self, dem, output, callback=default_callback):
        """ Measures the average slope gradient from each grid cell to all upslope divide cells.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('AverageFlowpathSlope', args, callback) # returns 1 if error

    def average_upslope_flowpath_length(self, dem, output, callback=default_callback):
        """ Measures the average length of all upslope flowpaths draining each grid cell.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('AverageUpslopeFlowpathLength', args, callback) # returns 1 if error

    def basins(self, d8_pntr, output, esri_pntr=False, callback=default_callback):
        """ Identifies drainage basins that drain to the DEM edge.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('Basins', args, callback) # returns 1 if error

    def breach_depressions(self, dem, output, max_depth=None, max_length=None, callback=default_callback):
        """ Breaches all of the depressions in a DEM using Lindsay's (2016) algorithm. This should be preferred over depression filling in most cases.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        max_depth -- Optional maximum breach depth (default is Inf). 
        max_length -- Optional maximum breach channel length (in grid cells; default is Inf). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if max_depth is not None: args.append("--max_depth='{}'".format(max_depth))
        if max_length is not None: args.append("--max_length='{}'".format(max_length))
        return self.run_tool('BreachDepressions', args, callback) # returns 1 if error

    def breach_single_cell_pits(self, dem, output, callback=default_callback):
        """ Removes single-cell pits from an input DEM by breaching.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('BreachSingleCellPits', args, callback) # returns 1 if error

    def d8_flow_accumulation(self, dem, output, out_type="specific contributing area", log=False, clip=False, callback=default_callback):
        """ Calculates a D8 flow accumulation raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        out_type -- Output type; one of 'cells', 'specific contributing area' (default), and 'catchment area'. 
        log -- Optional flag to request the output be log-transformed. 
        clip -- Optional flag to request clipping the display max by 1%. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--out_type={}".format(out_type))
        if log: args.append("--log")
        if clip: args.append("--clip")
        return self.run_tool('D8FlowAccumulation', args, callback) # returns 1 if error

    def d8_mass_flux(self, dem, loading, efficiency, absorption, output, callback=default_callback):
        """ Performs a D8 mass flux calculation.

        Keyword arguments:

        dem -- Input raster DEM file. 
        loading -- Input loading raster file. 
        efficiency -- Input efficiency raster file. 
        absorption -- Input absorption raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--loading='{}'".format(loading))
        args.append("--efficiency='{}'".format(efficiency))
        args.append("--absorption='{}'".format(absorption))
        args.append("--output='{}'".format(output))
        return self.run_tool('D8MassFlux', args, callback) # returns 1 if error

    def d8_pointer(self, dem, output, esri_pntr=False, callback=default_callback):
        """ Calculates a D8 flow pointer raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('D8Pointer', args, callback) # returns 1 if error

    def d_inf_flow_accumulation(self, dem, output, out_type="Specific Contributing Area", threshold=None, log=False, clip=False, callback=default_callback):
        """ Calculates a D-infinity flow accumulation raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        out_type -- Output type; one of 'cells', 'sca' (default), and 'ca'. 
        threshold -- Optional convergence threshold parameter, in grid cells; default is inifinity. 
        log -- Optional flag to request the output be log-transformed. 
        clip -- Optional flag to request clipping the display max by 1%. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--out_type={}".format(out_type))
        if threshold is not None: args.append("--threshold='{}'".format(threshold))
        if log: args.append("--log")
        if clip: args.append("--clip")
        return self.run_tool('DInfFlowAccumulation', args, callback) # returns 1 if error

    def d_inf_mass_flux(self, dem, loading, efficiency, absorption, output, callback=default_callback):
        """ Performs a D-infinity mass flux calculation.

        Keyword arguments:

        dem -- Input raster DEM file. 
        loading -- Input loading raster file. 
        efficiency -- Input efficiency raster file. 
        absorption -- Input absorption raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--loading='{}'".format(loading))
        args.append("--efficiency='{}'".format(efficiency))
        args.append("--absorption='{}'".format(absorption))
        args.append("--output='{}'".format(output))
        return self.run_tool('DInfMassFlux', args, callback) # returns 1 if error

    def d_inf_pointer(self, dem, output, callback=default_callback):
        """ Calculates a D-infinity flow pointer (flow direction) raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('DInfPointer', args, callback) # returns 1 if error

    def depth_in_sink(self, dem, output, zero_background=False, callback=default_callback):
        """ Measures the depth of sinks (depressions) in a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        zero_background -- Flag indicating whether the background value of zero should be used. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if zero_background: args.append("--zero_background")
        return self.run_tool('DepthInSink', args, callback) # returns 1 if error

    def downslope_distance_to_stream(self, dem, streams, output, callback=default_callback):
        """ Measures distance to the nearest downslope stream cell.

        Keyword arguments:

        dem -- Input raster DEM file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        return self.run_tool('DownslopeDistanceToStream', args, callback) # returns 1 if error

    def downslope_flowpath_length(self, d8_pntr, output, watersheds=None, weights=None, esri_pntr=False, callback=default_callback):
        """ Calculates the downslope flowpath length from each cell to basin outlet.

        Keyword arguments:

        d8_pntr -- Input D8 pointer raster file. 
        watersheds -- Optional input watershed raster file. 
        weights -- Optional input weights raster file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        if watersheds is not None: args.append("--watersheds='{}'".format(watersheds))
        if weights is not None: args.append("--weights='{}'".format(weights))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('DownslopeFlowpathLength', args, callback) # returns 1 if error

    def elevation_above_stream(self, dem, streams, output, callback=default_callback):
        """ Calculates the elevation of cells above the nearest downslope stream cell.

        Keyword arguments:

        dem -- Input raster DEM file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        return self.run_tool('ElevationAboveStream', args, callback) # returns 1 if error

    def elevation_above_stream_euclidean(self, dem, streams, output, callback=default_callback):
        """ Calculates the elevation of cells above the nearest (Euclidean distance) stream cell.

        Keyword arguments:

        dem -- Input raster DEM file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        return self.run_tool('ElevationAboveStreamEuclidean', args, callback) # returns 1 if error

    def fd8_flow_accumulation(self, dem, output, out_type="specific contributing area", exponent=1.1, threshold=None, log=False, clip=False, callback=default_callback):
        """ Calculates an FD8 flow accumulation raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        out_type -- Output type; one of 'cells', 'specific contributing area' (default), and 'catchment area'. 
        exponent -- Optional exponent parameter; default is 1.1. 
        threshold -- Optional convergence threshold parameter, in grid cells; default is inifinity. 
        log -- Optional flag to request the output be log-transformed. 
        clip -- Optional flag to request clipping the display max by 1%. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--out_type={}".format(out_type))
        args.append("--exponent={}".format(exponent))
        if threshold is not None: args.append("--threshold='{}'".format(threshold))
        if log: args.append("--log")
        if clip: args.append("--clip")
        return self.run_tool('FD8FlowAccumulation', args, callback) # returns 1 if error

    def fd8_pointer(self, dem, output, callback=default_callback):
        """ Calculates an FD8 flow pointer raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('FD8Pointer', args, callback) # returns 1 if error

    def fill_burn(self, dem, streams, output, callback=default_callback):
        """ Burns streams into a DEM using the FillBurn (Saunders, 1999) method.

        Keyword arguments:

        dem -- Input raster DEM file. 
        streams -- Input vector streams file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        return self.run_tool('FillBurn', args, callback) # returns 1 if error

    def fill_depressions(self, dem, output, fix_flats=True, callback=default_callback):
        """ Fills all of the depressions in a DEM. Depression breaching should be preferred in most cases.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        fix_flats -- Optional flag indicating whether flat areas should have a small gradient applied. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if fix_flats: args.append("--fix_flats")
        return self.run_tool('FillDepressions', args, callback) # returns 1 if error

    def fill_single_cell_pits(self, dem, output, callback=default_callback):
        """ Raises pit cells to the elevation of their lowest neighbour.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('FillSingleCellPits', args, callback) # returns 1 if error

    def find_no_flow_cells(self, dem, output, callback=default_callback):
        """ Finds grid cells with no downslope neighbours.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('FindNoFlowCells', args, callback) # returns 1 if error

    def find_parallel_flow(self, d8_pntr, streams, output, callback=default_callback):
        """ Finds areas of parallel flow in D8 flow direction rasters.

        Keyword arguments:

        d8_pntr -- Input D8 pointer raster file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        return self.run_tool('FindParallelFlow', args, callback) # returns 1 if error

    def flatten_lakes(self, dem, lakes, output, callback=default_callback):
        """ Flattens lake polygons in a raster DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        lakes -- Input lakes vector polygons file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--lakes='{}'".format(lakes))
        args.append("--output='{}'".format(output))
        return self.run_tool('FlattenLakes', args, callback) # returns 1 if error

    def flood_order(self, dem, output, callback=default_callback):
        """ Assigns each DEM grid cell its order in the sequence of inundations that are encountered during a search starting from the edges, moving inward at increasing elevations.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('FloodOrder', args, callback) # returns 1 if error

    def flow_accumulation_full_workflow(self, dem, out_dem, out_pntr, out_accum, out_type="Specific Contributing Area", log=False, clip=False, esri_pntr=False, callback=default_callback):
        """ Resolves all of the depressions in a DEM, outputting a breached DEM, an aspect-aligned non-divergent flow pointer, a flow accumulation raster.

        Keyword arguments:

        dem -- Input raster DEM file. 
        out_dem -- Output raster DEM file. 
        out_pntr -- Output raster flow pointer file. 
        out_accum -- Output raster flow accumulation file. 
        out_type -- Output type; one of 'cells', 'sca' (default), and 'ca'. 
        log -- Optional flag to request the output be log-transformed. 
        clip -- Optional flag to request clipping the display max by 1%. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--out_dem='{}'".format(out_dem))
        args.append("--out_pntr='{}'".format(out_pntr))
        args.append("--out_accum='{}'".format(out_accum))
        args.append("--out_type={}".format(out_type))
        if log: args.append("--log")
        if clip: args.append("--clip")
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('FlowAccumulationFullWorkflow', args, callback) # returns 1 if error

    def flow_length_diff(self, d8_pntr, output, esri_pntr=False, callback=default_callback):
        """ Calculates the local maximum absolute difference in downslope flowpath length, useful in mapping drainage divides and ridges.

        Keyword arguments:

        d8_pntr -- Input D8 pointer raster file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('FlowLengthDiff', args, callback) # returns 1 if error

    def hillslopes(self, d8_pntr, streams, output, esri_pntr=False, callback=default_callback):
        """ Identifies the individual hillslopes draining to each link in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('Hillslopes', args, callback) # returns 1 if error

    def isobasins(self, dem, output, size, callback=default_callback):
        """ Divides a landscape into nearly equal sized drainage basins (i.e. watersheds).

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        size -- Target basin size, in grid cells. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--size='{}'".format(size))
        return self.run_tool('Isobasins', args, callback) # returns 1 if error

    def jenson_snap_pour_points(self, pour_pts, streams, output, snap_dist, callback=default_callback):
        """ Moves outlet points used to specify points of interest in a watershedding operation to the nearest stream cell.

        Keyword arguments:

        pour_pts -- Input raster pour points (outlet) file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        snap_dist -- Maximum snap distance in map units. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--pour_pts='{}'".format(pour_pts))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        args.append("--snap_dist='{}'".format(snap_dist))
        return self.run_tool('JensonSnapPourPoints', args, callback) # returns 1 if error

    def max_upslope_flowpath_length(self, dem, output, callback=default_callback):
        """ Measures the maximum length of all upslope flowpaths draining each grid cell.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('MaxUpslopeFlowpathLength', args, callback) # returns 1 if error

    def num_inflowing_neighbours(self, dem, output, callback=default_callback):
        """ Computes the number of inflowing neighbours to each cell in an input DEM based on the D8 algorithm.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('NumInflowingNeighbours', args, callback) # returns 1 if error

    def raise_walls(self, i, dem, output, breach=None, height=100.0, callback=default_callback):
        """ Raises walls in a DEM along a line or around a polygon, e.g. a watershed.

        Keyword arguments:

        i -- Input vector lines or polygons file. 
        breach -- Optional input vector breach lines. 
        dem -- Input raster DEM file. 
        output -- Output raster file. 
        height -- Wall height. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        if breach is not None: args.append("--breach='{}'".format(breach))
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--height={}".format(height))
        return self.run_tool('RaiseWalls', args, callback) # returns 1 if error

    def rho8_pointer(self, dem, output, esri_pntr=False, callback=default_callback):
        """ Calculates a stochastic Rho8 flow pointer raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('Rho8Pointer', args, callback) # returns 1 if error

    def sink(self, dem, output, zero_background=False, callback=default_callback):
        """ Identifies the depressions in a DEM, giving each feature a unique identifier.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if zero_background: args.append("--zero_background")
        return self.run_tool('Sink', args, callback) # returns 1 if error

    def snap_pour_points(self, pour_pts, flow_accum, output, snap_dist, callback=default_callback):
        """ Moves outlet points used to specify points of interest in a watershedding operation to the cell with the highest flow accumulation in its neighbourhood.

        Keyword arguments:

        pour_pts -- Input raster pour points (outlet) file. 
        flow_accum -- Input raster D8 flow accumulation file. 
        output -- Output raster file. 
        snap_dist -- Maximum snap distance in map units. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--pour_pts='{}'".format(pour_pts))
        args.append("--flow_accum='{}'".format(flow_accum))
        args.append("--output='{}'".format(output))
        args.append("--snap_dist='{}'".format(snap_dist))
        return self.run_tool('SnapPourPoints', args, callback) # returns 1 if error

    def strahler_order_basins(self, d8_pntr, streams, output, esri_pntr=False, callback=default_callback):
        """ Identifies Strahler-order basins from an input stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('StrahlerOrderBasins', args, callback) # returns 1 if error

    def subbasins(self, d8_pntr, streams, output, esri_pntr=False, callback=default_callback):
        """ Identifies the catchments, or sub-basin, draining to each link in a stream network.

        Keyword arguments:

        d8_pntr -- Input D8 pointer raster file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('Subbasins', args, callback) # returns 1 if error

    def trace_downslope_flowpaths(self, seed_pts, d8_pntr, output, esri_pntr=False, zero_background=False, callback=default_callback):
        """ Traces downslope flowpaths from one or more target sites (i.e. seed points).

        Keyword arguments:

        seed_pts -- Input raster seed points file. 
        d8_pntr -- Input D8 pointer raster file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--seed_pts='{}'".format(seed_pts))
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('TraceDownslopeFlowpaths', args, callback) # returns 1 if error

    def unnest_basins(self, d8_pntr, pour_pts, output, esri_pntr=False, callback=default_callback):
        """ Extract whole watersheds for a set of outlet points.

        Keyword arguments:

        d8_pntr -- Input D8 pointer raster file. 
        pour_pts -- Input vector pour points (outlet) file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--pour_pts='{}'".format(pour_pts))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('UnnestBasins', args, callback) # returns 1 if error

    def watershed(self, d8_pntr, pour_pts, output, esri_pntr=False, callback=default_callback):
        """ Identifies the watershed, or drainage basin, draining to a set of target cells.

        Keyword arguments:

        d8_pntr -- Input D8 pointer raster file. 
        pour_pts -- Input vector pour points (outlet) file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--pour_pts='{}'".format(pour_pts))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('Watershed', args, callback) # returns 1 if error

    ##########################
    # Image Processing Tools #
    ##########################

    def change_vector_analysis(self, date1, date2, magnitude, direction, callback=default_callback):
        """ Performs a change vector analysis on a two-date multi-spectral dataset.

        Keyword arguments:

        date1 -- Input raster files for the earlier date. 
        date2 -- Input raster files for the later date. 
        magnitude -- Output vector magnitude raster file. 
        direction -- Output vector Direction raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--date1='{}'".format(date1))
        args.append("--date2='{}'".format(date2))
        args.append("--magnitude='{}'".format(magnitude))
        args.append("--direction='{}'".format(direction))
        return self.run_tool('ChangeVectorAnalysis', args, callback) # returns 1 if error

    def closing(self, i, output, filterx=11, filtery=11, callback=default_callback):
        """ A closing is a mathematical morphology operating involving an erosion (min filter) of a dilation (max filter) set.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('Closing', args, callback) # returns 1 if error

    def create_colour_composite(self, red, green, blue, output, opacity=None, enhance=True, callback=default_callback):
        """ Creates a colour-composite image from three bands of multispectral imagery.

        Keyword arguments:

        red -- Input red band image file. 
        green -- Input green band image file. 
        blue -- Input blue band image file. 
        opacity -- Input opacity band image file (optional). 
        output -- Output colour composite file. 
        enhance -- Optional flag indicating whether a balance contrast enhancement is performed. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--red='{}'".format(red))
        args.append("--green='{}'".format(green))
        args.append("--blue='{}'".format(blue))
        if opacity is not None: args.append("--opacity='{}'".format(opacity))
        args.append("--output='{}'".format(output))
        if enhance: args.append("--enhance")
        return self.run_tool('CreateColourComposite', args, callback) # returns 1 if error

    def flip_image(self, i, output, direction="vertical", callback=default_callback):
        """ Reflects an image in the vertical or horizontal axis.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        direction -- Direction of reflection; options include 'v' (vertical), 'h' (horizontal), and 'b' (both). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--direction={}".format(direction))
        return self.run_tool('FlipImage', args, callback) # returns 1 if error

    def ihs_to_rgb(self, intensity, hue, saturation, red=None, green=None, blue=None, output=None, callback=default_callback):
        """ Converts intensity, hue, and saturation (IHS) images into red, green, and blue (RGB) images.

        Keyword arguments:

        intensity -- Input intensity file. 
        hue -- Input hue file. 
        saturation -- Input saturation file. 
        red -- Output red band file. Optionally specified if colour-composite not specified. 
        green -- Output green band file. Optionally specified if colour-composite not specified. 
        blue -- Output blue band file. Optionally specified if colour-composite not specified. 
        output -- Output colour-composite file. Only used if individual bands are not specified. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--intensity='{}'".format(intensity))
        args.append("--hue='{}'".format(hue))
        args.append("--saturation='{}'".format(saturation))
        if red is not None: args.append("--red='{}'".format(red))
        if green is not None: args.append("--green='{}'".format(green))
        if blue is not None: args.append("--blue='{}'".format(blue))
        if output is not None: args.append("--output='{}'".format(output))
        return self.run_tool('IhsToRgb', args, callback) # returns 1 if error

    def image_stack_profile(self, inputs, points, output, callback=default_callback):
        """ Plots an image stack profile (i.e. signature) for a set of points and multispectral images.

        Keyword arguments:

        inputs -- Input multispectral image files. 
        points -- Input vector points file. 
        output -- Output HTML file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--points='{}'".format(points))
        args.append("--output='{}'".format(output))
        return self.run_tool('ImageStackProfile', args, callback) # returns 1 if error

    def integral_image(self, i, output, callback=default_callback):
        """ Transforms an input image (summed area table) into its integral image equivalent.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('IntegralImage', args, callback) # returns 1 if error

    def k_means_clustering(self, inputs, output, classes, out_html=None, max_iterations=10, class_change=2.0, initialize="diagonal", min_class_size=10, callback=default_callback):
        """ Performs a k-means clustering operation on a multi-spectral dataset.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        out_html -- Output HTML report file. 
        classes -- Number of classes. 
        max_iterations -- Maximum number of iterations. 
        class_change -- Minimum percent of cells changed between iterations before completion. 
        initialize -- How to initialize cluster centres?. 
        min_class_size -- Minimum class size, in pixels. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        if out_html is not None: args.append("--out_html='{}'".format(out_html))
        args.append("--classes='{}'".format(classes))
        args.append("--max_iterations={}".format(max_iterations))
        args.append("--class_change={}".format(class_change))
        args.append("--initialize={}".format(initialize))
        args.append("--min_class_size={}".format(min_class_size))
        return self.run_tool('KMeansClustering', args, callback) # returns 1 if error

    def line_thinning(self, i, output, callback=default_callback):
        """ Performs line thinning a on Boolean raster image; intended to be used with the RemoveSpurs tool.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('LineThinning', args, callback) # returns 1 if error

    def modified_k_means_clustering(self, inputs, output, out_html=None, start_clusters=1000, merger_dist=None, max_iterations=10, class_change=2.0, callback=default_callback):
        """ Performs a modified k-means clustering operation on a multi-spectral dataset.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        out_html -- Output HTML report file. 
        start_clusters -- Initial number of clusters. 
        merger_dist -- Cluster merger distance. 
        max_iterations -- Maximum number of iterations. 
        class_change -- Minimum percent of cells changed between iterations before completion. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        if out_html is not None: args.append("--out_html='{}'".format(out_html))
        args.append("--start_clusters={}".format(start_clusters))
        if merger_dist is not None: args.append("--merger_dist='{}'".format(merger_dist))
        args.append("--max_iterations={}".format(max_iterations))
        args.append("--class_change={}".format(class_change))
        return self.run_tool('ModifiedKMeansClustering', args, callback) # returns 1 if error

    def mosaic(self, inputs, output, method="cc", callback=default_callback):
        """ Mosaics two or more images together.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        method -- Resampling method. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        args.append("--method={}".format(method))
        return self.run_tool('Mosaic', args, callback) # returns 1 if error

    def normalized_difference_vegetation_index(self, nir, red, output, clip=0.0, osavi=False, callback=default_callback):
        """ Calculates the normalized difference vegetation index (NDVI) from near-infrared and red imagery.

        Keyword arguments:

        nir -- Input near-infrared band image. 
        red -- Input red band image. 
        output -- Output raster file. 
        clip -- Optional amount to clip the distribution tails by, in percent. 
        osavi -- Optional flag indicating whether the optimized soil-adjusted veg index (OSAVI) should be used. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--nir='{}'".format(nir))
        args.append("--red='{}'".format(red))
        args.append("--output='{}'".format(output))
        args.append("--clip={}".format(clip))
        if osavi: args.append("--osavi")
        return self.run_tool('NormalizedDifferenceVegetationIndex', args, callback) # returns 1 if error

    def opening(self, i, output, filterx=11, filtery=11, callback=default_callback):
        """ An opening is a mathematical morphology operating involving a dilation (max filter) of an erosion (min filter) set.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('Opening', args, callback) # returns 1 if error

    def remove_spurs(self, i, output, iterations=10, callback=default_callback):
        """ Removes the spurs (pruning operation) from a Boolean line image.; intended to be used on the output of the LineThinning tool.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        iterations -- Maximum number of iterations. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--iterations={}".format(iterations))
        return self.run_tool('RemoveSpurs', args, callback) # returns 1 if error

    def resample(self, inputs, destination, method="cc", callback=default_callback):
        """ Resamples one or more input images into a destination image.

        Keyword arguments:

        inputs -- Input raster files. 
        destination -- Destination raster file. 
        method -- Resampling method. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--destination='{}'".format(destination))
        args.append("--method={}".format(method))
        return self.run_tool('Resample', args, callback) # returns 1 if error

    def rgb_to_ihs(self, intensity, hue, saturation, red=None, green=None, blue=None, composite=None, callback=default_callback):
        """ Converts red, green, and blue (RGB) images into intensity, hue, and saturation (IHS) images.

        Keyword arguments:

        red -- Input red band image file. Optionally specified if colour-composite not specified. 
        green -- Input green band image file. Optionally specified if colour-composite not specified. 
        blue -- Input blue band image file. Optionally specified if colour-composite not specified. 
        composite -- Input colour-composite image file. Only used if individual bands are not specified. 
        intensity -- Output intensity raster file. 
        hue -- Output hue raster file. 
        saturation -- Output saturation raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        if red is not None: args.append("--red='{}'".format(red))
        if green is not None: args.append("--green='{}'".format(green))
        if blue is not None: args.append("--blue='{}'".format(blue))
        if composite is not None: args.append("--composite='{}'".format(composite))
        args.append("--intensity='{}'".format(intensity))
        args.append("--hue='{}'".format(hue))
        args.append("--saturation='{}'".format(saturation))
        return self.run_tool('RgbToIhs', args, callback) # returns 1 if error

    def split_colour_composite(self, i, output, callback=default_callback):
        """ This tool splits an RGB colour composite image into seperate multispectral images.

        Keyword arguments:

        i -- Input colour composite image file. 
        output -- Output raster file (suffixes of '_r', '_g', and '_b' will be appended). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('SplitColourComposite', args, callback) # returns 1 if error

    def thicken_raster_line(self, i, output, callback=default_callback):
        """ Thickens single-cell wide lines within a raster image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('ThickenRasterLine', args, callback) # returns 1 if error

    def tophat_transform(self, i, output, filterx=11, filtery=11, variant="white", callback=default_callback):
        """ Performs either a white or black top-hat transform on an input image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        variant -- Optional variant value. Options include 'white' and 'black'. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        args.append("--variant={}".format(variant))
        return self.run_tool('TophatTransform', args, callback) # returns 1 if error

    def write_function_memory_insertion(self, input1, input2, output, input3=None, callback=default_callback):
        """ Performs a write function memory insertion for single-band multi-date change detection.

        Keyword arguments:

        input1 -- Input raster file associated with the first date. 
        input2 -- Input raster file associated with the second date. 
        input3 -- Optional input raster file associated with the third date. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        if input3 is not None: args.append("--input3='{}'".format(input3))
        args.append("--output='{}'".format(output))
        return self.run_tool('WriteFunctionMemoryInsertion', args, callback) # returns 1 if error

    ##################################
    # Image Processing Tools/Filters #
    ##################################

    def adaptive_filter(self, i, output, filterx=11, filtery=11, threshold=2.0, callback=default_callback):
        """ Performs an adaptive filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        threshold -- Difference from mean threshold, in standard deviations. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        args.append("--threshold={}".format(threshold))
        return self.run_tool('AdaptiveFilter', args, callback) # returns 1 if error

    def bilateral_filter(self, i, output, sigma_dist=0.75, sigma_int=1.0, callback=default_callback):
        """ A bilateral filter is an edge-preserving smoothing filter introduced by Tomasi and Manduchi (1998).

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        sigma_dist -- Standard deviation in distance in pixels. 
        sigma_int -- Standard deviation in intensity in pixels. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--sigma_dist={}".format(sigma_dist))
        args.append("--sigma_int={}".format(sigma_int))
        return self.run_tool('BilateralFilter', args, callback) # returns 1 if error

    def conservative_smoothing_filter(self, i, output, filterx=11, filtery=11, callback=default_callback):
        """ Performs a conservative-smoothing filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('ConservativeSmoothingFilter', args, callback) # returns 1 if error

    def diff_of_gaussian_filter(self, i, output, sigma1=2.0, sigma2=4.0, callback=default_callback):
        """ Performs a Difference of Gaussian (DoG) filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        sigma1 -- Standard deviation distance in pixels. 
        sigma2 -- Standard deviation distance in pixels. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--sigma1={}".format(sigma1))
        args.append("--sigma2={}".format(sigma2))
        return self.run_tool('DiffOfGaussianFilter', args, callback) # returns 1 if error

    def diversity_filter(self, i, output, filterx=11, filtery=11, callback=default_callback):
        """ Assigns each cell in the output grid the number of different values in a moving window centred on each grid cell in the input raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('DiversityFilter', args, callback) # returns 1 if error

    def edge_preserving_mean_filter(self, i, output, threshold, filter=11, callback=default_callback):
        """ Performs a simple edge-preserving mean filter on an input image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filter -- Size of the filter kernel. 
        threshold -- Maximum difference in values. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filter={}".format(filter))
        args.append("--threshold='{}'".format(threshold))
        return self.run_tool('EdgePreservingMeanFilter', args, callback) # returns 1 if error

    def emboss_filter(self, i, output, direction="n", clip=0.0, callback=default_callback):
        """ Performs an emboss filter on an image, similar to a hillshade operation.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        direction -- Direction of reflection; options include 'n', 's', 'e', 'w', 'ne', 'se', 'nw', 'sw'. 
        clip -- Optional amount to clip the distribution tails by, in percent. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--direction={}".format(direction))
        args.append("--clip={}".format(clip))
        return self.run_tool('EmbossFilter', args, callback) # returns 1 if error

    def gaussian_filter(self, i, output, sigma=0.75, callback=default_callback):
        """ Performs a Gaussian filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        sigma -- Standard deviation distance in pixels. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--sigma={}".format(sigma))
        return self.run_tool('GaussianFilter', args, callback) # returns 1 if error

    def high_pass_filter(self, i, output, filterx=11, filtery=11, callback=default_callback):
        """ Performs a high-pass filter on an input image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('HighPassFilter', args, callback) # returns 1 if error

    def k_nearest_mean_filter(self, i, output, filterx=11, filtery=11, k=5, callback=default_callback):
        """ A k-nearest mean filter is a type of edge-preserving smoothing filter.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        k -- k-value in pixels; this is the number of nearest-valued neighbours to use. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        args.append("-k={}".format(k))
        return self.run_tool('KNearestMeanFilter', args, callback) # returns 1 if error

    def laplacian_filter(self, i, output, variant="3x3(1)", clip=0.0, callback=default_callback):
        """ Performs a Laplacian filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        variant -- Optional variant value. Options include 3x3(1), 3x3(2), 3x3(3), 3x3(4), 5x5(1), and 5x5(2) (default is 3x3(1)). 
        clip -- Optional amount to clip the distribution tails by, in percent. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--variant={}".format(variant))
        args.append("--clip={}".format(clip))
        return self.run_tool('LaplacianFilter', args, callback) # returns 1 if error

    def laplacian_of_gaussian_filter(self, i, output, sigma=0.75, callback=default_callback):
        """ Performs a Laplacian-of-Gaussian (LoG) filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        sigma -- Standard deviation in pixels. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--sigma={}".format(sigma))
        return self.run_tool('LaplacianOfGaussianFilter', args, callback) # returns 1 if error

    def lee_filter(self, i, output, filterx=11, filtery=11, sigma=10.0, m=5.0, callback=default_callback):
        """ Performs a Lee (Sigma) smoothing filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        sigma -- Sigma value should be related to the standarad deviation of the distribution of image speckle noise. 
        m -- M-threshold value the minimum allowable number of pixels within the intensity range. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        args.append("--sigma={}".format(sigma))
        args.append("-m={}".format(m))
        return self.run_tool('LeeFilter', args, callback) # returns 1 if error

    def line_detection_filter(self, i, output, variant="vertical", absvals=False, clip=0.0, callback=default_callback):
        """ Performs a line-detection filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        variant -- Optional variant value. Options include 'v' (vertical), 'h' (horizontal), '45', and '135' (default is 'v'). 
        absvals -- Optional flag indicating whether outputs should be absolute values. 
        clip -- Optional amount to clip the distribution tails by, in percent. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--variant={}".format(variant))
        if absvals: args.append("--absvals")
        args.append("--clip={}".format(clip))
        return self.run_tool('LineDetectionFilter', args, callback) # returns 1 if error

    def majority_filter(self, i, output, filterx=11, filtery=11, callback=default_callback):
        """ Assigns each cell in the output grid the most frequently occurring value (mode) in a moving window centred on each grid cell in the input raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('MajorityFilter', args, callback) # returns 1 if error

    def maximum_filter(self, i, output, filterx=11, filtery=11, callback=default_callback):
        """ Assigns each cell in the output grid the maximum value in a moving window centred on each grid cell in the input raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('MaximumFilter', args, callback) # returns 1 if error

    def mean_filter(self, i, output, filterx=3, filtery=3, callback=default_callback):
        """ Performs a mean filter (low-pass filter) on an input image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('MeanFilter', args, callback) # returns 1 if error

    def median_filter(self, i, output, filterx=11, filtery=11, sig_digits=2, callback=default_callback):
        """ Performs a median filter on an input image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        sig_digits -- Number of significant digits. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        args.append("--sig_digits={}".format(sig_digits))
        return self.run_tool('MedianFilter', args, callback) # returns 1 if error

    def minimum_filter(self, i, output, filterx=11, filtery=11, callback=default_callback):
        """ Assigns each cell in the output grid the minimum value in a moving window centred on each grid cell in the input raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('MinimumFilter', args, callback) # returns 1 if error

    def olympic_filter(self, i, output, filterx=11, filtery=11, callback=default_callback):
        """ Performs an olympic smoothing filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('OlympicFilter', args, callback) # returns 1 if error

    def percentile_filter(self, i, output, filterx=11, filtery=11, sig_digits=2, callback=default_callback):
        """ Performs a percentile filter on an input image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        sig_digits -- Number of significant digits. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        args.append("--sig_digits={}".format(sig_digits))
        return self.run_tool('PercentileFilter', args, callback) # returns 1 if error

    def prewitt_filter(self, i, output, clip=0.0, callback=default_callback):
        """ Performs a Prewitt edge-detection filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        clip -- Optional amount to clip the distribution tails by, in percent. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--clip={}".format(clip))
        return self.run_tool('PrewittFilter', args, callback) # returns 1 if error

    def range_filter(self, i, output, filterx=11, filtery=11, callback=default_callback):
        """ Assigns each cell in the output grid the range of values in a moving window centred on each grid cell in the input raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('RangeFilter', args, callback) # returns 1 if error

    def roberts_cross_filter(self, i, output, clip=0.0, callback=default_callback):
        """ Performs a Robert's cross edge-detection filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        clip -- Optional amount to clip the distribution tails by, in percent. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--clip={}".format(clip))
        return self.run_tool('RobertsCrossFilter', args, callback) # returns 1 if error

    def scharr_filter(self, i, output, clip=0.0, callback=default_callback):
        """ Performs a Scharr edge-detection filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        clip -- Optional amount to clip the distribution tails by, in percent. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--clip={}".format(clip))
        return self.run_tool('ScharrFilter', args, callback) # returns 1 if error

    def sobel_filter(self, i, output, variant="3x3", clip=0.0, callback=default_callback):
        """ Performs a Sobel edge-detection filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        variant -- Optional variant value. Options include 3x3 and 5x5 (default is 3x3). 
        clip -- Optional amount to clip the distribution tails by, in percent (default is 0.0). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--variant={}".format(variant))
        args.append("--clip={}".format(clip))
        return self.run_tool('SobelFilter', args, callback) # returns 1 if error

    def standard_deviation_filter(self, i, output, filterx=11, filtery=11, callback=default_callback):
        """ Assigns each cell in the output grid the standard deviation of values in a moving window centred on each grid cell in the input raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('StandardDeviationFilter', args, callback) # returns 1 if error

    def total_filter(self, i, output, filterx=11, filtery=11, callback=default_callback):
        """ Performs a total filter on an input image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('TotalFilter', args, callback) # returns 1 if error

    def user_defined_weights_filter(self, i, weights, output, center="center", normalize=False, callback=default_callback):
        """ Performs a user-defined weights filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        weights -- Input weights file. 
        output -- Output raster file. 
        center -- Kernel center cell; options include 'center', 'upper-left', 'upper-right', 'lower-left', 'lower-right'. 
        normalize -- Normalize kernel weights? This can reduce edge effects and lessen the impact of data gaps (nodata) but is not suited when the kernel weights sum to zero. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--weights='{}'".format(weights))
        args.append("--output='{}'".format(output))
        args.append("--center={}".format(center))
        if normalize: args.append("--normalize")
        return self.run_tool('UserDefinedWeightsFilter', args, callback) # returns 1 if error

    ############################################
    # Image Processing Tools/Image Enhancement #
    ############################################

    def balance_contrast_enhancement(self, i, output, band_mean=100.0, callback=default_callback):
        """ Performs a balance contrast enhancement on a colour-composite image of multispectral data.

        Keyword arguments:

        i -- Input colour composite image file. 
        output -- Output raster file. 
        band_mean -- Band mean value. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--band_mean={}".format(band_mean))
        return self.run_tool('BalanceContrastEnhancement', args, callback) # returns 1 if error

    def correct_vignetting(self, i, pp, output, focal_length=304.8, image_width=228.6, n=4.0, callback=default_callback):
        """ Corrects the darkening of images towards corners.

        Keyword arguments:

        i -- Input raster file. 
        pp -- Input principal point file. 
        output -- Output raster file. 
        focal_length -- Camera focal length, in millimeters. 
        image_width -- Distance between photograph edges, in millimeters. 
        n -- The 'n' parameter. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--pp='{}'".format(pp))
        args.append("--output='{}'".format(output))
        args.append("--focal_length={}".format(focal_length))
        args.append("--image_width={}".format(image_width))
        args.append("-n={}".format(n))
        return self.run_tool('CorrectVignetting', args, callback) # returns 1 if error

    def direct_decorrelation_stretch(self, i, output, k=0.5, clip=1.0, callback=default_callback):
        """ Performs a direct decorrelation stretch enhancement on a colour-composite image of multispectral data.

        Keyword arguments:

        i -- Input colour composite image file. 
        output -- Output raster file. 
        k -- Achromatic factor (k) ranges between 0 (no effect) and 1 (full saturation stretch), although typical values range from 0.3 to 0.7. 
        clip -- Optional percent to clip the upper tail by during the stretch. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("-k={}".format(k))
        args.append("--clip={}".format(clip))
        return self.run_tool('DirectDecorrelationStretch', args, callback) # returns 1 if error

    def gamma_correction(self, i, output, gamma=0.5, callback=default_callback):
        """ Performs a sigmoidal contrast stretch on input images.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        gamma -- Gamma value. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--gamma={}".format(gamma))
        return self.run_tool('GammaCorrection', args, callback) # returns 1 if error

    def histogram_equalization(self, i, output, num_tones=256, callback=default_callback):
        """ Performs a histogram equalization contrast enhancment on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        num_tones -- Number of tones in the output image. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--num_tones={}".format(num_tones))
        return self.run_tool('HistogramEqualization', args, callback) # returns 1 if error

    def histogram_matching(self, i, histo_file, output, callback=default_callback):
        """ Alters the statistical distribution of a raster image matching it to a specified PDF.

        Keyword arguments:

        i -- Input raster file. 
        histo_file -- Input reference probability distribution function (pdf) text file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--histo_file='{}'".format(histo_file))
        args.append("--output='{}'".format(output))
        return self.run_tool('HistogramMatching', args, callback) # returns 1 if error

    def histogram_matching_two_images(self, input1, input2, output, callback=default_callback):
        """ This tool alters the cumulative distribution function of a raster image to that of another image.

        Keyword arguments:

        input1 -- Input raster file to modify. 
        input2 -- Input reference raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('HistogramMatchingTwoImages', args, callback) # returns 1 if error

    def min_max_contrast_stretch(self, i, output, min_val, max_val, num_tones=256, callback=default_callback):
        """ Performs a min-max contrast stretch on an input greytone image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        min_val -- Lower tail clip value. 
        max_val -- Upper tail clip value. 
        num_tones -- Number of tones in the output image. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--min_val='{}'".format(min_val))
        args.append("--max_val='{}'".format(max_val))
        args.append("--num_tones={}".format(num_tones))
        return self.run_tool('MinMaxContrastStretch', args, callback) # returns 1 if error

    def panchromatic_sharpening(self, pan, output, red=None, green=None, blue=None, composite=None, method="brovey", callback=default_callback):
        """ Increases the spatial resolution of image data by combining multispectral bands with panchromatic data.

        Keyword arguments:

        red -- Input red band image file. Optionally specified if colour-composite not specified. 
        green -- Input green band image file. Optionally specified if colour-composite not specified. 
        blue -- Input blue band image file. Optionally specified if colour-composite not specified. 
        composite -- Input colour-composite image file. Only used if individual bands are not specified. 
        pan -- Input panchromatic band file. 
        output -- Output colour composite file. 
        method -- Options include 'brovey' (default) and 'ihs'. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        if red is not None: args.append("--red='{}'".format(red))
        if green is not None: args.append("--green='{}'".format(green))
        if blue is not None: args.append("--blue='{}'".format(blue))
        if composite is not None: args.append("--composite='{}'".format(composite))
        args.append("--pan='{}'".format(pan))
        args.append("--output='{}'".format(output))
        args.append("--method={}".format(method))
        return self.run_tool('PanchromaticSharpening', args, callback) # returns 1 if error

    def percentage_contrast_stretch(self, i, output, clip=0.0, tail="both", num_tones=256, callback=default_callback):
        """ Performs a percentage linear contrast stretch on input images.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        clip -- Optional amount to clip the distribution tails by, in percent. 
        tail -- Specified which tails to clip; options include 'upper', 'lower', and 'both' (default is 'both'). 
        num_tones -- Number of tones in the output image. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--clip={}".format(clip))
        args.append("--tail={}".format(tail))
        args.append("--num_tones={}".format(num_tones))
        return self.run_tool('PercentageContrastStretch', args, callback) # returns 1 if error

    def sigmoidal_contrast_stretch(self, i, output, cutoff=0.0, gain=1.0, num_tones=256, callback=default_callback):
        """ Performs a sigmoidal contrast stretch on input images.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        cutoff -- Cutoff value between 0.0 and 0.95. 
        gain -- Gain value. 
        num_tones -- Number of tones in the output image. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--cutoff={}".format(cutoff))
        args.append("--gain={}".format(gain))
        args.append("--num_tones={}".format(num_tones))
        return self.run_tool('SigmoidalContrastStretch', args, callback) # returns 1 if error

    def standard_deviation_contrast_stretch(self, i, output, stdev=2.0, num_tones=256, callback=default_callback):
        """ Performs a standard-deviation contrast stretch on input images.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        stdev -- Standard deviation clip value. 
        num_tones -- Number of tones in the output image. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--stdev={}".format(stdev))
        args.append("--num_tones={}".format(num_tones))
        return self.run_tool('StandardDeviationContrastStretch', args, callback) # returns 1 if error

    ###############
    # LiDAR Tools #
    ###############

    def block_maximum(self, i=None, output=None, resolution=1.0, callback=default_callback):
        """ Creates a block-maximum raster from an input LAS file. When the input/output parameters are not specified, the tool grids all LAS files contained within the working directory.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output file. 
        resolution -- Output raster's grid resolution. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        if i is not None: args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        args.append("--resolution={}".format(resolution))
        return self.run_tool('BlockMaximum', args, callback) # returns 1 if error

    def block_minimum(self, i=None, output=None, resolution=1.0, callback=default_callback):
        """ Creates a block-minimum raster from an input LAS file. When the input/output parameters are not specified, the tool grids all LAS files contained within the working directory.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output file. 
        resolution -- Output raster's grid resolution. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        if i is not None: args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        args.append("--resolution={}".format(resolution))
        return self.run_tool('BlockMinimum', args, callback) # returns 1 if error

    def classify_overlap_points(self, i, output, resolution=2.0, filter=False, callback=default_callback):
        """ Classifies or filters LAS point in regions of overlapping flight lines.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        resolution -- The distance of the square area used to evaluate nearby points in the LiDAR data. 
        filter -- Filter out points from overlapping flightlines? If false, overlaps will simply be classified. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--resolution={}".format(resolution))
        if filter: args.append("--filter")
        return self.run_tool('ClassifyOverlapPoints', args, callback) # returns 1 if error

    def clip_lidar_to_polygon(self, i, polygons, output, callback=default_callback):
        """ Clips a LiDAR point cloud to a vector polygon or polygons.

        Keyword arguments:

        i -- Input LiDAR file. 
        polygons -- Input vector polygons file. 
        output -- Output LiDAR file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--polygons='{}'".format(polygons))
        args.append("--output='{}'".format(output))
        return self.run_tool('ClipLidarToPolygon', args, callback) # returns 1 if error

    def erase_polygon_from_lidar(self, i, polygons, output, callback=default_callback):
        """ Erases (cuts out) a vector polygon or polygons from a LiDAR point cloud.

        Keyword arguments:

        i -- Input LiDAR file. 
        polygons -- Input vector polygons file. 
        output -- Output LiDAR file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--polygons='{}'".format(polygons))
        args.append("--output='{}'".format(output))
        return self.run_tool('ErasePolygonFromLidar', args, callback) # returns 1 if error

    def filter_lidar_scan_angles(self, i, output, threshold, callback=default_callback):
        """ Removes points in a LAS file with scan angles greater than a threshold.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        threshold -- Scan angle threshold. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--threshold='{}'".format(threshold))
        return self.run_tool('FilterLidarScanAngles', args, callback) # returns 1 if error

    def find_flightline_edge_points(self, i, output, callback=default_callback):
        """ Identifies points along a flightline's edge in a LAS file.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('FindFlightlineEdgePoints', args, callback) # returns 1 if error

    def flightline_overlap(self, i=None, output=None, resolution=1.0, callback=default_callback):
        """ Reads a LiDAR (LAS) point file and outputs a raster containing the number of overlapping flight lines in each grid cell.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output file. 
        resolution -- Output raster's grid resolution. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        if i is not None: args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        args.append("--resolution={}".format(resolution))
        return self.run_tool('FlightlineOverlap', args, callback) # returns 1 if error

    def las_to_ascii(self, inputs, callback=default_callback):
        """ Converts one or more LAS files into ASCII text files.

        Keyword arguments:

        inputs -- Input LiDAR files. 
callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        return self.run_tool('LasToAscii', args, callback) # returns 1 if error

    def lidar_colourize(self, in_lidar, in_image, output, callback=default_callback):
        """ Adds the red-green-blue colour fields of a LiDAR (LAS) file based on an input image.

        Keyword arguments:

        in_lidar -- Input LiDAR file. 
        in_image -- Input colour image file. 
        output -- Output LiDAR file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--in_lidar='{}'".format(in_lidar))
        args.append("--in_image='{}'".format(in_image))
        args.append("--output='{}'".format(output))
        return self.run_tool('LidarColourize', args, callback) # returns 1 if error

    def lidar_elevation_slice(self, i, output, minz=None, maxz=None, cls=False, inclassval=2, outclassval=1, callback=default_callback):
        """ Outputs all of the points within a LiDAR (LAS) point file that lie between a specified elevation range.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        minz -- Minimum elevation value (optional). 
        maxz -- Maximum elevation value (optional). 
        cls -- Optional boolean flag indicating whether points outside the range should be retained in output but reclassified. 
        inclassval -- Optional parameter specifying the class value assigned to points within the slice. 
        outclassval -- Optional parameter specifying the class value assigned to points within the slice. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if minz is not None: args.append("--minz='{}'".format(minz))
        if maxz is not None: args.append("--maxz='{}'".format(maxz))
        if cls: args.append("--class")
        args.append("--inclassval={}".format(inclassval))
        args.append("--outclassval={}".format(outclassval))
        return self.run_tool('LidarElevationSlice', args, callback) # returns 1 if error

    def lidar_ground_point_filter(self, i, output, radius=2.0, slope_threshold=45.0, height_threshold=1.0, callback=default_callback):
        """ Identifies ground points within LiDAR dataset using a slope-based method.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        radius -- Search Radius. 
        slope_threshold -- Maximum inter-point slope to be considered an off-terrain point. 
        height_threshold -- Inter-point height difference to be considered an off-terrain point. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--radius={}".format(radius))
        args.append("--slope_threshold={}".format(slope_threshold))
        args.append("--height_threshold={}".format(height_threshold))
        return self.run_tool('LidarGroundPointFilter', args, callback) # returns 1 if error

    def lidar_hillshade(self, i, output, azimuth=315.0, altitude=30.0, radius=1.0, callback=default_callback):
        """ Calculates a hillshade value for points within a LAS file and stores these data in the RGB field.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output file. 
        azimuth -- Illumination source azimuth in degrees. 
        altitude -- Illumination source altitude in degrees. 
        radius -- Search Radius. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--azimuth={}".format(azimuth))
        args.append("--altitude={}".format(altitude))
        args.append("--radius={}".format(radius))
        return self.run_tool('LidarHillshade', args, callback) # returns 1 if error

    def lidar_histogram(self, i, output, parameter="elevation", clip=1.0, callback=default_callback):
        """ Creates a histogram from LiDAR data.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        parameter -- Parameter; options are 'elevation' (default), 'intensity', 'scan angle', 'class. 
        clip -- Amount to clip distribution tails (in percent). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--parameter={}".format(parameter))
        args.append("--clip={}".format(clip))
        return self.run_tool('LidarHistogram', args, callback) # returns 1 if error

    def lidar_idw_interpolation(self, i=None, output=None, parameter="elevation", returns="all", resolution=1.0, weight=1.0, radius=2.5, exclude_cls=None, minz=None, maxz=None, callback=default_callback):
        """ Interpolates LAS files using an inverse-distance weighted (IDW) scheme. When the input/output parameters are not specified, the tool interpolates all LAS files contained within the working directory.

        Keyword arguments:

        i -- Input LiDAR file (including extension). 
        output -- Output raster file (including extension). 
        parameter -- Interpolation parameter; options are 'elevation' (default), 'intensity', 'class', 'scan angle', 'user data'. 
        returns -- Point return types to include; options are 'all' (default), 'last', 'first'. 
        resolution -- Output raster's grid resolution. 
        weight -- IDW weight value. 
        radius -- Search Radius. 
        exclude_cls -- Optional exclude classes from interpolation; Valid class values range from 0 to 18, based on LAS specifications. Example, --exclude_cls='3,4,5,6,7,18'. 
        minz -- Optional minimum elevation for inclusion in interpolation. 
        maxz -- Optional maximum elevation for inclusion in interpolation. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        if i is not None: args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        args.append("--parameter={}".format(parameter))
        args.append("--returns={}".format(returns))
        args.append("--resolution={}".format(resolution))
        args.append("--weight={}".format(weight))
        args.append("--radius={}".format(radius))
        if exclude_cls is not None: args.append("--exclude_cls='{}'".format(exclude_cls))
        if minz is not None: args.append("--minz='{}'".format(minz))
        if maxz is not None: args.append("--maxz='{}'".format(maxz))
        return self.run_tool('LidarIdwInterpolation', args, callback) # returns 1 if error

    def lidar_info(self, i, output=None, vlr=False, geokeys=False, callback=default_callback):
        """ Prints information about a LiDAR (LAS) dataset, including header, point return frequency, and classification data and information about the variable length records (VLRs) and geokeys.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output HTML file for summary report. 
        vlr -- Flag indicating whether or not to print the variable length records (VLRs). 
        geokeys -- Flag indicating whether or not to print the geokeys. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        if vlr: args.append("--vlr")
        if geokeys: args.append("--geokeys")
        return self.run_tool('LidarInfo', args, callback) # returns 1 if error

    def lidar_join(self, inputs, output, callback=default_callback):
        """ Joins multiple LiDAR (LAS) files into a single LAS file.

        Keyword arguments:

        inputs -- Input LiDAR files. 
        output -- Output LiDAR file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('LidarJoin', args, callback) # returns 1 if error

    def lidar_kappa_index(self, input1, input2, output, callback=default_callback):
        """ Performs a kappa index of agreement (KIA) analysis on the classifications of two LAS files.

        Keyword arguments:

        input1 -- Input LiDAR classification file. 
        input2 -- Input LiDAR reference file. 
        output -- Output HTML file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('LidarKappaIndex', args, callback) # returns 1 if error

    def lidar_nearest_neighbour_gridding(self, i=None, output=None, parameter="elevation", returns="all", resolution=1.0, radius=2.5, exclude_cls=None, minz=None, maxz=None, callback=default_callback):
        """ Grids LAS files using nearest-neighbour scheme. When the input/output parameters are not specified, the tool grids all LAS files contained within the working directory.

        Keyword arguments:

        i -- Input LiDAR file (including extension). 
        output -- Output raster file (including extension). 
        parameter -- Interpolation parameter; options are 'elevation' (default), 'intensity', 'class', 'scan angle', 'user data'. 
        returns -- Point return types to include; options are 'all' (default), 'last', 'first'. 
        resolution -- Output raster's grid resolution. 
        radius -- Search Radius. 
        exclude_cls -- Optional exclude classes from interpolation; Valid class values range from 0 to 18, based on LAS specifications. Example, --exclude_cls='3,4,5,6,7,18'. 
        minz -- Optional minimum elevation for inclusion in interpolation. 
        maxz -- Optional maximum elevation for inclusion in interpolation. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        if i is not None: args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        args.append("--parameter={}".format(parameter))
        args.append("--returns={}".format(returns))
        args.append("--resolution={}".format(resolution))
        args.append("--radius={}".format(radius))
        if exclude_cls is not None: args.append("--exclude_cls='{}'".format(exclude_cls))
        if minz is not None: args.append("--minz='{}'".format(minz))
        if maxz is not None: args.append("--maxz='{}'".format(maxz))
        return self.run_tool('LidarNearestNeighbourGridding', args, callback) # returns 1 if error

    def lidar_point_density(self, i=None, output=None, returns="all", resolution=1.0, radius=2.5, exclude_cls=None, minz=None, maxz=None, callback=default_callback):
        """ Calculates the spatial pattern of point density for a LiDAR data set. When the input/output parameters are not specified, the tool grids all LAS files contained within the working directory.

        Keyword arguments:

        i -- Input LiDAR file (including extension). 
        output -- Output raster file (including extension). 
        returns -- Point return types to include; options are 'all' (default), 'last', 'first'. 
        resolution -- Output raster's grid resolution. 
        radius -- Search Radius. 
        exclude_cls -- Optional exclude classes from interpolation; Valid class values range from 0 to 18, based on LAS specifications. Example, --exclude_cls='3,4,5,6,7,18'. 
        minz -- Optional minimum elevation for inclusion in interpolation. 
        maxz -- Optional maximum elevation for inclusion in interpolation. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        if i is not None: args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        args.append("--returns={}".format(returns))
        args.append("--resolution={}".format(resolution))
        args.append("--radius={}".format(radius))
        if exclude_cls is not None: args.append("--exclude_cls='{}'".format(exclude_cls))
        if minz is not None: args.append("--minz='{}'".format(minz))
        if maxz is not None: args.append("--maxz='{}'".format(maxz))
        return self.run_tool('LidarPointDensity', args, callback) # returns 1 if error

    def lidar_point_stats(self, i=None, resolution=1.0, num_points=False, num_pulses=False, z_range=False, intensity_range=False, predom_class=False, callback=default_callback):
        """ Creates several rasters summarizing the distribution of LAS point data. When the input/output parameters are not specified, the tool works on all LAS files contained within the working directory.

        Keyword arguments:

        i -- Input LiDAR file. 
        resolution -- Output raster's grid resolution. 
        num_points -- Flag indicating whether or not to output the number of points raster. 
        num_pulses -- Flag indicating whether or not to output the number of pulses raster. 
        z_range -- Flag indicating whether or not to output the elevation range raster. 
        intensity_range -- Flag indicating whether or not to output the intensity range raster. 
        predom_class -- Flag indicating whether or not to output the predominant classification raster. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        if i is not None: args.append("--input='{}'".format(i))
        args.append("--resolution={}".format(resolution))
        if num_points: args.append("--num_points")
        if num_pulses: args.append("--num_pulses")
        if z_range: args.append("--z_range")
        if intensity_range: args.append("--intensity_range")
        if predom_class: args.append("--predom_class")
        return self.run_tool('LidarPointStats', args, callback) # returns 1 if error

    def lidar_remove_duplicates(self, i, output, include_z=False, callback=default_callback):
        """ Removes duplicate points from a LiDAR data set.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        include_z -- Include z-values in point comparison?. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if include_z: args.append("--include_z")
        return self.run_tool('LidarRemoveDuplicates', args, callback) # returns 1 if error

    def lidar_remove_outliers(self, i, output, radius=2.0, elev_diff=50.0, callback=default_callback):
        """ Removes outliers (high and low points) in a LiDAR point cloud.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        radius -- Search Radius. 
        elev_diff -- Max. elevation difference. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--radius={}".format(radius))
        args.append("--elev_diff={}".format(elev_diff))
        return self.run_tool('LidarRemoveOutliers', args, callback) # returns 1 if error

    def lidar_segmentation(self, i, output, radius=5.0, norm_diff=10.0, maxzdiff=1.0, callback=default_callback):
        """ Segments a LiDAR point cloud based on normal vectors.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output file. 
        radius -- Search Radius. 
        norm_diff -- Maximum difference in normal vectors, in degrees. 
        maxzdiff -- Maximum difference in elevation (z units) between neighbouring points of the same segment. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--radius={}".format(radius))
        args.append("--norm_diff={}".format(norm_diff))
        args.append("--maxzdiff={}".format(maxzdiff))
        return self.run_tool('LidarSegmentation', args, callback) # returns 1 if error

    def lidar_segmentation_based_filter(self, i, output, radius=5.0, norm_diff=2.0, maxzdiff=1.0, classify=False, callback=default_callback):
        """ Identifies ground points within LiDAR point clouds using a segmentation based approach.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output file. 
        radius -- Search Radius. 
        norm_diff -- Maximum difference in normal vectors, in degrees. 
        maxzdiff -- Maximum difference in elevation (z units) between neighbouring points of the same segment. 
        classify -- Classify points as ground (2) or off-ground (1). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--radius={}".format(radius))
        args.append("--norm_diff={}".format(norm_diff))
        args.append("--maxzdiff={}".format(maxzdiff))
        if classify: args.append("--classify")
        return self.run_tool('LidarSegmentationBasedFilter', args, callback) # returns 1 if error

    def lidar_tile(self, i, width_x=1000.0, width_y=1000.0, origin_x=0.0, origin_y=0.0, min_points=0, callback=default_callback):
        """ Tiles a LiDAR LAS file into multiple LAS files.

        Keyword arguments:

        i -- Input LiDAR file. 
        width_x -- Width of tiles in the X dimension; default 1000.0. 
        width_y -- Width of tiles in the Y dimension. 
        origin_x -- Origin point X coordinate for tile grid. 
        origin_y -- Origin point Y coordinate for tile grid. 
        min_points -- Minimum number of points contained in a tile for it to be saved. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--width_x={}".format(width_x))
        args.append("--width_y={}".format(width_y))
        args.append("--origin_x={}".format(origin_x))
        args.append("--origin_y={}".format(origin_y))
        args.append("--min_points={}".format(min_points))
        return self.run_tool('LidarTile', args, callback) # returns 1 if error

    def lidar_tophat_transform(self, i, output, radius=1.0, callback=default_callback):
        """ Performs a white top-hat transform on a Lidar dataset; as an estimate of height above ground, this is useful for modelling the vegetation canopy.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        radius -- Search Radius. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--radius={}".format(radius))
        return self.run_tool('LidarTophatTransform', args, callback) # returns 1 if error

    def normal_vectors(self, i, output, radius=1.0, callback=default_callback):
        """ Calculates normal vectors for points within a LAS file and stores these data (XYZ vector components) in the RGB field.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        radius -- Search Radius. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--radius={}".format(radius))
        return self.run_tool('NormalVectors', args, callback) # returns 1 if error

    ########################
    # Math and Stats Tools #
    ########################

    def absolute_value(self, i, output, callback=default_callback):
        """ Calculates the absolute value of every cell in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('AbsoluteValue', args, callback) # returns 1 if error

    def add(self, input1, input2, output, callback=default_callback):
        """ Performs an addition operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Add', args, callback) # returns 1 if error

    def And(self, input1, input2, output, callback=default_callback):
        """ Performs a logical AND operator on two Boolean raster images.

        Keyword arguments:

        input1 -- Input raster file. 
        input2 -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('And', args, callback) # returns 1 if error

    def anova(self, i, features, output, callback=default_callback):
        """ Performs an analysis of variance (ANOVA) test on a raster dataset.

        Keyword arguments:

        i -- Input raster file. 
        features -- Feature definition (or class) raster. 
        output -- Output HTML file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--features='{}'".format(features))
        args.append("--output='{}'".format(output))
        return self.run_tool('Anova', args, callback) # returns 1 if error

    def arc_cos(self, i, output, callback=default_callback):
        """ Returns the inverse cosine (arccos) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('ArcCos', args, callback) # returns 1 if error

    def arc_sin(self, i, output, callback=default_callback):
        """ Returns the inverse sine (arcsin) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('ArcSin', args, callback) # returns 1 if error

    def arc_tan(self, i, output, callback=default_callback):
        """ Returns the inverse tangent (arctan) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('ArcTan', args, callback) # returns 1 if error

    def atan2(self, input_y, input_x, output, callback=default_callback):
        """ Returns the 2-argument inverse tangent (atan2).

        Keyword arguments:

        input_y -- Input y raster file or constant value (rise). 
        input_x -- Input x raster file or constant value (run). 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input_y='{}'".format(input_y))
        args.append("--input_x='{}'".format(input_x))
        args.append("--output='{}'".format(output))
        return self.run_tool('Atan2', args, callback) # returns 1 if error

    def attribute_correlation(self, i, output=None, callback=default_callback):
        """ Performs a correlation analysis on attribute fields from a vector database.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        return self.run_tool('AttributeCorrelation', args, callback) # returns 1 if error

    def attribute_histogram(self, i, field, output, callback=default_callback):
        """ Creates a histogram for the field values of a vector's attribute table.

        Keyword arguments:

        i -- Input raster file. 
        field -- Input field name in attribute table. 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--field='{}'".format(field))
        args.append("--output='{}'".format(output))
        return self.run_tool('AttributeHistogram', args, callback) # returns 1 if error

    def attribute_scattergram(self, i, fieldx, fieldy, output, trendline=False, callback=default_callback):
        """ Creates a scattergram for two field values of a vector's attribute table.

        Keyword arguments:

        i -- Input raster file. 
        fieldx -- Input field name in attribute table for the x-axis. 
        fieldy -- Input field name in attribute table for the y-axis. 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        trendline -- Draw the trendline. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--fieldx='{}'".format(fieldx))
        args.append("--fieldy='{}'".format(fieldy))
        args.append("--output='{}'".format(output))
        if trendline: args.append("--trendline")
        return self.run_tool('AttributeScattergram', args, callback) # returns 1 if error

    def ceil(self, i, output, callback=default_callback):
        """ Returns the smallest (closest to negative infinity) value that is greater than or equal to the values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('Ceil', args, callback) # returns 1 if error

    def cos(self, i, output, callback=default_callback):
        """ Returns the cosine (cos) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('Cos', args, callback) # returns 1 if error

    def cosh(self, i, output, callback=default_callback):
        """ Returns the hyperbolic cosine (cosh) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('Cosh', args, callback) # returns 1 if error

    def crispness_index(self, i, output=None, callback=default_callback):
        """ Calculates the Crispness Index, which is used to quantify how crisp (or conversely how fuzzy) a probability image is.

        Keyword arguments:

        i -- Input raster file. 
        output -- Optional output html file (default name will be based on input file if unspecified). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        return self.run_tool('CrispnessIndex', args, callback) # returns 1 if error

    def cross_tabulation(self, input1, input2, output, callback=default_callback):
        """ Performs a cross-tabulation on two categorical images.

        Keyword arguments:

        input1 -- Input raster file 1. 
        input2 -- Input raster file 1. 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('CrossTabulation', args, callback) # returns 1 if error

    def cumulative_distribution(self, i, output, callback=default_callback):
        """ Converts a raster image to its cumulative distribution function.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('CumulativeDistribution', args, callback) # returns 1 if error

    def decrement(self, i, output, callback=default_callback):
        """ Decreases the values of each grid cell in an input raster by 1.0 (see also InPlaceSubtract).

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('Decrement', args, callback) # returns 1 if error

    def divide(self, input1, input2, output, callback=default_callback):
        """ Performs a division operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Divide', args, callback) # returns 1 if error

    def equal_to(self, input1, input2, output, callback=default_callback):
        """ Performs a equal-to comparison operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('EqualTo', args, callback) # returns 1 if error

    def exp(self, i, output, callback=default_callback):
        """ Returns the exponential (base e) of values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('Exp', args, callback) # returns 1 if error

    def exp2(self, i, output, callback=default_callback):
        """ Returns the exponential (base 2) of values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('Exp2', args, callback) # returns 1 if error

    def extract_raster_statistics(self, i, features, output=None, stat="average", out_table=None, callback=default_callback):
        """ Extracts descriptive statistics for a group of patches in a raster.

        Keyword arguments:

        i -- Input data raster file. 
        features -- Input feature definition raster file. 
        output -- Output raster file. 
        stat -- Statistic to extract. 
        out_table -- Output HTML Table file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--features='{}'".format(features))
        if output is not None: args.append("--output='{}'".format(output))
        args.append("--stat={}".format(stat))
        if out_table is not None: args.append("--out_table='{}'".format(out_table))
        return self.run_tool('ExtractRasterStatistics', args, callback) # returns 1 if error

    def floor(self, i, output, callback=default_callback):
        """ Returns the largest (closest to positive infinity) value that is less than or equal to the values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('Floor', args, callback) # returns 1 if error

    def greater_than(self, input1, input2, output, incl_equals=False, callback=default_callback):
        """ Performs a greater-than comparison operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        incl_equals -- Perform a greater-than-or-equal-to operation. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        if incl_equals: args.append("--incl_equals")
        return self.run_tool('GreaterThan', args, callback) # returns 1 if error

    def image_autocorrelation(self, inputs, output, contiguity="Rook", callback=default_callback):
        """ Performs Moran's I analysis on two or more input images.

        Keyword arguments:

        inputs -- Input raster files. 
        contiguity -- Contiguity type. 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--contiguity={}".format(contiguity))
        args.append("--output='{}'".format(output))
        return self.run_tool('ImageAutocorrelation', args, callback) # returns 1 if error

    def image_correlation(self, inputs, output=None, callback=default_callback):
        """ Performs image correlation on two or more input images.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        if output is not None: args.append("--output='{}'".format(output))
        return self.run_tool('ImageCorrelation', args, callback) # returns 1 if error

    def image_regression(self, input1, input2, output, out_residuals=None, standardize=False, callback=default_callback):
        """ Performs image regression analysis on two input images.

        Keyword arguments:

        input1 -- Input raster file (independent variable, X). 
        input2 -- Input raster file (dependent variable, Y). 
        output -- Output HTML file for regression summary report. 
        out_residuals -- Output raster regression resdidual file. 
        standardize -- Optional flag indicating whether to standardize the residuals map. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        if out_residuals is not None: args.append("--out_residuals='{}'".format(out_residuals))
        if standardize: args.append("--standardize")
        return self.run_tool('ImageRegression', args, callback) # returns 1 if error

    def in_place_add(self, input1, input2, callback=default_callback):
        """ Performs an in-place addition operation (input1 += input2).

        Keyword arguments:

        input1 -- Input raster file. 
        input2 -- Input raster file or constant value. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        return self.run_tool('InPlaceAdd', args, callback) # returns 1 if error

    def in_place_divide(self, input1, input2, callback=default_callback):
        """ Performs an in-place division operation (input1 /= input2).

        Keyword arguments:

        input1 -- Input raster file. 
        input2 -- Input raster file or constant value. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        return self.run_tool('InPlaceDivide', args, callback) # returns 1 if error

    def in_place_multiply(self, input1, input2, callback=default_callback):
        """ Performs an in-place multiplication operation (input1 *= input2).

        Keyword arguments:

        input1 -- Input raster file. 
        input2 -- Input raster file or constant value. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        return self.run_tool('InPlaceMultiply', args, callback) # returns 1 if error

    def in_place_subtract(self, input1, input2, callback=default_callback):
        """ Performs an in-place subtraction operation (input1 -= input2).

        Keyword arguments:

        input1 -- Input raster file. 
        input2 -- Input raster file or constant value. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        return self.run_tool('InPlaceSubtract', args, callback) # returns 1 if error

    def increment(self, i, output, callback=default_callback):
        """ Increases the values of each grid cell in an input raster by 1.0. (see also InPlaceAdd).

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('Increment', args, callback) # returns 1 if error

    def integer_division(self, input1, input2, output, callback=default_callback):
        """ Performs an integer division operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('IntegerDivision', args, callback) # returns 1 if error

    def is_no_data(self, i, output, callback=default_callback):
        """ Identifies NoData valued pixels in an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('IsNoData', args, callback) # returns 1 if error

    def ks_test_for_normality(self, i, output, num_samples=None, callback=default_callback):
        """ Evaluates whether the values in a raster are normally distributed.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output HTML file. 
        num_samples -- Number of samples. Leave blank to use whole image. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if num_samples is not None: args.append("--num_samples='{}'".format(num_samples))
        return self.run_tool('KSTestForNormality', args, callback) # returns 1 if error

    def kappa_index(self, input1, input2, output, callback=default_callback):
        """ Performs a kappa index of agreement (KIA) analysis on two categorical raster files.

        Keyword arguments:

        input1 -- Input classification raster file. 
        input2 -- Input reference raster file. 
        output -- Output HTML file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('KappaIndex', args, callback) # returns 1 if error

    def less_than(self, input1, input2, output, incl_equals=False, callback=default_callback):
        """ Performs a less-than comparison operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        incl_equals -- Perform a less-than-or-equal-to operation. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        if incl_equals: args.append("--incl_equals")
        return self.run_tool('LessThan', args, callback) # returns 1 if error

    def list_unique_values(self, i, field, output, callback=default_callback):
        """ Lists the unique values contained in a field witin a vector's attribute table.

        Keyword arguments:

        i -- Input raster file. 
        field -- Input field name in attribute table. 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--field='{}'".format(field))
        args.append("--output='{}'".format(output))
        return self.run_tool('ListUniqueValues', args, callback) # returns 1 if error

    def ln(self, i, output, callback=default_callback):
        """ Returns the natural logarithm of values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('Ln', args, callback) # returns 1 if error

    def log10(self, i, output, callback=default_callback):
        """ Returns the base-10 logarithm of values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('Log10', args, callback) # returns 1 if error

    def log2(self, i, output, callback=default_callback):
        """ Returns the base-2 logarithm of values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('Log2', args, callback) # returns 1 if error

    def max(self, input1, input2, output, callback=default_callback):
        """ Performs a MAX operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Max', args, callback) # returns 1 if error

    def min(self, input1, input2, output, callback=default_callback):
        """ Performs a MIN operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Min', args, callback) # returns 1 if error

    def modulo(self, input1, input2, output, callback=default_callback):
        """ Performs a modulo operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Modulo', args, callback) # returns 1 if error

    def multiply(self, input1, input2, output, callback=default_callback):
        """ Performs a multiplication operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Multiply', args, callback) # returns 1 if error

    def negate(self, i, output, callback=default_callback):
        """ Changes the sign of values in a raster or the 0-1 values of a Boolean raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('Negate', args, callback) # returns 1 if error

    def Not(self, input1, input2, output, callback=default_callback):
        """ Performs a logical NOT operator on two Boolean raster images.

        Keyword arguments:

        input1 -- Input raster file. 
        input2 -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Not', args, callback) # returns 1 if error

    def not_equal_to(self, input1, input2, output, callback=default_callback):
        """ Performs a not-equal-to comparison operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('NotEqualTo', args, callback) # returns 1 if error

    def Or(self, input1, input2, output, callback=default_callback):
        """ Performs a logical OR operator on two Boolean raster images.

        Keyword arguments:

        input1 -- Input raster file. 
        input2 -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Or', args, callback) # returns 1 if error

    def power(self, input1, input2, output, callback=default_callback):
        """ Raises the values in grid cells of one rasters, or a constant value, by values in another raster or constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Power', args, callback) # returns 1 if error

    def principal_component_analysis(self, inputs, out_html, num_comp=None, standardized=False, callback=default_callback):
        """ Performs a principal component analysis (PCA) on a multi-spectral dataset.

        Keyword arguments:

        inputs -- Input raster files. 
        out_html -- Output HTML report file. 
        num_comp -- Number of component images to output; <= to num. input images. 
        standardized -- Perform standardized PCA?. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--out_html='{}'".format(out_html))
        if num_comp is not None: args.append("--num_comp='{}'".format(num_comp))
        if standardized: args.append("--standardized")
        return self.run_tool('PrincipalComponentAnalysis', args, callback) # returns 1 if error

    def quantiles(self, i, output, num_quantiles=4, callback=default_callback):
        """ Transforms raster values into quantiles.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        num_quantiles -- Number of quantiles. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--num_quantiles={}".format(num_quantiles))
        return self.run_tool('Quantiles', args, callback) # returns 1 if error

    def random_field(self, base, output, callback=default_callback):
        """ Creates an image containing random values.

        Keyword arguments:

        base -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--base='{}'".format(base))
        args.append("--output='{}'".format(output))
        return self.run_tool('RandomField', args, callback) # returns 1 if error

    def random_sample(self, base, output, num_samples=1000, callback=default_callback):
        """ Creates an image containing randomly located sample grid cells with unique IDs.

        Keyword arguments:

        base -- Input raster file. 
        output -- Output raster file. 
        num_samples -- Number of samples. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--base='{}'".format(base))
        args.append("--output='{}'".format(output))
        args.append("--num_samples={}".format(num_samples))
        return self.run_tool('RandomSample', args, callback) # returns 1 if error

    def raster_histogram(self, i, output, callback=default_callback):
        """ Creates a histogram from raster values.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('RasterHistogram', args, callback) # returns 1 if error

    def raster_summary_stats(self, i, callback=default_callback):
        """ Measures a rasters average, standard deviation, num. non-nodata cells, and total.

        Keyword arguments:

        i -- Input raster file. 
callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        return self.run_tool('RasterSummaryStats', args, callback) # returns 1 if error

    def reciprocal(self, i, output, callback=default_callback):
        """ Returns the reciprocal (i.e. 1 / z) of values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('Reciprocal', args, callback) # returns 1 if error

    def rescale_value_range(self, i, output, out_min_val, out_max_val, clip_min=None, clip_max=None, callback=default_callback):
        """ Performs a min-max contrast stretch on an input greytone image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        out_min_val -- New minimum value in output image. 
        out_max_val -- New maximum value in output image. 
        clip_min -- Optional lower tail clip value. 
        clip_max -- Optional upper tail clip value. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--out_min_val='{}'".format(out_min_val))
        args.append("--out_max_val='{}'".format(out_max_val))
        if clip_min is not None: args.append("--clip_min='{}'".format(clip_min))
        if clip_max is not None: args.append("--clip_max='{}'".format(clip_max))
        return self.run_tool('RescaleValueRange', args, callback) # returns 1 if error

    def root_mean_square_error(self, i, base, callback=default_callback):
        """ Calculates the RMSE and other accuracy statistics.

        Keyword arguments:

        i -- Input raster file. 
        base -- Input base raster file used for comparison. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--base='{}'".format(base))
        return self.run_tool('RootMeanSquareError', args, callback) # returns 1 if error

    def round(self, i, output, callback=default_callback):
        """ Rounds the values in an input raster to the nearest integer value.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('Round', args, callback) # returns 1 if error

    def sin(self, i, output, callback=default_callback):
        """ Returns the sine (sin) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('Sin', args, callback) # returns 1 if error

    def sinh(self, i, output, callback=default_callback):
        """ Returns the hyperbolic sine (sinh) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('Sinh', args, callback) # returns 1 if error

    def square(self, i, output, callback=default_callback):
        """ Squares the values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('Square', args, callback) # returns 1 if error

    def square_root(self, i, output, callback=default_callback):
        """ Returns the square root of the values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('SquareRoot', args, callback) # returns 1 if error

    def subtract(self, input1, input2, output, callback=default_callback):
        """ Performs a differencing operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Subtract', args, callback) # returns 1 if error

    def tan(self, i, output, callback=default_callback):
        """ Returns the tangent (tan) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('Tan', args, callback) # returns 1 if error

    def tanh(self, i, output, callback=default_callback):
        """ Returns the hyperbolic tangent (tanh) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('Tanh', args, callback) # returns 1 if error

    def to_degrees(self, i, output, callback=default_callback):
        """ Converts a raster from radians to degrees.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('ToDegrees', args, callback) # returns 1 if error

    def to_radians(self, i, output, callback=default_callback):
        """ Converts a raster from degrees to radians.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('ToRadians', args, callback) # returns 1 if error

    def trend_surface(self, i, output, order=1, callback=default_callback):
        """ Estimates the trend surface of an input raster file.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        order -- Polynomial order (1 to 10). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--order={}".format(order))
        return self.run_tool('TrendSurface', args, callback) # returns 1 if error

    def trend_surface_vector_points(self, i, field, output, cell_size, order=1, callback=default_callback):
        """ Estimates a trend surface from vector points.

        Keyword arguments:

        i -- Input vector Points file. 
        field -- Input field name in attribute table. 
        output -- Output raster file. 
        order -- Polynomial order (1 to 10). 
        cell_size -- Optionally specified cell size of output raster. Not used when base raster is specified. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--field='{}'".format(field))
        args.append("--output='{}'".format(output))
        args.append("--order={}".format(order))
        args.append("--cell_size='{}'".format(cell_size))
        return self.run_tool('TrendSurfaceVectorPoints', args, callback) # returns 1 if error

    def truncate(self, i, output, num_decimals=None, callback=default_callback):
        """ Truncates the values in a raster to the desired number of decimal places.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        num_decimals -- Number of decimals left after truncation (default is zero). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if num_decimals is not None: args.append("--num_decimals='{}'".format(num_decimals))
        return self.run_tool('Truncate', args, callback) # returns 1 if error

    def turning_bands_simulation(self, base, output, range, iterations=1000, callback=default_callback):
        """ Creates an image containing random values based on a turning-bands simulation.

        Keyword arguments:

        base -- Input base raster file. 
        output -- Output file. 
        range -- The field's range, in xy-units, related to the extent of spatial autocorrelation. 
        iterations -- The number of iterations. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--base='{}'".format(base))
        args.append("--output='{}'".format(output))
        args.append("--range='{}'".format(range))
        args.append("--iterations={}".format(iterations))
        return self.run_tool('TurningBandsSimulation', args, callback) # returns 1 if error

    def xor(self, input1, input2, output, callback=default_callback):
        """ Performs a logical XOR operator on two Boolean raster images.

        Keyword arguments:

        input1 -- Input raster file. 
        input2 -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Xor', args, callback) # returns 1 if error

    def z_scores(self, i, output, callback=default_callback):
        """ Standardizes the values in an input raster by converting to z-scores.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('ZScores', args, callback) # returns 1 if error

    ###########################
    # Stream Network Analysis #
    ###########################

    def distance_to_outlet(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        """ Calculates the distance of stream grid cells to the channel network outlet cell.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('DistanceToOutlet', args, callback) # returns 1 if error

    def extract_streams(self, flow_accum, output, threshold, zero_background=False, callback=default_callback):
        """ Extracts stream grid cells from a flow accumulation raster.

        Keyword arguments:

        flow_accum -- Input raster D8 flow accumulation file. 
        output -- Output raster file. 
        threshold -- Threshold in flow accumulation values for channelization. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--flow_accum='{}'".format(flow_accum))
        args.append("--output='{}'".format(output))
        args.append("--threshold='{}'".format(threshold))
        if zero_background: args.append("--zero_background")
        return self.run_tool('ExtractStreams', args, callback) # returns 1 if error

    def extract_valleys(self, dem, output, variant="Lower Quartile", line_thin=True, filter=5, callback=default_callback):
        """ Identifies potential valley bottom grid cells based on local topolography alone.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        variant -- Options include 'lq' (lower quartile), 'JandR' (Johnston and Rosenfeld), and 'PandD' (Peucker and Douglas); default is 'lq'. 
        line_thin -- Optional flag indicating whether post-processing line-thinning should be performed. 
        filter -- Optional argument (only used when variant='lq') providing the filter size, in grid cells, used for lq-filtering (default is 5). 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--variant={}".format(variant))
        if line_thin: args.append("--line_thin")
        args.append("--filter={}".format(filter))
        return self.run_tool('ExtractValleys', args, callback) # returns 1 if error

    def farthest_channel_head(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        """ Calculates the distance to the furthest upstream channel head for each stream cell.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('FarthestChannelHead', args, callback) # returns 1 if error

    def find_main_stem(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        """ Finds the main stem, based on stream lengths, of each stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('FindMainStem', args, callback) # returns 1 if error

    def hack_stream_order(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        """ Assigns the Hack stream order to each tributary in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('HackStreamOrder', args, callback) # returns 1 if error

    def horton_stream_order(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        """ Assigns the Horton stream order to each tributary in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('HortonStreamOrder', args, callback) # returns 1 if error

    def length_of_upstream_channels(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        """ Calculates the total length of channels upstream.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('LengthOfUpstreamChannels', args, callback) # returns 1 if error

    def long_profile(self, d8_pntr, streams, dem, output, esri_pntr=False, callback=default_callback):
        """ Plots the stream longitudinal profiles for one or more rivers.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        dem -- Input raster DEM file. 
        output -- Output HTML file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('LongProfile', args, callback) # returns 1 if error

    def long_profile_from_points(self, d8_pntr, points, dem, output, esri_pntr=False, callback=default_callback):
        """ Plots the longitudinal profiles from flow-paths initiating from a set of vector points.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        points -- Input vector points file. 
        dem -- Input raster DEM file. 
        output -- Output HTML file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--points='{}'".format(points))
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('LongProfileFromPoints', args, callback) # returns 1 if error

    def rasterize_streams(self, streams, base, output, nodata=True, feature_id=False, callback=default_callback):
        """ Rasterizes vector streams based on Lindsay (2016) method.

        Keyword arguments:

        streams -- Input vector streams file. 
        base -- Input base raster file. 
        output -- Output raster file. 
        nodata -- Use NoData value for background?. 
        feature_id -- Use feature number as output value?. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--streams='{}'".format(streams))
        args.append("--base='{}'".format(base))
        args.append("--output='{}'".format(output))
        if nodata: args.append("--nodata")
        if feature_id: args.append("--feature_id")
        return self.run_tool('RasterizeStreams', args, callback) # returns 1 if error

    def remove_short_streams(self, d8_pntr, streams, output, min_length, esri_pntr=False, callback=default_callback):
        """ Removes short first-order streams from a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        min_length -- Minimum tributary length (in map units) used for network prunning. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        args.append("--min_length='{}'".format(min_length))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('RemoveShortStreams', args, callback) # returns 1 if error

    def shreve_stream_magnitude(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        """ Assigns the Shreve stream magnitude to each link in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('ShreveStreamMagnitude', args, callback) # returns 1 if error

    def strahler_stream_order(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        """ Assigns the Strahler stream order to each link in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('StrahlerStreamOrder', args, callback) # returns 1 if error

    def stream_link_class(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        """ Identifies the exterior/interior links and nodes in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('StreamLinkClass', args, callback) # returns 1 if error

    def stream_link_identifier(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        """ Assigns a unique identifier to each link in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('StreamLinkIdentifier', args, callback) # returns 1 if error

    def stream_link_length(self, d8_pntr, linkid, output, esri_pntr=False, zero_background=False, callback=default_callback):
        """ Estimates the length of each link (or tributary) in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        linkid -- Input raster streams link ID (or tributary ID) file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--linkid='{}'".format(linkid))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('StreamLinkLength', args, callback) # returns 1 if error

    def stream_link_slope(self, d8_pntr, linkid, dem, output, esri_pntr=False, zero_background=False, callback=default_callback):
        """ Estimates the average slope of each link (or tributary) in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        linkid -- Input raster streams link ID (or tributary ID) file. 
        dem -- Input raster DEM file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--linkid='{}'".format(linkid))
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('StreamLinkSlope', args, callback) # returns 1 if error

    def stream_slope_continuous(self, d8_pntr, streams, dem, output, esri_pntr=False, zero_background=False, callback=default_callback):
        """ Estimates the slope of each grid cell in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        dem -- Input raster DEM file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('StreamSlopeContinuous', args, callback) # returns 1 if error

    def topological_stream_order(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        """ Assigns each link in a stream network its topological order.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('TopologicalStreamOrder', args, callback) # returns 1 if error

    def tributary_identifier(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        """ Assigns a unique identifier to each tributary in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom functon for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('TributaryIdentifier', args, callback) # returns 1 if error
