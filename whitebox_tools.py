#!/usr/bin/env python3
''' This file is intended to be a helper for running whitebox-tools plugins from a Python script.
See whitebox_example.py for an example of how to use it.
'''

# This script is part of the WhiteboxTools geospatial library.
# Authors: Dr. John Lindsay
# Created: 28/11/2017
# Last Modified: 09/12/2019
# License: MIT

from __future__ import print_function
import os
from os import path
import sys
import platform
import re
# import shutil
from subprocess import CalledProcessError, Popen, PIPE, STDOUT


def default_callback(value):
    ''' 
    A simple default callback that outputs using the print function. When
    tools are called without providing a custom callback, this function
    will be used to print to standard output.
    '''
    print(value)


def to_camelcase(name):
    '''
    Convert snake_case name to CamelCase name 
    '''
    return ''.join(x.title() for x in name.split('_'))


def to_snakecase(name):
    '''
    Convert CamelCase name to snake_case name 
    '''
    s1 = re.sub('(.)([A-Z][a-z]+)', r'\1_\2', name)
    return re.sub('([a-z0-9])([A-Z])', r'\1_\2', s1).lower()


class WhiteboxTools(object):
    ''' 
    An object for interfacing with the WhiteboxTools executable.
    '''

    def __init__(self):
        if platform.system() == 'Windows':
            self.ext = '.exe'
        else:
            self.ext = ''
        self.exe_name = "whitebox_tools{}".format(self.ext)
        # self.exe_path = os.path.dirname(shutil.which(
        #     self.exe_name) or path.dirname(path.abspath(__file__)))
        # self.exe_path = os.path.dirname(os.path.join(os.path.realpath(__file__)))
        self.exe_path = path.dirname(path.abspath(__file__))
        self.work_dir = ""
        self.verbose = True
        self.cancel_op = False
        self.default_callback = default_callback

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

    def run_tool(self, tool_name, args, callback=None):
        ''' 
        Runs a tool and specifies tool arguments.
        Returns 0 if completes without error.
        Returns 1 if error encountered (details are sent to callback).
        Returns 2 if process is cancelled by user.
        '''
        try:
            if callback is None:
                callback = self.default_callback

            os.chdir(self.exe_path)
            args2 = []
            args2.append("." + path.sep + self.exe_name)
            args2.append("--run=\"{}\"".format(to_camelcase(tool_name)))

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
            args.append("--toolhelp={}".format(to_camelcase(tool_name)))

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
            args.append("--toolparameters={}".format(to_camelcase(tool_name)))

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
            args.append("--toolbox={}".format(to_camelcase(tool_name)))

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
            args.append("--viewcode={}".format(to_camelcase(tool_name)))

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
            ret = {}
            line = proc.stdout.readline()  # skip number of available tools header
            while True:
                line = proc.stdout.readline()
                if line != '':
                    if line.strip() != '':
                        name, descr = line.split(':')
                        ret[to_snakecase(name.strip())] = descr.strip()
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

    def add_point_coordinates_to_table(self, i, callback=None):
        """Modifies the attribute table of a point vector by adding fields containing each point's X and Y coordinates.

        Keyword arguments:

        i -- Input vector Points file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        return self.run_tool('add_point_coordinates_to_table', args, callback) # returns 1 if error

    def clean_vector(self, i, output, callback=None):
        """Removes null features and lines/polygons with fewer than the required number of vertices.

        Keyword arguments:

        i -- Input vector file. 
        output -- Output vector file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('clean_vector', args, callback) # returns 1 if error

    def convert_nodata_to_zero(self, i, output, callback=None):
        """Converts nodata values in a raster to zero.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('convert_nodata_to_zero', args, callback) # returns 1 if error

    def convert_raster_format(self, i, output, callback=None):
        """Converts raster data from one format to another.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('convert_raster_format', args, callback) # returns 1 if error

    def csv_points_to_vector(self, i, output, xfield=0, yfield=1, epsg=None, callback=None):
        """Converts a CSV text file to vector points.

        Keyword arguments:

        i -- Input CSV file (i.e. source of data to be imported). 
        output -- Output vector file. 
        xfield -- X field number (e.g. 0 for first field). 
        yfield -- Y field number (e.g. 1 for second field). 
        epsg -- EPSG projection (e.g. 2958). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--xfield={}".format(xfield))
        args.append("--yfield={}".format(yfield))
        if epsg is not None: args.append("--epsg='{}'".format(epsg))
        return self.run_tool('csv_points_to_vector', args, callback) # returns 1 if error

    def export_table_to_csv(self, i, output, headers=True, callback=None):
        """Exports an attribute table to a CSV text file.

        Keyword arguments:

        i -- Input vector file. 
        output -- Output raster file. 
        headers -- Export field names as file header?. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if headers: args.append("--headers")
        return self.run_tool('export_table_to_csv', args, callback) # returns 1 if error

    def join_tables(self, input1, pkey, input2, fkey, import_field, callback=None):
        """Merge a vector's attribute table with another table based on a common field.

        Keyword arguments:

        input1 -- Input primary vector file (i.e. the table to be modified). 
        pkey -- Primary key field. 
        input2 -- Input foreign vector file (i.e. source of data to be imported). 
        fkey -- Foreign key field. 
        import_field -- Imported field (all fields will be imported if not specified). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--pkey='{}'".format(pkey))
        args.append("--input2='{}'".format(input2))
        args.append("--fkey='{}'".format(fkey))
        args.append("--import_field='{}'".format(import_field))
        return self.run_tool('join_tables', args, callback) # returns 1 if error

    def lines_to_polygons(self, i, output, callback=None):
        """Converts vector polylines to polygons.

        Keyword arguments:

        i -- Input vector line file. 
        output -- Output vector polygon file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('lines_to_polygons', args, callback) # returns 1 if error

    def merge_table_with_csv(self, i, pkey, csv, fkey, import_field=None, callback=None):
        """Merge a vector's attribute table with a table contained within a CSV text file.

        Keyword arguments:

        i -- Input primary vector file (i.e. the table to be modified). 
        pkey -- Primary key field. 
        csv -- Input CSV file (i.e. source of data to be imported). 
        fkey -- Foreign key field. 
        import_field -- Imported field (all fields will be imported if not specified). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--pkey='{}'".format(pkey))
        args.append("--csv='{}'".format(csv))
        args.append("--fkey='{}'".format(fkey))
        if import_field is not None: args.append("--import_field='{}'".format(import_field))
        return self.run_tool('merge_table_with_csv', args, callback) # returns 1 if error

    def merge_vectors(self, inputs, output, callback=None):
        """Combines two or more input vectors of the same ShapeType creating a single, new output vector.

        Keyword arguments:

        inputs -- Input vector files. 
        output -- Output vector file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('merge_vectors', args, callback) # returns 1 if error

    def modify_no_data_value(self, i, new_value="-32768.0", callback=None):
        """Converts nodata values in a raster to zero.

        Keyword arguments:

        i -- Input raster file. 
        new_value -- New NoData value. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--new_value={}".format(new_value))
        return self.run_tool('modify_no_data_value', args, callback) # returns 1 if error

    def multi_part_to_single_part(self, i, output, exclude_holes=True, callback=None):
        """Converts a vector file containing multi-part features into a vector containing only single-part features.

        Keyword arguments:

        i -- Input vector line or polygon file. 
        output -- Output vector line or polygon file. 
        exclude_holes -- Exclude hole parts from the feature splitting? (holes will continue to belong to their features in output.). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if exclude_holes: args.append("--exclude_holes")
        return self.run_tool('multi_part_to_single_part', args, callback) # returns 1 if error

    def new_raster_from_base(self, base, output, value="nodata", data_type="float", callback=None):
        """Creates a new raster using a base image.

        Keyword arguments:

        base -- Input base raster file. 
        output -- Output raster file. 
        value -- Constant value to fill raster with; either 'nodata' or numeric value. 
        data_type -- Output raster data type; options include 'double' (64-bit), 'float' (32-bit), and 'integer' (signed 16-bit) (default is 'float'). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--base='{}'".format(base))
        args.append("--output='{}'".format(output))
        args.append("--value={}".format(value))
        args.append("--data_type={}".format(data_type))
        return self.run_tool('new_raster_from_base', args, callback) # returns 1 if error

    def polygons_to_lines(self, i, output, callback=None):
        """Converts vector polygons to polylines.

        Keyword arguments:

        i -- Input vector polygon file. 
        output -- Output vector lines file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('polygons_to_lines', args, callback) # returns 1 if error

    def print_geo_tiff_tags(self, i, callback=None):
        """Prints the tags within a GeoTIFF.

        Keyword arguments:

        i -- Input GeoTIFF file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        return self.run_tool('print_geo_tiff_tags', args, callback) # returns 1 if error

    def raster_to_vector_lines(self, i, output, callback=None):
        """Converts a raster lines features into a vector of the POLYLINE shapetype.

        Keyword arguments:

        i -- Input raster lines file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('raster_to_vector_lines', args, callback) # returns 1 if error

    def raster_to_vector_points(self, i, output, callback=None):
        """Converts a raster dataset to a vector of the POINT shapetype.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output vector points file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('raster_to_vector_points', args, callback) # returns 1 if error

    def raster_to_vector_polygons(self, i, output, callback=None):
        """Converts a raster dataset to a vector of the POLYGON shapetype.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output vector polygons file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('raster_to_vector_polygons', args, callback) # returns 1 if error

    def reinitialize_attribute_table(self, i, callback=None):
        """Reinitializes a vector's attribute table deleting all fields but the feature ID (FID).

        Keyword arguments:

        i -- Input vector file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        return self.run_tool('reinitialize_attribute_table', args, callback) # returns 1 if error

    def remove_polygon_holes(self, i, output, callback=None):
        """Removes holes within the features of a vector polygon file.

        Keyword arguments:

        i -- Input vector polygon file. 
        output -- Output vector polygon file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('remove_polygon_holes', args, callback) # returns 1 if error

    def set_nodata_value(self, i, output, back_value=0.0, callback=None):
        """Assign a specified value in an input image to the NoData value.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        back_value -- Background value to set to nodata. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--back_value={}".format(back_value))
        return self.run_tool('set_nodata_value', args, callback) # returns 1 if error

    def single_part_to_multi_part(self, i, output, field=None, callback=None):
        """Converts a vector file containing multi-part features into a vector containing only single-part features.

        Keyword arguments:

        i -- Input vector line or polygon file. 
        field -- Grouping ID field name in attribute table. 
        output -- Output vector line or polygon file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        if field is not None: args.append("--field='{}'".format(field))
        args.append("--output='{}'".format(output))
        return self.run_tool('single_part_to_multi_part', args, callback) # returns 1 if error

    def vector_lines_to_raster(self, i, output, field="FID", nodata=True, cell_size=None, base=None, callback=None):
        """Converts a vector containing polylines into a raster.

        Keyword arguments:

        i -- Input vector lines file. 
        field -- Input field name in attribute table. 
        output -- Output raster file. 
        nodata -- Background value to set to NoData. Without this flag, it will be set to 0.0. 
        cell_size -- Optionally specified cell size of output raster. Not used when base raster is specified. 
        base -- Optionally specified input base raster file. Not used when a cell size is specified. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--field={}".format(field))
        args.append("--output='{}'".format(output))
        if nodata: args.append("--nodata")
        if cell_size is not None: args.append("--cell_size='{}'".format(cell_size))
        if base is not None: args.append("--base='{}'".format(base))
        return self.run_tool('vector_lines_to_raster', args, callback) # returns 1 if error

    def vector_points_to_raster(self, i, output, field="FID", assign="last", nodata=True, cell_size=None, base=None, callback=None):
        """Converts a vector containing points into a raster.

        Keyword arguments:

        i -- Input vector Points file. 
        field -- Input field name in attribute table. 
        output -- Output raster file. 
        assign -- Assignment operation, where multiple points are in the same grid cell; options include 'first', 'last' (default), 'min', 'max', 'sum'. 
        nodata -- Background value to set to NoData. Without this flag, it will be set to 0.0. 
        cell_size -- Optionally specified cell size of output raster. Not used when base raster is specified. 
        base -- Optionally specified input base raster file. Not used when a cell size is specified. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--field={}".format(field))
        args.append("--output='{}'".format(output))
        args.append("--assign={}".format(assign))
        if nodata: args.append("--nodata")
        if cell_size is not None: args.append("--cell_size='{}'".format(cell_size))
        if base is not None: args.append("--base='{}'".format(base))
        return self.run_tool('vector_points_to_raster', args, callback) # returns 1 if error

    def vector_polygons_to_raster(self, i, output, field="FID", nodata=True, cell_size=None, base=None, callback=None):
        """Converts a vector containing polygons into a raster.

        Keyword arguments:

        i -- Input vector polygons file. 
        field -- Input field name in attribute table. 
        output -- Output raster file. 
        nodata -- Background value to set to NoData. Without this flag, it will be set to 0.0. 
        cell_size -- Optionally specified cell size of output raster. Not used when base raster is specified. 
        base -- Optionally specified input base raster file. Not used when a cell size is specified. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--field={}".format(field))
        args.append("--output='{}'".format(output))
        if nodata: args.append("--nodata")
        if cell_size is not None: args.append("--cell_size='{}'".format(cell_size))
        if base is not None: args.append("--base='{}'".format(base))
        return self.run_tool('vector_polygons_to_raster', args, callback) # returns 1 if error

    ################
    # GIS Analysis #
    ################

    def aggregate_raster(self, i, output, agg_factor=2, type="mean", callback=None):
        """Aggregates a raster to a lower resolution.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        agg_factor -- Aggregation factor, in pixels. 
        type -- Statistic used to fill output pixels. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--agg_factor={}".format(agg_factor))
        args.append("--type={}".format(type))
        return self.run_tool('aggregate_raster', args, callback) # returns 1 if error

    def block_maximum_gridding(self, i, field, output, use_z=False, cell_size=None, base=None, callback=None):
        """Creates a raster grid based on a set of vector points and assigns grid values using a block maximum scheme.

        Keyword arguments:

        i -- Input vector Points file. 
        field -- Input field name in attribute table. 
        use_z -- Use z-coordinate instead of field?. 
        output -- Output raster file. 
        cell_size -- Optionally specified cell size of output raster. Not used when base raster is specified. 
        base -- Optionally specified input base raster file. Not used when a cell size is specified. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--field='{}'".format(field))
        if use_z: args.append("--use_z")
        args.append("--output='{}'".format(output))
        if cell_size is not None: args.append("--cell_size='{}'".format(cell_size))
        if base is not None: args.append("--base='{}'".format(base))
        return self.run_tool('block_maximum_gridding', args, callback) # returns 1 if error

    def block_minimum_gridding(self, i, field, output, use_z=False, cell_size=None, base=None, callback=None):
        """Creates a raster grid based on a set of vector points and assigns grid values using a block minimum scheme.

        Keyword arguments:

        i -- Input vector Points file. 
        field -- Input field name in attribute table. 
        use_z -- Use z-coordinate instead of field?. 
        output -- Output raster file. 
        cell_size -- Optionally specified cell size of output raster. Not used when base raster is specified. 
        base -- Optionally specified input base raster file. Not used when a cell size is specified. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--field='{}'".format(field))
        if use_z: args.append("--use_z")
        args.append("--output='{}'".format(output))
        if cell_size is not None: args.append("--cell_size='{}'".format(cell_size))
        if base is not None: args.append("--base='{}'".format(base))
        return self.run_tool('block_minimum_gridding', args, callback) # returns 1 if error

    def centroid(self, i, output, text_output=False, callback=None):
        """Calculates the centroid, or average location, of raster polygon objects.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        text_output -- Optional text output. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if text_output: args.append("--text_output")
        return self.run_tool('centroid', args, callback) # returns 1 if error

    def centroid_vector(self, i, output, callback=None):
        """Identifes the centroid point of a vector polyline or polygon feature or a group of vector points.

        Keyword arguments:

        i -- Input vector file. 
        output -- Output vector file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('centroid_vector', args, callback) # returns 1 if error

    def clump(self, i, output, diag=True, zero_back=False, callback=None):
        """Groups cells that form discrete areas, assigning them unique identifiers.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        diag -- Flag indicating whether diagonal connections should be considered. 
        zero_back -- Flag indicating whether zero values should be treated as a background. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if diag: args.append("--diag")
        if zero_back: args.append("--zero_back")
        return self.run_tool('clump', args, callback) # returns 1 if error

    def construct_vector_tin(self, i, output, field=None, use_z=False, max_triangle_edge_length=None, callback=None):
        """Creates a vector triangular irregular network (TIN) for a set of vector points.

        Keyword arguments:

        i -- Input vector points file. 
        field -- Input field name in attribute table. 
        use_z -- Use the 'z' dimension of the Shapefile's geometry instead of an attribute field?. 
        output -- Output vector polygon file. 
        max_triangle_edge_length -- Optional maximum triangle edge length; triangles larger than this size will not be gridded. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        if field is not None: args.append("--field='{}'".format(field))
        if use_z: args.append("--use_z")
        args.append("--output='{}'".format(output))
        if max_triangle_edge_length is not None: args.append("--max_triangle_edge_length='{}'".format(max_triangle_edge_length))
        return self.run_tool('construct_vector_tin', args, callback) # returns 1 if error

    def create_hexagonal_vector_grid(self, i, output, width, orientation="horizontal", callback=None):
        """Creates a hexagonal vector grid.

        Keyword arguments:

        i -- Input base file. 
        output -- Output vector polygon file. 
        width -- The grid cell width. 
        orientation -- Grid Orientation, 'horizontal' or 'vertical'. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--width='{}'".format(width))
        args.append("--orientation={}".format(orientation))
        return self.run_tool('create_hexagonal_vector_grid', args, callback) # returns 1 if error

    def create_plane(self, base, output, gradient=15.0, aspect=90.0, constant=0.0, callback=None):
        """Creates a raster image based on the equation for a simple plane.

        Keyword arguments:

        base -- Input base raster file. 
        output -- Output raster file. 
        gradient -- Slope gradient in degrees (-85.0 to 85.0). 
        aspect -- Aspect (direction) in degrees clockwise from north (0.0-360.0). 
        constant -- Constant value. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--base='{}'".format(base))
        args.append("--output='{}'".format(output))
        args.append("--gradient={}".format(gradient))
        args.append("--aspect={}".format(aspect))
        args.append("--constant={}".format(constant))
        return self.run_tool('create_plane', args, callback) # returns 1 if error

    def create_rectangular_vector_grid(self, i, output, width, height, xorig=0, yorig=0, callback=None):
        """Creates a rectangular vector grid.

        Keyword arguments:

        i -- Input base file. 
        output -- Output vector polygon file. 
        width -- The grid cell width. 
        height -- The grid cell height. 
        xorig -- The grid origin x-coordinate. 
        yorig -- The grid origin y-coordinate. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--width='{}'".format(width))
        args.append("--height='{}'".format(height))
        args.append("--xorig={}".format(xorig))
        args.append("--yorig={}".format(yorig))
        return self.run_tool('create_rectangular_vector_grid', args, callback) # returns 1 if error

    def dissolve(self, i, output, field=None, snap=0.0, callback=None):
        """Removes the interior, or shared, boundaries within a vector polygon coverage.

        Keyword arguments:

        i -- Input vector file. 
        field -- Dissolve field attribute (optional). 
        output -- Output vector file. 
        snap -- Snap tolerance. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        if field is not None: args.append("--field='{}'".format(field))
        args.append("--output='{}'".format(output))
        args.append("--snap={}".format(snap))
        return self.run_tool('dissolve', args, callback) # returns 1 if error

    def eliminate_coincident_points(self, i, output, tolerance, callback=None):
        """Removes any coincident, or nearly coincident, points from a vector points file.

        Keyword arguments:

        i -- Input vector file. 
        output -- Output vector polygon file. 
        tolerance -- The distance tolerance for points. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--tolerance='{}'".format(tolerance))
        return self.run_tool('eliminate_coincident_points', args, callback) # returns 1 if error

    def extend_vector_lines(self, i, output, dist, extend="both ends", callback=None):
        """Extends vector lines by a specified distance.

        Keyword arguments:

        i -- Input vector polyline file. 
        output -- Output vector polyline file. 
        dist -- The distance to extend. 
        extend -- Extend direction, 'both ends' (default), 'line start', 'line end'. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--dist='{}'".format(dist))
        args.append("--extend={}".format(extend))
        return self.run_tool('extend_vector_lines', args, callback) # returns 1 if error

    def extract_nodes(self, i, output, callback=None):
        """Converts vector lines or polygons into vertex points.

        Keyword arguments:

        i -- Input vector lines or polygon file. 
        output -- Output vector points file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('extract_nodes', args, callback) # returns 1 if error

    def extract_raster_values_at_points(self, inputs, points, out_text=False, callback=None):
        """Extracts the values of raster(s) at vector point locations.

        Keyword arguments:

        inputs -- Input raster files. 
        points -- Input vector points file. 
        out_text -- Output point values as text? Otherwise, the only output is to to the points file's attribute table. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--points='{}'".format(points))
        if out_text: args.append("--out_text")
        return self.run_tool('extract_raster_values_at_points', args, callback) # returns 1 if error

    def find_lowest_or_highest_points(self, i, output, out_type="lowest", callback=None):
        """Locates the lowest and/or highest valued cells in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output vector points file. 
        out_type -- Output type; one of 'area' (default) and 'volume'. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--out_type={}".format(out_type))
        return self.run_tool('find_lowest_or_highest_points', args, callback) # returns 1 if error

    def idw_interpolation(self, i, field, output, use_z=False, weight=2.0, radius=None, min_points=None, cell_size=None, base=None, callback=None):
        """Interpolates vector points into a raster surface using an inverse-distance weighted scheme.

        Keyword arguments:

        i -- Input vector Points file. 
        field -- Input field name in attribute table. 
        use_z -- Use z-coordinate instead of field?. 
        output -- Output raster file. 
        weight -- IDW weight value. 
        radius -- Search Radius in map units. 
        min_points -- Minimum number of points. 
        cell_size -- Optionally specified cell size of output raster. Not used when base raster is specified. 
        base -- Optionally specified input base raster file. Not used when a cell size is specified. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--field='{}'".format(field))
        if use_z: args.append("--use_z")
        args.append("--output='{}'".format(output))
        args.append("--weight={}".format(weight))
        if radius is not None: args.append("--radius='{}'".format(radius))
        if min_points is not None: args.append("--min_points='{}'".format(min_points))
        if cell_size is not None: args.append("--cell_size='{}'".format(cell_size))
        if base is not None: args.append("--base='{}'".format(base))
        return self.run_tool('idw_interpolation', args, callback) # returns 1 if error

    def layer_footprint(self, i, output, callback=None):
        """Creates a vector polygon footprint of the area covered by a raster grid or vector layer.

        Keyword arguments:

        i -- Input raster or vector file. 
        output -- Output vector polygon file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('layer_footprint', args, callback) # returns 1 if error

    def medoid(self, i, output, callback=None):
        """Calculates the medoid for a series of vector features contained in a shapefile.

        Keyword arguments:

        i -- Input vector file. 
        output -- Output vector file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('medoid', args, callback) # returns 1 if error

    def minimum_bounding_box(self, i, output, criterion="area", features=True, callback=None):
        """Creates a vector minimum bounding rectangle around vector features.

        Keyword arguments:

        i -- Input vector file. 
        output -- Output vector polygon file. 
        criterion -- Minimization criterion; options include 'area' (default), 'length', 'width', and 'perimeter'. 
        features -- Find the minimum bounding rectangles around each individual vector feature. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--criterion={}".format(criterion))
        if features: args.append("--features")
        return self.run_tool('minimum_bounding_box', args, callback) # returns 1 if error

    def minimum_bounding_circle(self, i, output, features=True, callback=None):
        """Delineates the minimum bounding circle (i.e. smallest enclosing circle) for a group of vectors.

        Keyword arguments:

        i -- Input vector file. 
        output -- Output vector polygon file. 
        features -- Find the minimum bounding circle around each individual vector feature. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if features: args.append("--features")
        return self.run_tool('minimum_bounding_circle', args, callback) # returns 1 if error

    def minimum_bounding_envelope(self, i, output, features=True, callback=None):
        """Creates a vector axis-aligned minimum bounding rectangle (envelope) around vector features.

        Keyword arguments:

        i -- Input vector file. 
        output -- Output vector polygon file. 
        features -- Find the minimum bounding envelop around each individual vector feature. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if features: args.append("--features")
        return self.run_tool('minimum_bounding_envelope', args, callback) # returns 1 if error

    def minimum_convex_hull(self, i, output, features=True, callback=None):
        """Creates a vector convex polygon around vector features.

        Keyword arguments:

        i -- Input vector file. 
        output -- Output vector polygon file. 
        features -- Find the hulls around each vector feature. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if features: args.append("--features")
        return self.run_tool('minimum_convex_hull', args, callback) # returns 1 if error

    def natural_neighbour_interpolation(self, i, output, field=None, use_z=False, cell_size=None, base=None, clip=True, callback=None):
        """Creates a raster grid based on Sibson's natural neighbour method.

        Keyword arguments:

        i -- Input vector points file. 
        field -- Input field name in attribute table. 
        use_z -- Use the 'z' dimension of the Shapefile's geometry instead of an attribute field?. 
        output -- Output raster file. 
        cell_size -- Optionally specified cell size of output raster. Not used when base raster is specified. 
        base -- Optionally specified input base raster file. Not used when a cell size is specified. 
        clip -- Clip the data to the convex hull of the points?. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        if field is not None: args.append("--field='{}'".format(field))
        if use_z: args.append("--use_z")
        args.append("--output='{}'".format(output))
        if cell_size is not None: args.append("--cell_size='{}'".format(cell_size))
        if base is not None: args.append("--base='{}'".format(base))
        if clip: args.append("--clip")
        return self.run_tool('natural_neighbour_interpolation', args, callback) # returns 1 if error

    def nearest_neighbour_gridding(self, i, field, output, use_z=False, cell_size=None, base=None, max_dist=None, callback=None):
        """Creates a raster grid based on a set of vector points and assigns grid values using the nearest neighbour.

        Keyword arguments:

        i -- Input vector Points file. 
        field -- Input field name in attribute table. 
        use_z -- Use z-coordinate instead of field?. 
        output -- Output raster file. 
        cell_size -- Optionally specified cell size of output raster. Not used when base raster is specified. 
        base -- Optionally specified input base raster file. Not used when a cell size is specified. 
        max_dist -- Maximum search distance (optional). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--field='{}'".format(field))
        if use_z: args.append("--use_z")
        args.append("--output='{}'".format(output))
        if cell_size is not None: args.append("--cell_size='{}'".format(cell_size))
        if base is not None: args.append("--base='{}'".format(base))
        if max_dist is not None: args.append("--max_dist='{}'".format(max_dist))
        return self.run_tool('nearest_neighbour_gridding', args, callback) # returns 1 if error

    def polygon_area(self, i, callback=None):
        """Calculates the area of vector polygons.

        Keyword arguments:

        i -- Input vector polygon file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        return self.run_tool('polygon_area', args, callback) # returns 1 if error

    def polygon_long_axis(self, i, output, callback=None):
        """This tool can be used to map the long axis of polygon features.

        Keyword arguments:

        i -- Input vector polygons file. 
        output -- Output vector polygon file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('polygon_long_axis', args, callback) # returns 1 if error

    def polygon_perimeter(self, i, callback=None):
        """Calculates the perimeter of vector polygons.

        Keyword arguments:

        i -- Input vector polygon file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        return self.run_tool('polygon_perimeter', args, callback) # returns 1 if error

    def polygon_short_axis(self, i, output, callback=None):
        """This tool can be used to map the short axis of polygon features.

        Keyword arguments:

        i -- Input vector polygons file. 
        output -- Output vector polygon file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('polygon_short_axis', args, callback) # returns 1 if error

    def radial_basis_function_interpolation(self, i, field, output, use_z=False, radius=None, min_points=None, func_type="ThinPlateSpline", poly_order="none", weight=0.1, cell_size=None, base=None, callback=None):
        """Interpolates vector points into a raster surface using a radial basis function scheme.

        Keyword arguments:

        i -- Input vector points file. 
        field -- Input field name in attribute table. 
        use_z -- Use z-coordinate instead of field?. 
        output -- Output raster file. 
        radius -- Search Radius (in map units). 
        min_points -- Minimum number of points. 
        func_type -- Radial basis function type; options are 'ThinPlateSpline' (default), 'PolyHarmonic', 'Gaussian', 'MultiQuadric', 'InverseMultiQuadric'. 
        poly_order -- Polynomial order; options are 'none' (default), 'constant', 'affine'. 
        weight -- Weight parameter used in basis function. 
        cell_size -- Optionally specified cell size of output raster. Not used when base raster is specified. 
        base -- Optionally specified input base raster file. Not used when a cell size is specified. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--field='{}'".format(field))
        if use_z: args.append("--use_z")
        args.append("--output='{}'".format(output))
        if radius is not None: args.append("--radius='{}'".format(radius))
        if min_points is not None: args.append("--min_points='{}'".format(min_points))
        args.append("--func_type={}".format(func_type))
        args.append("--poly_order={}".format(poly_order))
        args.append("--weight={}".format(weight))
        if cell_size is not None: args.append("--cell_size='{}'".format(cell_size))
        if base is not None: args.append("--base='{}'".format(base))
        return self.run_tool('radial_basis_function_interpolation', args, callback) # returns 1 if error

    def raster_area(self, i, output=None, out_text=False, units="grid cells", zero_back=False, callback=None):
        """Calculates the area of polygons or classes within a raster image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        out_text -- Would you like to output polygon areas to text?. 
        units -- Area units; options include 'grid cells' and 'map units'. 
        zero_back -- Flag indicating whether zero values should be treated as a background. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        if out_text: args.append("--out_text")
        args.append("--units={}".format(units))
        if zero_back: args.append("--zero_back")
        return self.run_tool('raster_area', args, callback) # returns 1 if error

    def raster_cell_assignment(self, i, output, assign="column", callback=None):
        """Assign row or column number to cells.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        assign -- Which variable would you like to assign to grid cells? Options include 'column', 'row', 'x', and 'y'. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--assign={}".format(assign))
        return self.run_tool('raster_cell_assignment', args, callback) # returns 1 if error

    def raster_perimeter(self, i, output=None, out_text=False, units="grid cells", zero_back=False, callback=None):
        """Calculates the perimeters of polygons or classes within a raster image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        out_text -- Would you like to output polygon areas to text?. 
        units -- Area units; options include 'grid cells' and 'map units'. 
        zero_back -- Flag indicating whether zero values should be treated as a background. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        if out_text: args.append("--out_text")
        args.append("--units={}".format(units))
        if zero_back: args.append("--zero_back")
        return self.run_tool('raster_perimeter', args, callback) # returns 1 if error

    def reclass(self, i, output, reclass_vals, assign_mode=False, callback=None):
        """Reclassifies the values in a raster image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        reclass_vals -- Reclassification triplet values (new value; from value; to less than), e.g. '0.0;0.0;1.0;1.0;1.0;2.0'. 
        assign_mode -- Optional Boolean flag indicating whether to operate in assign mode, reclass_vals values are interpreted as new value; old value pairs. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--reclass_vals='{}'".format(reclass_vals))
        if assign_mode: args.append("--assign_mode")
        return self.run_tool('reclass', args, callback) # returns 1 if error

    def reclass_equal_interval(self, i, output, interval=10.0, start_val=None, end_val=None, callback=None):
        """Reclassifies the values in a raster image based on equal-ranges.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        interval -- Class interval size. 
        start_val -- Optional starting value (default is input minimum value). 
        end_val -- Optional ending value (default is input maximum value). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--interval={}".format(interval))
        if start_val is not None: args.append("--start_val='{}'".format(start_val))
        if end_val is not None: args.append("--end_val='{}'".format(end_val))
        return self.run_tool('reclass_equal_interval', args, callback) # returns 1 if error

    def reclass_from_file(self, i, reclass_file, output, callback=None):
        """Reclassifies the values in a raster image using reclass ranges in a text file.

        Keyword arguments:

        i -- Input raster file. 
        reclass_file -- Input text file containing reclass ranges. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--reclass_file='{}'".format(reclass_file))
        args.append("--output='{}'".format(output))
        return self.run_tool('reclass_from_file', args, callback) # returns 1 if error

    def smooth_vectors(self, i, output, filter=3, callback=None):
        """Smooths a vector coverage of either a POLYLINE or POLYGON base ShapeType.

        Keyword arguments:

        i -- Input vector POLYLINE or POLYGON file. 
        output -- Output vector file. 
        filter -- The filter size, any odd integer greater than or equal to 3; default is 3. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filter={}".format(filter))
        return self.run_tool('smooth_vectors', args, callback) # returns 1 if error

    def tin_gridding(self, i, output, field=None, use_z=False, resolution=None, base=None, max_triangle_edge_length=None, callback=None):
        """Creates a raster grid based on a triangular irregular network (TIN) fitted to vector points.

        Keyword arguments:

        i -- Input vector points file. 
        field -- Input field name in attribute table. 
        use_z -- Use the 'z' dimension of the Shapefile's geometry instead of an attribute field?. 
        output -- Output raster file. 
        resolution -- Output raster's grid resolution. 
        base -- Optionally specified input base raster file. Not used when a cell size is specified. 
        max_triangle_edge_length -- Optional maximum triangle edge length; triangles larger than this size will not be gridded. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        if field is not None: args.append("--field='{}'".format(field))
        if use_z: args.append("--use_z")
        args.append("--output='{}'".format(output))
        if resolution is not None: args.append("--resolution='{}'".format(resolution))
        if base is not None: args.append("--base='{}'".format(base))
        if max_triangle_edge_length is not None: args.append("--max_triangle_edge_length='{}'".format(max_triangle_edge_length))
        return self.run_tool('tin_gridding', args, callback) # returns 1 if error

    def vector_hex_binning(self, i, output, width, orientation="horizontal", callback=None):
        """Hex-bins a set of vector points.

        Keyword arguments:

        i -- Input base file. 
        output -- Output vector polygon file. 
        width -- The grid cell width. 
        orientation -- Grid Orientation, 'horizontal' or 'vertical'. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--width='{}'".format(width))
        args.append("--orientation={}".format(orientation))
        return self.run_tool('vector_hex_binning', args, callback) # returns 1 if error

    def voronoi_diagram(self, i, output, callback=None):
        """Creates a vector Voronoi diagram for a set of vector points.

        Keyword arguments:

        i -- Input vector points file. 
        output -- Output vector polygon file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('voronoi_diagram', args, callback) # returns 1 if error

    ###############################
    # GIS Analysis/Distance Tools #
    ###############################

    def buffer_raster(self, i, output, size, gridcells=False, callback=None):
        """Maps a distance-based buffer around each non-background (non-zero/non-nodata) grid cell in an input image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        size -- Buffer size. 
        gridcells -- Optional flag to indicate that the 'size' threshold should be measured in grid cells instead of the default map units. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--size='{}'".format(size))
        if gridcells: args.append("--gridcells")
        return self.run_tool('buffer_raster', args, callback) # returns 1 if error

    def cost_allocation(self, source, backlink, output, callback=None):
        """Identifies the source cell to which each grid cell is connected by a least-cost pathway in a cost-distance analysis.

        Keyword arguments:

        source -- Input source raster file. 
        backlink -- Input backlink raster file generated by the cost-distance tool. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--source='{}'".format(source))
        args.append("--backlink='{}'".format(backlink))
        args.append("--output='{}'".format(output))
        return self.run_tool('cost_allocation', args, callback) # returns 1 if error

    def cost_distance(self, source, cost, out_accum, out_backlink, callback=None):
        """Performs cost-distance accumulation on a cost surface and a group of source cells.

        Keyword arguments:

        source -- Input source raster file. 
        cost -- Input cost (friction) raster file. 
        out_accum -- Output cost accumulation raster file. 
        out_backlink -- Output backlink raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--source='{}'".format(source))
        args.append("--cost='{}'".format(cost))
        args.append("--out_accum='{}'".format(out_accum))
        args.append("--out_backlink='{}'".format(out_backlink))
        return self.run_tool('cost_distance', args, callback) # returns 1 if error

    def cost_pathway(self, destination, backlink, output, zero_background=False, callback=None):
        """Performs cost-distance pathway analysis using a series of destination grid cells.

        Keyword arguments:

        destination -- Input destination raster file. 
        backlink -- Input backlink raster file generated by the cost-distance tool. 
        output -- Output cost pathway raster file. 
        zero_background -- Flag indicating whether zero values should be treated as a background. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--destination='{}'".format(destination))
        args.append("--backlink='{}'".format(backlink))
        args.append("--output='{}'".format(output))
        if zero_background: args.append("--zero_background")
        return self.run_tool('cost_pathway', args, callback) # returns 1 if error

    def euclidean_allocation(self, i, output, callback=None):
        """Assigns grid cells in the output raster the value of the nearest target cell in the input image, measured by the Shih and Wu (2004) Euclidean distance transform.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('euclidean_allocation', args, callback) # returns 1 if error

    def euclidean_distance(self, i, output, callback=None):
        """Calculates the Shih and Wu (2004) Euclidean distance transform.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('euclidean_distance', args, callback) # returns 1 if error

    ##############################
    # GIS Analysis/Overlay Tools #
    ##############################

    def average_overlay(self, inputs, output, callback=None):
        """Calculates the average for each grid cell from a group of raster images.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('average_overlay', args, callback) # returns 1 if error

    def clip(self, i, clip, output, callback=None):
        """Extract all the features, or parts of features, that overlap with the features of the clip vector.

        Keyword arguments:

        i -- Input vector file. 
        clip -- Input clip polygon vector file. 
        output -- Output vector file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--clip='{}'".format(clip))
        args.append("--output='{}'".format(output))
        return self.run_tool('clip', args, callback) # returns 1 if error

    def clip_raster_to_polygon(self, i, polygons, output, maintain_dimensions=False, callback=None):
        """Clips a raster to a vector polygon.

        Keyword arguments:

        i -- Input raster file. 
        polygons -- Input vector polygons file. 
        output -- Output raster file. 
        maintain_dimensions -- Maintain input raster dimensions?. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--polygons='{}'".format(polygons))
        args.append("--output='{}'".format(output))
        if maintain_dimensions: args.append("--maintain_dimensions")
        return self.run_tool('clip_raster_to_polygon', args, callback) # returns 1 if error

    def count_if(self, inputs, output, value, callback=None):
        """Counts the number of occurrences of a specified value in a cell-stack of rasters.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        value -- Search value (e.g. countif value = 5.0). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        args.append("--value='{}'".format(value))
        return self.run_tool('count_if', args, callback) # returns 1 if error

    def difference(self, i, overlay, output, callback=None):
        """Outputs the features that occur in one of the two vector inputs but not both, i.e. no overlapping features.

        Keyword arguments:

        i -- Input vector file. 
        overlay -- Input overlay vector file. 
        output -- Output vector file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--overlay='{}'".format(overlay))
        args.append("--output='{}'".format(output))
        return self.run_tool('difference', args, callback) # returns 1 if error

    def erase(self, i, erase, output, callback=None):
        """Removes all the features, or parts of features, that overlap with the features of the erase vector polygon.

        Keyword arguments:

        i -- Input vector file. 
        erase -- Input erase polygon vector file. 
        output -- Output vector file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--erase='{}'".format(erase))
        args.append("--output='{}'".format(output))
        return self.run_tool('erase', args, callback) # returns 1 if error

    def erase_polygon_from_raster(self, i, polygons, output, callback=None):
        """Erases (cuts out) a vector polygon from a raster.

        Keyword arguments:

        i -- Input raster file. 
        polygons -- Input vector polygons file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--polygons='{}'".format(polygons))
        args.append("--output='{}'".format(output))
        return self.run_tool('erase_polygon_from_raster', args, callback) # returns 1 if error

    def highest_position(self, inputs, output, callback=None):
        """Identifies the stack position of the maximum value within a raster stack on a cell-by-cell basis.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('highest_position', args, callback) # returns 1 if error

    def intersect(self, i, overlay, output, snap=0.0, callback=None):
        """Identifies the parts of features in common between two input vector layers.

        Keyword arguments:

        i -- Input vector file. 
        overlay -- Input overlay vector file. 
        output -- Output vector file. 
        snap -- Snap tolerance. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--overlay='{}'".format(overlay))
        args.append("--output='{}'".format(output))
        args.append("--snap={}".format(snap))
        return self.run_tool('intersect', args, callback) # returns 1 if error

    def line_intersections(self, input1, input2, output, callback=None):
        """Identifies points where the features of two vector line layers intersect.

        Keyword arguments:

        input1 -- Input vector polyline file. 
        input2 -- Input vector polyline file. 
        output -- Output vector point file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('line_intersections', args, callback) # returns 1 if error

    def lowest_position(self, inputs, output, callback=None):
        """Identifies the stack position of the minimum value within a raster stack on a cell-by-cell basis.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('lowest_position', args, callback) # returns 1 if error

    def max_absolute_overlay(self, inputs, output, callback=None):
        """Evaluates the maximum absolute value for each grid cell from a stack of input rasters.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('max_absolute_overlay', args, callback) # returns 1 if error

    def max_overlay(self, inputs, output, callback=None):
        """Evaluates the maximum value for each grid cell from a stack of input rasters.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('max_overlay', args, callback) # returns 1 if error

    def merge_line_segments(self, i, output, snap=0.0, callback=None):
        """Merges vector line segments into larger features.

        Keyword arguments:

        i -- Input vector file. 
        output -- Output vector file. 
        snap -- Snap tolerance. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--snap={}".format(snap))
        return self.run_tool('merge_line_segments', args, callback) # returns 1 if error

    def min_absolute_overlay(self, inputs, output, callback=None):
        """Evaluates the minimum absolute value for each grid cell from a stack of input rasters.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('min_absolute_overlay', args, callback) # returns 1 if error

    def min_overlay(self, inputs, output, callback=None):
        """Evaluates the minimum value for each grid cell from a stack of input rasters.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('min_overlay', args, callback) # returns 1 if error

    def percent_equal_to(self, inputs, comparison, output, callback=None):
        """Calculates the percentage of a raster stack that have cell values equal to an input on a cell-by-cell basis.

        Keyword arguments:

        inputs -- Input raster files. 
        comparison -- Input comparison raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--comparison='{}'".format(comparison))
        args.append("--output='{}'".format(output))
        return self.run_tool('percent_equal_to', args, callback) # returns 1 if error

    def percent_greater_than(self, inputs, comparison, output, callback=None):
        """Calculates the percentage of a raster stack that have cell values greather than an input on a cell-by-cell basis.

        Keyword arguments:

        inputs -- Input raster files. 
        comparison -- Input comparison raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--comparison='{}'".format(comparison))
        args.append("--output='{}'".format(output))
        return self.run_tool('percent_greater_than', args, callback) # returns 1 if error

    def percent_less_than(self, inputs, comparison, output, callback=None):
        """Calculates the percentage of a raster stack that have cell values less than an input on a cell-by-cell basis.

        Keyword arguments:

        inputs -- Input raster files. 
        comparison -- Input comparison raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--comparison='{}'".format(comparison))
        args.append("--output='{}'".format(output))
        return self.run_tool('percent_less_than', args, callback) # returns 1 if error

    def pick_from_list(self, inputs, pos_input, output, callback=None):
        """Outputs the value from a raster stack specified by a position raster.

        Keyword arguments:

        inputs -- Input raster files. 
        pos_input -- Input position raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--pos_input='{}'".format(pos_input))
        args.append("--output='{}'".format(output))
        return self.run_tool('pick_from_list', args, callback) # returns 1 if error

    def polygonize(self, inputs, output, callback=None):
        """Creates a polygon layer from two or more intersecting line features contained in one or more input vector line files.

        Keyword arguments:

        inputs -- Input vector polyline file. 
        output -- Output vector polygon file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('polygonize', args, callback) # returns 1 if error

    def split_with_lines(self, i, split, output, callback=None):
        """Splits the lines or polygons in one layer using the lines in another layer.

        Keyword arguments:

        i -- Input vector line or polygon file. 
        split -- Input vector polyline file. 
        output -- Output vector file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--split='{}'".format(split))
        args.append("--output='{}'".format(output))
        return self.run_tool('split_with_lines', args, callback) # returns 1 if error

    def sum_overlay(self, inputs, output, callback=None):
        """Calculates the sum for each grid cell from a group of raster images.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('sum_overlay', args, callback) # returns 1 if error

    def symmetrical_difference(self, i, overlay, output, snap=0.0, callback=None):
        """Outputs the features that occur in one of the two vector inputs but not both, i.e. no overlapping features.

        Keyword arguments:

        i -- Input vector file. 
        overlay -- Input overlay vector file. 
        output -- Output vector file. 
        snap -- Snap tolerance. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--overlay='{}'".format(overlay))
        args.append("--output='{}'".format(output))
        args.append("--snap={}".format(snap))
        return self.run_tool('symmetrical_difference', args, callback) # returns 1 if error

    def union(self, i, overlay, output, snap=0.0, callback=None):
        """Splits vector layers at their overlaps, creating a layer containing all the portions from both input and overlay layers.

        Keyword arguments:

        i -- Input vector file. 
        overlay -- Input overlay vector file. 
        output -- Output vector file. 
        snap -- Snap tolerance. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--overlay='{}'".format(overlay))
        args.append("--output='{}'".format(output))
        args.append("--snap={}".format(snap))
        return self.run_tool('union', args, callback) # returns 1 if error

    def weighted_overlay(self, factors, weights, output, cost=None, constraints=None, scale_max=1.0, callback=None):
        """Performs a weighted sum on multiple input rasters after converting each image to a common scale. The tool performs a multi-criteria evaluation (MCE).

        Keyword arguments:

        factors -- Input factor raster files. 
        weights -- Weight values, contained in quotes and separated by commas or semicolons. Must have the same number as factors. 
        cost -- Weight values, contained in quotes and separated by commas or semicolons. Must have the same number as factors. 
        constraints -- Input constraints raster files. 
        output -- Output raster file. 
        scale_max -- Suitability scale maximum value (common values are 1.0, 100.0, and 255.0). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--factors='{}'".format(factors))
        args.append("--weights='{}'".format(weights))
        if cost is not None: args.append("--cost='{}'".format(cost))
        if constraints is not None: args.append("--constraints='{}'".format(constraints))
        args.append("--output='{}'".format(output))
        args.append("--scale_max={}".format(scale_max))
        return self.run_tool('weighted_overlay', args, callback) # returns 1 if error

    def weighted_sum(self, inputs, weights, output, callback=None):
        """Performs a weighted-sum overlay on multiple input raster images.

        Keyword arguments:

        inputs -- Input raster files. 
        weights -- Weight values, contained in quotes and separated by commas or semicolons. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--weights='{}'".format(weights))
        args.append("--output='{}'".format(output))
        return self.run_tool('weighted_sum', args, callback) # returns 1 if error

    ##################################
    # GIS Analysis/Patch Shape Tools #
    ##################################

    def boundary_shape_complexity(self, i, output, callback=None):
        """Calculates the complexity of the boundaries of raster polygons.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('boundary_shape_complexity', args, callback) # returns 1 if error

    def compactness_ratio(self, i, callback=None):
        """Calculates the compactness ratio (A/P), a measure of shape complexity, for vector polygons.

        Keyword arguments:

        i -- Input vector polygon file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        return self.run_tool('compactness_ratio', args, callback) # returns 1 if error

    def edge_proportion(self, i, output, output_text=False, callback=None):
        """Calculate the proportion of cells in a raster polygon that are edge cells.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        output_text -- flag indicating whether a text report should also be output. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if output_text: args.append("--output_text")
        return self.run_tool('edge_proportion', args, callback) # returns 1 if error

    def elongation_ratio(self, i, callback=None):
        """Calculates the elongation ratio for vector polygons.

        Keyword arguments:

        i -- Input vector polygon file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        return self.run_tool('elongation_ratio', args, callback) # returns 1 if error

    def find_patch_or_class_edge_cells(self, i, output, callback=None):
        """Finds all cells located on the edge of patch or class features.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('find_patch_or_class_edge_cells', args, callback) # returns 1 if error

    def hole_proportion(self, i, callback=None):
        """Calculates the proportion of the total area of a polygon's holes relative to the area of the polygon's hull.

        Keyword arguments:

        i -- Input vector polygon file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        return self.run_tool('hole_proportion', args, callback) # returns 1 if error

    def linearity_index(self, i, callback=None):
        """Calculates the linearity index for vector polygons.

        Keyword arguments:

        i -- Input vector polygon file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        return self.run_tool('linearity_index', args, callback) # returns 1 if error

    def narrowness_index(self, i, output, callback=None):
        """Calculates the narrowness of raster polygons.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('narrowness_index', args, callback) # returns 1 if error

    def patch_orientation(self, i, callback=None):
        """Calculates the orientation of vector polygons.

        Keyword arguments:

        i -- Input vector polygon file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        return self.run_tool('patch_orientation', args, callback) # returns 1 if error

    def perimeter_area_ratio(self, i, callback=None):
        """Calculates the perimeter-area ratio of vector polygons.

        Keyword arguments:

        i -- Input vector polygon file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        return self.run_tool('perimeter_area_ratio', args, callback) # returns 1 if error

    def radius_of_gyration(self, i, output, text_output=False, callback=None):
        """Calculates the distance of cells from their polygon's centroid.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        text_output -- Optional text output. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if text_output: args.append("--text_output")
        return self.run_tool('radius_of_gyration', args, callback) # returns 1 if error

    def related_circumscribing_circle(self, i, callback=None):
        """Calculates the related circumscribing circle of vector polygons.

        Keyword arguments:

        i -- Input vector polygon file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        return self.run_tool('related_circumscribing_circle', args, callback) # returns 1 if error

    def shape_complexity_index(self, i, callback=None):
        """Calculates overall polygon shape complexity or irregularity.

        Keyword arguments:

        i -- Input vector polygon file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        return self.run_tool('shape_complexity_index', args, callback) # returns 1 if error

    def shape_complexity_index_raster(self, i, output, callback=None):
        """Calculates the complexity of raster polygons or classes.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('shape_complexity_index_raster', args, callback) # returns 1 if error

    ############################
    # Geomorphometric Analysis #
    ############################

    def aspect(self, dem, output, zfactor=1.0, callback=None):
        """Calculates an aspect raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('aspect', args, callback) # returns 1 if error

    def average_normal_vector_angular_deviation(self, dem, output, filter=11, callback=None):
        """Calculates the circular variance of aspect at a scale for a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        filter -- Size of the filter kernel. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filter={}".format(filter))
        return self.run_tool('average_normal_vector_angular_deviation', args, callback) # returns 1 if error

    def circular_variance_of_aspect(self, dem, output, filter=11, callback=None):
        """Calculates the circular variance of aspect at a scale for a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        filter -- Size of the filter kernel. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filter={}".format(filter))
        return self.run_tool('circular_variance_of_aspect', args, callback) # returns 1 if error

    def dev_from_mean_elev(self, dem, output, filterx=11, filtery=11, callback=None):
        """Calculates deviation from mean elevation.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('dev_from_mean_elev', args, callback) # returns 1 if error

    def diff_from_mean_elev(self, dem, output, filterx=11, filtery=11, callback=None):
        """Calculates difference from mean elevation (equivalent to a high-pass filter).

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('diff_from_mean_elev', args, callback) # returns 1 if error

    def directional_relief(self, dem, output, azimuth=0.0, max_dist=None, callback=None):
        """Calculates relief for cells in an input DEM for a specified direction.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        azimuth -- Wind azimuth in degrees. 
        max_dist -- Optional maximum search distance (unspecified if none; in xy units). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--azimuth={}".format(azimuth))
        if max_dist is not None: args.append("--max_dist='{}'".format(max_dist))
        return self.run_tool('directional_relief', args, callback) # returns 1 if error

    def downslope_index(self, dem, output, drop=2.0, out_type="tangent", callback=None):
        """Calculates the Hjerdt et al. (2004) downslope index.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        drop -- Vertical drop value (default is 2.0). 
        out_type -- Output type, options include 'tangent', 'degrees', 'radians', 'distance' (default is 'tangent'). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--drop={}".format(drop))
        args.append("--out_type={}".format(out_type))
        return self.run_tool('downslope_index', args, callback) # returns 1 if error

    def edge_density(self, dem, output, filter=11, norm_diff=5.0, zfactor=1.0, callback=None):
        """Calculates the density of edges, or breaks-in-slope within DEMs.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        filter -- Size of the filter kernel. 
        norm_diff -- Maximum difference in normal vectors, in degrees. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filter={}".format(filter))
        args.append("--norm_diff={}".format(norm_diff))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('edge_density', args, callback) # returns 1 if error

    def elev_above_pit(self, dem, output, callback=None):
        """Calculate the elevation of each grid cell above the nearest downstream pit cell or grid edge cell.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('elev_above_pit', args, callback) # returns 1 if error

    def elev_percentile(self, dem, output, filterx=11, filtery=11, sig_digits=2, callback=None):
        """Calculates the elevation percentile raster from a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        sig_digits -- Number of significant digits. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        args.append("--sig_digits={}".format(sig_digits))
        return self.run_tool('elev_percentile', args, callback) # returns 1 if error

    def elev_relative_to_min_max(self, dem, output, callback=None):
        """Calculates the elevation of a location relative to the minimum and maximum elevations in a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('elev_relative_to_min_max', args, callback) # returns 1 if error

    def elev_relative_to_watershed_min_max(self, dem, watersheds, output, callback=None):
        """Calculates the elevation of a location relative to the minimum and maximum elevations in a watershed.

        Keyword arguments:

        dem -- Input raster DEM file. 
        watersheds -- Input raster watersheds file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--watersheds='{}'".format(watersheds))
        args.append("--output='{}'".format(output))
        return self.run_tool('elev_relative_to_watershed_min_max', args, callback) # returns 1 if error

    def feature_preserving_smoothing(self, dem, output, filter=11, norm_diff=15.0, num_iter=3, max_diff=0.5, zfactor=1.0, callback=None):
        """Reduces short-scale variation in an input DEM using a modified Sun et al. (2007) algorithm.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        filter -- Size of the filter kernel. 
        norm_diff -- Maximum difference in normal vectors, in degrees. 
        num_iter -- Number of iterations. 
        max_diff -- Maximum allowable absolute elevation change (optional). 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filter={}".format(filter))
        args.append("--norm_diff={}".format(norm_diff))
        args.append("--num_iter={}".format(num_iter))
        args.append("--max_diff={}".format(max_diff))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('feature_preserving_smoothing', args, callback) # returns 1 if error

    def fetch_analysis(self, dem, output, azimuth=0.0, hgt_inc=0.05, callback=None):
        """Performs an analysis of fetch or upwind distance to an obstacle.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        azimuth -- Wind azimuth in degrees in degrees. 
        hgt_inc -- Height increment value. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--azimuth={}".format(azimuth))
        args.append("--hgt_inc={}".format(hgt_inc))
        return self.run_tool('fetch_analysis', args, callback) # returns 1 if error

    def fill_missing_data(self, i, output, filter=11, weight=2.0, callback=None):
        """Fills NoData holes in a DEM.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filter -- Filter size (cells). 
        weight -- IDW weight value. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filter={}".format(filter))
        args.append("--weight={}".format(weight))
        return self.run_tool('fill_missing_data', args, callback) # returns 1 if error

    def find_ridges(self, dem, output, line_thin=True, callback=None):
        """Identifies potential ridge and peak grid cells.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        line_thin -- Optional flag indicating whether post-processing line-thinning should be performed. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if line_thin: args.append("--line_thin")
        return self.run_tool('find_ridges', args, callback) # returns 1 if error

    def hillshade(self, dem, output, azimuth=315.0, altitude=30.0, zfactor=1.0, callback=None):
        """Calculates a hillshade raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        azimuth -- Illumination source azimuth in degrees. 
        altitude -- Illumination source altitude in degrees. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--azimuth={}".format(azimuth))
        args.append("--altitude={}".format(altitude))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('hillshade', args, callback) # returns 1 if error

    def horizon_angle(self, dem, output, azimuth=0.0, max_dist=None, callback=None):
        """Calculates horizon angle (maximum upwind slope) for each grid cell in an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        azimuth -- Wind azimuth in degrees. 
        max_dist -- Optional maximum search distance (unspecified if none; in xy units). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--azimuth={}".format(azimuth))
        if max_dist is not None: args.append("--max_dist='{}'".format(max_dist))
        return self.run_tool('horizon_angle', args, callback) # returns 1 if error

    def hypsometric_analysis(self, inputs, output, watershed=None, callback=None):
        """Calculates a hypsometric curve for one or more DEMs.

        Keyword arguments:

        inputs -- Input DEM files. 
        watershed -- Input watershed files (optional). 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        if watershed is not None: args.append("--watershed='{}'".format(watershed))
        args.append("--output='{}'".format(output))
        return self.run_tool('hypsometric_analysis', args, callback) # returns 1 if error

    def max_anisotropy_dev(self, dem, out_mag, out_scale, max_scale, min_scale=3, step=2, callback=None):
        """Calculates the maximum anisotropy (directionality) in elevation deviation over a range of spatial scales.

        Keyword arguments:

        dem -- Input raster DEM file. 
        out_mag -- Output raster DEVmax magnitude file. 
        out_scale -- Output raster DEVmax scale file. 
        min_scale -- Minimum search neighbourhood radius in grid cells. 
        max_scale -- Maximum search neighbourhood radius in grid cells. 
        step -- Step size as any positive non-zero integer. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--out_mag='{}'".format(out_mag))
        args.append("--out_scale='{}'".format(out_scale))
        args.append("--min_scale={}".format(min_scale))
        args.append("--max_scale='{}'".format(max_scale))
        args.append("--step={}".format(step))
        return self.run_tool('max_anisotropy_dev', args, callback) # returns 1 if error

    def max_anisotropy_dev_signature(self, dem, points, output, max_scale, min_scale=1, step=1, callback=None):
        """Calculates the anisotropy in deviation from mean for points over a range of spatial scales.

        Keyword arguments:

        dem -- Input raster DEM file. 
        points -- Input vector points file. 
        output -- Output HTML file. 
        min_scale -- Minimum search neighbourhood radius in grid cells. 
        max_scale -- Maximum search neighbourhood radius in grid cells. 
        step -- Step size as any positive non-zero integer. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--points='{}'".format(points))
        args.append("--output='{}'".format(output))
        args.append("--min_scale={}".format(min_scale))
        args.append("--max_scale='{}'".format(max_scale))
        args.append("--step={}".format(step))
        return self.run_tool('max_anisotropy_dev_signature', args, callback) # returns 1 if error

    def max_branch_length(self, dem, output, log=False, callback=None):
        """Lindsay and Seibert's (2013) branch length index is used to map drainage divides or ridge lines.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        log -- Optional flag to request the output be log-transformed. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if log: args.append("--log")
        return self.run_tool('max_branch_length', args, callback) # returns 1 if error

    def max_difference_from_mean(self, dem, out_mag, out_scale, min_scale, max_scale, step=1, callback=None):
        """Calculates the maximum difference from mean elevation over a range of spatial scales.

        Keyword arguments:

        dem -- Input raster DEM file. 
        out_mag -- Output raster DIFFmax magnitude file. 
        out_scale -- Output raster DIFFmax scale file. 
        min_scale -- Minimum search neighbourhood radius in grid cells. 
        max_scale -- Maximum search neighbourhood radius in grid cells. 
        step -- Step size as any positive non-zero integer. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--out_mag='{}'".format(out_mag))
        args.append("--out_scale='{}'".format(out_scale))
        args.append("--min_scale='{}'".format(min_scale))
        args.append("--max_scale='{}'".format(max_scale))
        args.append("--step={}".format(step))
        return self.run_tool('max_difference_from_mean', args, callback) # returns 1 if error

    def max_downslope_elev_change(self, dem, output, callback=None):
        """Calculates the maximum downslope change in elevation between a grid cell and its eight downslope neighbors.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('max_downslope_elev_change', args, callback) # returns 1 if error

    def max_elev_dev_signature(self, dem, points, output, min_scale, max_scale, step=10, callback=None):
        """Calculates the maximum elevation deviation over a range of spatial scales and for a set of points.

        Keyword arguments:

        dem -- Input raster DEM file. 
        points -- Input vector points file. 
        output -- Output HTML file. 
        min_scale -- Minimum search neighbourhood radius in grid cells. 
        max_scale -- Maximum search neighbourhood radius in grid cells. 
        step -- Step size as any positive non-zero integer. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--points='{}'".format(points))
        args.append("--output='{}'".format(output))
        args.append("--min_scale='{}'".format(min_scale))
        args.append("--max_scale='{}'".format(max_scale))
        args.append("--step={}".format(step))
        return self.run_tool('max_elev_dev_signature', args, callback) # returns 1 if error

    def max_elevation_deviation(self, dem, out_mag, out_scale, min_scale, max_scale, step=1, callback=None):
        """Calculates the maximum elevation deviation over a range of spatial scales.

        Keyword arguments:

        dem -- Input raster DEM file. 
        out_mag -- Output raster DEVmax magnitude file. 
        out_scale -- Output raster DEVmax scale file. 
        min_scale -- Minimum search neighbourhood radius in grid cells. 
        max_scale -- Maximum search neighbourhood radius in grid cells. 
        step -- Step size as any positive non-zero integer. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--out_mag='{}'".format(out_mag))
        args.append("--out_scale='{}'".format(out_scale))
        args.append("--min_scale='{}'".format(min_scale))
        args.append("--max_scale='{}'".format(max_scale))
        args.append("--step={}".format(step))
        return self.run_tool('max_elevation_deviation', args, callback) # returns 1 if error

    def min_downslope_elev_change(self, dem, output, callback=None):
        """Calculates the minimum downslope change in elevation between a grid cell and its eight downslope neighbors.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('min_downslope_elev_change', args, callback) # returns 1 if error

    def multiscale_elevation_percentile(self, dem, out_mag, out_scale, sig_digits=3, min_scale=4, step=1, num_steps=10, step_nonlinearity=1.0, callback=None):
        """Calculates surface roughness over a range of spatial scales.

        Keyword arguments:

        dem -- Input raster DEM file. 
        out_mag -- Output raster roughness magnitude file. 
        out_scale -- Output raster roughness scale file. 
        sig_digits -- Number of significant digits. 
        min_scale -- Minimum search neighbourhood radius in grid cells. 
        step -- Step size as any positive non-zero integer. 
        num_steps -- Number of steps. 
        step_nonlinearity -- Step nonlinearity factor (1.0-2.0 is typical). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--out_mag='{}'".format(out_mag))
        args.append("--out_scale='{}'".format(out_scale))
        args.append("--sig_digits={}".format(sig_digits))
        args.append("--min_scale={}".format(min_scale))
        args.append("--step={}".format(step))
        args.append("--num_steps={}".format(num_steps))
        args.append("--step_nonlinearity={}".format(step_nonlinearity))
        return self.run_tool('multiscale_elevation_percentile', args, callback) # returns 1 if error

    def multiscale_roughness(self, dem, out_mag, out_scale, max_scale, min_scale=1, step=1, callback=None):
        """Calculates surface roughness over a range of spatial scales.

        Keyword arguments:

        dem -- Input raster DEM file. 
        out_mag -- Output raster roughness magnitude file. 
        out_scale -- Output raster roughness scale file. 
        min_scale -- Minimum search neighbourhood radius in grid cells. 
        max_scale -- Maximum search neighbourhood radius in grid cells. 
        step -- Step size as any positive non-zero integer. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--out_mag='{}'".format(out_mag))
        args.append("--out_scale='{}'".format(out_scale))
        args.append("--min_scale={}".format(min_scale))
        args.append("--max_scale='{}'".format(max_scale))
        args.append("--step={}".format(step))
        return self.run_tool('multiscale_roughness', args, callback) # returns 1 if error

    def multiscale_roughness_signature(self, dem, points, output, max_scale, min_scale=1, step=1, callback=None):
        """Calculates the surface roughness for points over a range of spatial scales.

        Keyword arguments:

        dem -- Input raster DEM file. 
        points -- Input vector points file. 
        output -- Output HTML file. 
        min_scale -- Minimum search neighbourhood radius in grid cells. 
        max_scale -- Maximum search neighbourhood radius in grid cells. 
        step -- Step size as any positive non-zero integer. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--points='{}'".format(points))
        args.append("--output='{}'".format(output))
        args.append("--min_scale={}".format(min_scale))
        args.append("--max_scale='{}'".format(max_scale))
        args.append("--step={}".format(step))
        return self.run_tool('multiscale_roughness_signature', args, callback) # returns 1 if error

    def multiscale_std_dev_normals(self, dem, out_mag, out_scale, min_scale=1, step=1, num_steps=10, step_nonlinearity=1.0, callback=None):
        """Calculates surface roughness over a range of spatial scales.

        Keyword arguments:

        dem -- Input raster DEM file. 
        out_mag -- Output raster roughness magnitude file. 
        out_scale -- Output raster roughness scale file. 
        min_scale -- Minimum search neighbourhood radius in grid cells. 
        step -- Step size as any positive non-zero integer. 
        num_steps -- Number of steps. 
        step_nonlinearity -- Step nonlinearity factor (1.0-2.0 is typical). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--out_mag='{}'".format(out_mag))
        args.append("--out_scale='{}'".format(out_scale))
        args.append("--min_scale={}".format(min_scale))
        args.append("--step={}".format(step))
        args.append("--num_steps={}".format(num_steps))
        args.append("--step_nonlinearity={}".format(step_nonlinearity))
        return self.run_tool('multiscale_std_dev_normals', args, callback) # returns 1 if error

    def multiscale_std_dev_normals_signature(self, dem, points, output, min_scale=1, step=1, num_steps=10, step_nonlinearity=1.0, callback=None):
        """Calculates the surface roughness for points over a range of spatial scales.

        Keyword arguments:

        dem -- Input raster DEM file. 
        points -- Input vector points file. 
        output -- Output HTML file. 
        min_scale -- Minimum search neighbourhood radius in grid cells. 
        step -- Step size as any positive non-zero integer. 
        num_steps -- Number of steps. 
        step_nonlinearity -- Step nonlinearity factor (1.0-2.0 is typical). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--points='{}'".format(points))
        args.append("--output='{}'".format(output))
        args.append("--min_scale={}".format(min_scale))
        args.append("--step={}".format(step))
        args.append("--num_steps={}".format(num_steps))
        args.append("--step_nonlinearity={}".format(step_nonlinearity))
        return self.run_tool('multiscale_std_dev_normals_signature', args, callback) # returns 1 if error

    def multiscale_topographic_position_image(self, local, meso, broad, output, lightness=1.2, callback=None):
        """Creates a multiscale topographic position image from three DEVmax rasters of differing spatial scale ranges.

        Keyword arguments:

        local -- Input local-scale topographic position (DEVmax) raster file. 
        meso -- Input meso-scale topographic position (DEVmax) raster file. 
        broad -- Input broad-scale topographic position (DEVmax) raster file. 
        output -- Output raster file. 
        lightness -- Image lightness value (default is 1.2). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--local='{}'".format(local))
        args.append("--meso='{}'".format(meso))
        args.append("--broad='{}'".format(broad))
        args.append("--output='{}'".format(output))
        args.append("--lightness={}".format(lightness))
        return self.run_tool('multiscale_topographic_position_image', args, callback) # returns 1 if error

    def num_downslope_neighbours(self, dem, output, callback=None):
        """Calculates the number of downslope neighbours to each grid cell in a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('num_downslope_neighbours', args, callback) # returns 1 if error

    def num_upslope_neighbours(self, dem, output, callback=None):
        """Calculates the number of upslope neighbours to each grid cell in a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('num_upslope_neighbours', args, callback) # returns 1 if error

    def pennock_landform_class(self, dem, output, slope=3.0, prof=0.1, plan=0.0, zfactor=1.0, callback=None):
        """Classifies hillslope zones based on slope, profile curvature, and plan curvature.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        slope -- Slope threshold value, in degrees (default is 3.0). 
        prof -- Profile curvature threshold value (default is 0.1). 
        plan -- Plan curvature threshold value (default is 0.0). 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--slope={}".format(slope))
        args.append("--prof={}".format(prof))
        args.append("--plan={}".format(plan))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('pennock_landform_class', args, callback) # returns 1 if error

    def percent_elev_range(self, dem, output, filterx=3, filtery=3, callback=None):
        """Calculates percent of elevation range from a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('percent_elev_range', args, callback) # returns 1 if error

    def plan_curvature(self, dem, output, zfactor=1.0, callback=None):
        """Calculates a plan (contour) curvature raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('plan_curvature', args, callback) # returns 1 if error

    def profile(self, lines, surface, output, callback=None):
        """Plots profiles from digital surface models.

        Keyword arguments:

        lines -- Input vector line file. 
        surface -- Input raster surface file. 
        output -- Output HTML file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--lines='{}'".format(lines))
        args.append("--surface='{}'".format(surface))
        args.append("--output='{}'".format(output))
        return self.run_tool('profile', args, callback) # returns 1 if error

    def profile_curvature(self, dem, output, zfactor=1.0, callback=None):
        """Calculates a profile curvature raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('profile_curvature', args, callback) # returns 1 if error

    def relative_aspect(self, dem, output, azimuth=0.0, zfactor=1.0, callback=None):
        """Calculates relative aspect (relative to a user-specified direction) from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        azimuth -- Illumination source azimuth. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--azimuth={}".format(azimuth))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('relative_aspect', args, callback) # returns 1 if error

    def relative_topographic_position(self, dem, output, filterx=11, filtery=11, callback=None):
        """Calculates the relative topographic position index from a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('relative_topographic_position', args, callback) # returns 1 if error

    def remove_off_terrain_objects(self, dem, output, filter=11, slope=15.0, callback=None):
        """Removes off-terrain objects from a raster digital elevation model (DEM).

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        filter -- Filter size (cells). 
        slope -- Slope threshold value. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filter={}".format(filter))
        args.append("--slope={}".format(slope))
        return self.run_tool('remove_off_terrain_objects', args, callback) # returns 1 if error

    def ruggedness_index(self, dem, output, zfactor=1.0, callback=None):
        """Calculates the Riley et al.'s (1999) terrain ruggedness index from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('ruggedness_index', args, callback) # returns 1 if error

    def sediment_transport_index(self, sca, slope, output, sca_exponent=0.4, slope_exponent=1.3, callback=None):
        """Calculates the sediment transport index.

        Keyword arguments:

        sca -- Input raster specific contributing area (SCA) file. 
        slope -- Input raster slope file. 
        output -- Output raster file. 
        sca_exponent -- SCA exponent value. 
        slope_exponent -- Slope exponent value. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--sca='{}'".format(sca))
        args.append("--slope='{}'".format(slope))
        args.append("--output='{}'".format(output))
        args.append("--sca_exponent={}".format(sca_exponent))
        args.append("--slope_exponent={}".format(slope_exponent))
        return self.run_tool('sediment_transport_index', args, callback) # returns 1 if error

    def slope(self, dem, output, zfactor=1.0, callback=None):
        """Calculates a slope raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('slope', args, callback) # returns 1 if error

    def slope_vs_elevation_plot(self, inputs, output, watershed=None, callback=None):
        """Creates a slope vs. elevation plot for one or more DEMs.

        Keyword arguments:

        inputs -- Input DEM files. 
        watershed -- Input watershed files (optional). 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        if watershed is not None: args.append("--watershed='{}'".format(watershed))
        args.append("--output='{}'".format(output))
        return self.run_tool('slope_vs_elevation_plot', args, callback) # returns 1 if error

    def spherical_std_dev_of_normals(self, dem, output, filter=11, callback=None):
        """Calculates the spherical standard deviation of surface normals for a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        filter -- Size of the filter kernel. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filter={}".format(filter))
        return self.run_tool('spherical_std_dev_of_normals', args, callback) # returns 1 if error

    def standard_deviation_of_slope(self, i, output, zfactor=1.0, filterx=11, filtery=11, callback=None):
        """Calculates the standard deviation of slope from an input DEM.

        Keyword arguments:

        i -- Input raster DEM file. 
        output -- Output raster DEM file. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--zfactor={}".format(zfactor))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('standard_deviation_of_slope', args, callback) # returns 1 if error

    def stream_power_index(self, sca, slope, output, exponent=1.0, callback=None):
        """Calculates the relative stream power index.

        Keyword arguments:

        sca -- Input raster specific contributing area (SCA) file. 
        slope -- Input raster slope file. 
        output -- Output raster file. 
        exponent -- SCA exponent value. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--sca='{}'".format(sca))
        args.append("--slope='{}'".format(slope))
        args.append("--output='{}'".format(output))
        args.append("--exponent={}".format(exponent))
        return self.run_tool('stream_power_index', args, callback) # returns 1 if error

    def surface_area_ratio(self, dem, output, callback=None):
        """Calculates a the surface area ratio of each grid cell in an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('surface_area_ratio', args, callback) # returns 1 if error

    def tangential_curvature(self, dem, output, zfactor=1.0, callback=None):
        """Calculates a tangential curvature raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('tangential_curvature', args, callback) # returns 1 if error

    def total_curvature(self, dem, output, zfactor=1.0, callback=None):
        """Calculates a total curvature raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        zfactor -- Optional multiplier for when the vertical and horizontal units are not the same. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor={}".format(zfactor))
        return self.run_tool('total_curvature', args, callback) # returns 1 if error

    def viewshed(self, dem, stations, output, height=2.0, callback=None):
        """Identifies the viewshed for a point or set of points.

        Keyword arguments:

        dem -- Input raster DEM file. 
        stations -- Input viewing station vector file. 
        output -- Output raster file. 
        height -- Viewing station height, in z units. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--stations='{}'".format(stations))
        args.append("--output='{}'".format(output))
        args.append("--height={}".format(height))
        return self.run_tool('viewshed', args, callback) # returns 1 if error

    def visibility_index(self, dem, output, height=2.0, res_factor=2, callback=None):
        """Estimates the relative visibility of sites in a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        height -- Viewing station height, in z units. 
        res_factor -- The resolution factor determines the density of measured viewsheds. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--height={}".format(height))
        args.append("--res_factor={}".format(res_factor))
        return self.run_tool('visibility_index', args, callback) # returns 1 if error

    def wetness_index(self, sca, slope, output, callback=None):
        """Calculates the topographic wetness index, Ln(A / tan(slope)).

        Keyword arguments:

        sca -- Input raster specific contributing area (SCA) file. 
        slope -- Input raster slope file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--sca='{}'".format(sca))
        args.append("--slope='{}'".format(slope))
        args.append("--output='{}'".format(output))
        return self.run_tool('wetness_index', args, callback) # returns 1 if error

    #########################
    # Hydrological Analysis #
    #########################

    def average_flowpath_slope(self, dem, output, callback=None):
        """Measures the average slope gradient from each grid cell to all upslope divide cells.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('average_flowpath_slope', args, callback) # returns 1 if error

    def average_upslope_flowpath_length(self, dem, output, callback=None):
        """Measures the average length of all upslope flowpaths draining each grid cell.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('average_upslope_flowpath_length', args, callback) # returns 1 if error

    def basins(self, d8_pntr, output, esri_pntr=False, callback=None):
        """Identifies drainage basins that drain to the DEM edge.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('basins', args, callback) # returns 1 if error

    def breach_depressions(self, dem, output, max_depth=None, max_length=None, flat_increment=None, fill_pits=False, callback=None):
        """Breaches all of the depressions in a DEM using Lindsay's (2016) algorithm. This should be preferred over depression filling in most cases.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        max_depth -- Optional maximum breach depth (default is Inf). 
        max_length -- Optional maximum breach channel length (in grid cells; default is Inf). 
        flat_increment -- Optional elevation increment applied to flat areas. 
        fill_pits -- Optional flag indicating whether to fill single-cell pits. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if max_depth is not None: args.append("--max_depth='{}'".format(max_depth))
        if max_length is not None: args.append("--max_length='{}'".format(max_length))
        if flat_increment is not None: args.append("--flat_increment='{}'".format(flat_increment))
        if fill_pits: args.append("--fill_pits")
        return self.run_tool('breach_depressions', args, callback) # returns 1 if error

    def breach_depressions_least_cost(self, dem, output, dist, max_cost=None, min_dist=True, flat_increment=None, fill=True, callback=None):
        """Breaches the depressions in a DEM using a least-cost pathway method.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        dist -- . 
        max_cost -- Optional maximum breach cost (default is Inf). 
        min_dist -- Optional flag indicating whether to minimize breach distances. 
        flat_increment -- Optional elevation increment applied to flat areas. 
        fill -- Optional flag indicating whether to fill any remaining unbreached depressions. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--dist='{}'".format(dist))
        if max_cost is not None: args.append("--max_cost='{}'".format(max_cost))
        if min_dist: args.append("--min_dist")
        if flat_increment is not None: args.append("--flat_increment='{}'".format(flat_increment))
        if fill: args.append("--fill")
        return self.run_tool('breach_depressions_least_cost', args, callback) # returns 1 if error

    def breach_single_cell_pits(self, dem, output, callback=None):
        """Removes single-cell pits from an input DEM by breaching.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('breach_single_cell_pits', args, callback) # returns 1 if error

    def burn_streams_at_roads(self, dem, streams, roads, output, width=None, callback=None):
        """Burns-in streams at the sites of road embankments.

        Keyword arguments:

        dem -- Input raster digital elevation model (DEM) file. 
        streams -- Input vector streams file. 
        roads -- Input vector roads file. 
        output -- Output raster file. 
        width -- Maximum road embankment width, in map units. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--streams='{}'".format(streams))
        args.append("--roads='{}'".format(roads))
        args.append("--output='{}'".format(output))
        if width is not None: args.append("--width='{}'".format(width))
        return self.run_tool('burn_streams_at_roads', args, callback) # returns 1 if error

    def d8_flow_accumulation(self, i, output, out_type="cells", log=False, clip=False, pntr=False, esri_pntr=False, callback=None):
        """Calculates a D8 flow accumulation raster from an input DEM or flow pointer.

        Keyword arguments:

        i -- Input raster DEM or D8 pointer file. 
        output -- Output raster file. 
        out_type -- Output type; one of 'cells' (default), 'catchment area', and 'specific contributing area'. 
        log -- Optional flag to request the output be log-transformed. 
        clip -- Optional flag to request clipping the display max by 1%. 
        pntr -- Is the input raster a D8 flow pointer rather than a DEM?. 
        esri_pntr -- Input  D8 pointer uses the ESRI style scheme. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--out_type={}".format(out_type))
        if log: args.append("--log")
        if clip: args.append("--clip")
        if pntr: args.append("--pntr")
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('d8_flow_accumulation', args, callback) # returns 1 if error

    def d8_mass_flux(self, dem, loading, efficiency, absorption, output, callback=None):
        """Performs a D8 mass flux calculation.

        Keyword arguments:

        dem -- Input raster DEM file. 
        loading -- Input loading raster file. 
        efficiency -- Input efficiency raster file. 
        absorption -- Input absorption raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--loading='{}'".format(loading))
        args.append("--efficiency='{}'".format(efficiency))
        args.append("--absorption='{}'".format(absorption))
        args.append("--output='{}'".format(output))
        return self.run_tool('d8_mass_flux', args, callback) # returns 1 if error

    def d8_pointer(self, dem, output, esri_pntr=False, callback=None):
        """Calculates a D8 flow pointer raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('d8_pointer', args, callback) # returns 1 if error

    def d_inf_flow_accumulation(self, i, output, out_type="Specific Contributing Area", threshold=None, log=False, clip=False, pntr=False, callback=None):
        """Calculates a D-infinity flow accumulation raster from an input DEM.

        Keyword arguments:

        i -- Input raster DEM or D-infinity pointer file. 
        output -- Output raster file. 
        out_type -- Output type; one of 'cells', 'sca' (default), and 'ca'. 
        threshold -- Optional convergence threshold parameter, in grid cells; default is inifinity. 
        log -- Optional flag to request the output be log-transformed. 
        clip -- Optional flag to request clipping the display max by 1%. 
        pntr -- Is the input raster a D8 flow pointer rather than a DEM?. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--out_type={}".format(out_type))
        if threshold is not None: args.append("--threshold='{}'".format(threshold))
        if log: args.append("--log")
        if clip: args.append("--clip")
        if pntr: args.append("--pntr")
        return self.run_tool('d_inf_flow_accumulation', args, callback) # returns 1 if error

    def d_inf_mass_flux(self, dem, loading, efficiency, absorption, output, callback=None):
        """Performs a D-infinity mass flux calculation.

        Keyword arguments:

        dem -- Input raster DEM file. 
        loading -- Input loading raster file. 
        efficiency -- Input efficiency raster file. 
        absorption -- Input absorption raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--loading='{}'".format(loading))
        args.append("--efficiency='{}'".format(efficiency))
        args.append("--absorption='{}'".format(absorption))
        args.append("--output='{}'".format(output))
        return self.run_tool('d_inf_mass_flux', args, callback) # returns 1 if error

    def d_inf_pointer(self, dem, output, callback=None):
        """Calculates a D-infinity flow pointer (flow direction) raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('d_inf_pointer', args, callback) # returns 1 if error

    def depth_in_sink(self, dem, output, zero_background=False, callback=None):
        """Measures the depth of sinks (depressions) in a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        zero_background -- Flag indicating whether the background value of zero should be used. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if zero_background: args.append("--zero_background")
        return self.run_tool('depth_in_sink', args, callback) # returns 1 if error

    def downslope_distance_to_stream(self, dem, streams, output, callback=None):
        """Measures distance to the nearest downslope stream cell.

        Keyword arguments:

        dem -- Input raster DEM file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        return self.run_tool('downslope_distance_to_stream', args, callback) # returns 1 if error

    def downslope_flowpath_length(self, d8_pntr, output, watersheds=None, weights=None, esri_pntr=False, callback=None):
        """Calculates the downslope flowpath length from each cell to basin outlet.

        Keyword arguments:

        d8_pntr -- Input D8 pointer raster file. 
        watersheds -- Optional input watershed raster file. 
        weights -- Optional input weights raster file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        if watersheds is not None: args.append("--watersheds='{}'".format(watersheds))
        if weights is not None: args.append("--weights='{}'".format(weights))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('downslope_flowpath_length', args, callback) # returns 1 if error

    def elevation_above_stream(self, dem, streams, output, callback=None):
        """Calculates the elevation of cells above the nearest downslope stream cell.

        Keyword arguments:

        dem -- Input raster DEM file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        return self.run_tool('elevation_above_stream', args, callback) # returns 1 if error

    def elevation_above_stream_euclidean(self, dem, streams, output, callback=None):
        """Calculates the elevation of cells above the nearest (Euclidean distance) stream cell.

        Keyword arguments:

        dem -- Input raster DEM file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        return self.run_tool('elevation_above_stream_euclidean', args, callback) # returns 1 if error

    def fd8_flow_accumulation(self, dem, output, out_type="specific contributing area", exponent=1.1, threshold=None, log=False, clip=False, callback=None):
        """Calculates an FD8 flow accumulation raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        out_type -- Output type; one of 'cells', 'specific contributing area' (default), and 'catchment area'. 
        exponent -- Optional exponent parameter; default is 1.1. 
        threshold -- Optional convergence threshold parameter, in grid cells; default is inifinity. 
        log -- Optional flag to request the output be log-transformed. 
        clip -- Optional flag to request clipping the display max by 1%. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--out_type={}".format(out_type))
        args.append("--exponent={}".format(exponent))
        if threshold is not None: args.append("--threshold='{}'".format(threshold))
        if log: args.append("--log")
        if clip: args.append("--clip")
        return self.run_tool('fd8_flow_accumulation', args, callback) # returns 1 if error

    def fd8_pointer(self, dem, output, callback=None):
        """Calculates an FD8 flow pointer raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('fd8_pointer', args, callback) # returns 1 if error

    def fill_burn(self, dem, streams, output, callback=None):
        """Burns streams into a DEM using the FillBurn (Saunders, 1999) method.

        Keyword arguments:

        dem -- Input raster DEM file. 
        streams -- Input vector streams file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        return self.run_tool('fill_burn', args, callback) # returns 1 if error

    def fill_depressions(self, dem, output, fix_flats=True, flat_increment=None, max_depth=None, callback=None):
        """Fills all of the depressions in a DEM. Depression breaching should be preferred in most cases.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        fix_flats -- Optional flag indicating whether flat areas should have a small gradient applied. 
        flat_increment -- Optional elevation increment applied to flat areas. 
        max_depth -- Optional maximum depression depth to fill. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if fix_flats: args.append("--fix_flats")
        if flat_increment is not None: args.append("--flat_increment='{}'".format(flat_increment))
        if max_depth is not None: args.append("--max_depth='{}'".format(max_depth))
        return self.run_tool('fill_depressions', args, callback) # returns 1 if error

    def fill_depressions_planchon_and_darboux(self, dem, output, fix_flats=True, flat_increment=None, callback=None):
        """Fills all of the depressions in a DEM using the Planchon and Darboux (2002) method.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        fix_flats -- Optional flag indicating whether flat areas should have a small gradient applied. 
        flat_increment -- Optional elevation increment applied to flat areas. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if fix_flats: args.append("--fix_flats")
        if flat_increment is not None: args.append("--flat_increment='{}'".format(flat_increment))
        return self.run_tool('fill_depressions_planchon_and_darboux', args, callback) # returns 1 if error

    def fill_depressions_wang_and_liu(self, dem, output, fix_flats=True, flat_increment=None, callback=None):
        """Fills all of the depressions in a DEM using the Wang and Liu (2006) method. Depression breaching should be preferred in most cases.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        fix_flats -- Optional flag indicating whether flat areas should have a small gradient applied. 
        flat_increment -- Optional elevation increment applied to flat areas. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if fix_flats: args.append("--fix_flats")
        if flat_increment is not None: args.append("--flat_increment='{}'".format(flat_increment))
        return self.run_tool('fill_depressions_wang_and_liu', args, callback) # returns 1 if error

    def fill_single_cell_pits(self, dem, output, callback=None):
        """Raises pit cells to the elevation of their lowest neighbour.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('fill_single_cell_pits', args, callback) # returns 1 if error

    def find_no_flow_cells(self, dem, output, callback=None):
        """Finds grid cells with no downslope neighbours.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('find_no_flow_cells', args, callback) # returns 1 if error

    def find_parallel_flow(self, d8_pntr, streams, output, callback=None):
        """Finds areas of parallel flow in D8 flow direction rasters.

        Keyword arguments:

        d8_pntr -- Input D8 pointer raster file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        return self.run_tool('find_parallel_flow', args, callback) # returns 1 if error

    def flatten_lakes(self, dem, lakes, output, callback=None):
        """Flattens lake polygons in a raster DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        lakes -- Input lakes vector polygons file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--lakes='{}'".format(lakes))
        args.append("--output='{}'".format(output))
        return self.run_tool('flatten_lakes', args, callback) # returns 1 if error

    def flood_order(self, dem, output, callback=None):
        """Assigns each DEM grid cell its order in the sequence of inundations that are encountered during a search starting from the edges, moving inward at increasing elevations.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('flood_order', args, callback) # returns 1 if error

    def flow_accumulation_full_workflow(self, dem, out_dem, out_pntr, out_accum, out_type="Specific Contributing Area", log=False, clip=False, esri_pntr=False, callback=None):
        """Resolves all of the depressions in a DEM, outputting a breached DEM, an aspect-aligned non-divergent flow pointer, and a flow accumulation raster.

        Keyword arguments:

        dem -- Input raster DEM file. 
        out_dem -- Output raster DEM file. 
        out_pntr -- Output raster flow pointer file. 
        out_accum -- Output raster flow accumulation file. 
        out_type -- Output type; one of 'cells', 'sca' (default), and 'ca'. 
        log -- Optional flag to request the output be log-transformed. 
        clip -- Optional flag to request clipping the display max by 1%. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom function for handling tool text outputs.
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
        return self.run_tool('flow_accumulation_full_workflow', args, callback) # returns 1 if error

    def flow_length_diff(self, d8_pntr, output, esri_pntr=False, callback=None):
        """Calculates the local maximum absolute difference in downslope flowpath length, useful in mapping drainage divides and ridges.

        Keyword arguments:

        d8_pntr -- Input D8 pointer raster file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('flow_length_diff', args, callback) # returns 1 if error

    def hillslopes(self, d8_pntr, streams, output, esri_pntr=False, callback=None):
        """Identifies the individual hillslopes draining to each link in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('hillslopes', args, callback) # returns 1 if error

    def impoundment_size_index(self, dem, output, damlength, out_type="depth", callback=None):
        """Calculates the impoundment size resulting from damming a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output file. 
        out_type -- Output type; one of 'depth' (default), 'volume', and 'area'. 
        damlength -- Maximum length of the dam. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--out_type={}".format(out_type))
        args.append("--damlength='{}'".format(damlength))
        return self.run_tool('impoundment_size_index', args, callback) # returns 1 if error

    def insert_dams(self, dem, dam_pts, output, damlength, callback=None):
        """Calculates the impoundment size resulting from damming a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        dam_pts -- Input vector dam points file. 
        output -- Output file. 
        damlength -- Maximum length of the dam. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--dam_pts='{}'".format(dam_pts))
        args.append("--output='{}'".format(output))
        args.append("--damlength='{}'".format(damlength))
        return self.run_tool('insert_dams', args, callback) # returns 1 if error

    def isobasins(self, dem, output, size, callback=None):
        """Divides a landscape into nearly equal sized drainage basins (i.e. watersheds).

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        size -- Target basin size, in grid cells. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--size='{}'".format(size))
        return self.run_tool('isobasins', args, callback) # returns 1 if error

    def jenson_snap_pour_points(self, pour_pts, streams, output, snap_dist, callback=None):
        """Moves outlet points used to specify points of interest in a watershedding operation to the nearest stream cell.

        Keyword arguments:

        pour_pts -- Input vector pour points (outlet) file. 
        streams -- Input raster streams file. 
        output -- Output vector file. 
        snap_dist -- Maximum snap distance in map units. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--pour_pts='{}'".format(pour_pts))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        args.append("--snap_dist='{}'".format(snap_dist))
        return self.run_tool('jenson_snap_pour_points', args, callback) # returns 1 if error

    def longest_flowpath(self, dem, basins, output, callback=None):
        """Delineates the longest flowpaths for a group of subbasins or watersheds.

        Keyword arguments:

        dem -- Input raster DEM file. 
        basins -- Input raster basins file. 
        output -- Output vector file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--basins='{}'".format(basins))
        args.append("--output='{}'".format(output))
        return self.run_tool('longest_flowpath', args, callback) # returns 1 if error

    def max_upslope_flowpath_length(self, dem, output, callback=None):
        """Measures the maximum length of all upslope flowpaths draining each grid cell.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('max_upslope_flowpath_length', args, callback) # returns 1 if error

    def md_inf_flow_accumulation(self, dem, output, out_type="specific contributing area", exponent=1.1, threshold=None, log=False, clip=False, callback=None):
        """Calculates an FD8 flow accumulation raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        out_type -- Output type; one of 'cells', 'specific contributing area' (default), and 'catchment area'. 
        exponent -- Optional exponent parameter; default is 1.1. 
        threshold -- Optional convergence threshold parameter, in grid cells; default is inifinity. 
        log -- Optional flag to request the output be log-transformed. 
        clip -- Optional flag to request clipping the display max by 1%. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--out_type={}".format(out_type))
        args.append("--exponent={}".format(exponent))
        if threshold is not None: args.append("--threshold='{}'".format(threshold))
        if log: args.append("--log")
        if clip: args.append("--clip")
        return self.run_tool('md_inf_flow_accumulation', args, callback) # returns 1 if error

    def num_inflowing_neighbours(self, dem, output, callback=None):
        """Computes the number of inflowing neighbours to each cell in an input DEM based on the D8 algorithm.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('num_inflowing_neighbours', args, callback) # returns 1 if error

    def raise_walls(self, i, dem, output, breach=None, height=100.0, callback=None):
        """Raises walls in a DEM along a line or around a polygon, e.g. a watershed.

        Keyword arguments:

        i -- Input vector lines or polygons file. 
        breach -- Optional input vector breach lines. 
        dem -- Input raster DEM file. 
        output -- Output raster file. 
        height -- Wall height. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        if breach is not None: args.append("--breach='{}'".format(breach))
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--height={}".format(height))
        return self.run_tool('raise_walls', args, callback) # returns 1 if error

    def rho8_pointer(self, dem, output, esri_pntr=False, callback=None):
        """Calculates a stochastic Rho8 flow pointer raster from an input DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('rho8_pointer', args, callback) # returns 1 if error

    def sink(self, i, output, zero_background=False, callback=None):
        """Identifies the depressions in a DEM, giving each feature a unique identifier.

        Keyword arguments:

        i -- Input raster DEM file. 
        output -- Output raster file. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if zero_background: args.append("--zero_background")
        return self.run_tool('sink', args, callback) # returns 1 if error

    def snap_pour_points(self, pour_pts, flow_accum, output, snap_dist, callback=None):
        """Moves outlet points used to specify points of interest in a watershedding operation to the cell with the highest flow accumulation in its neighbourhood.

        Keyword arguments:

        pour_pts -- Input vector pour points (outlet) file. 
        flow_accum -- Input raster D8 flow accumulation file. 
        output -- Output vector file. 
        snap_dist -- Maximum snap distance in map units. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--pour_pts='{}'".format(pour_pts))
        args.append("--flow_accum='{}'".format(flow_accum))
        args.append("--output='{}'".format(output))
        args.append("--snap_dist='{}'".format(snap_dist))
        return self.run_tool('snap_pour_points', args, callback) # returns 1 if error

    def stochastic_depression_analysis(self, dem, output, rmse, range, iterations=100, callback=None):
        """Preforms a stochastic analysis of depressions within a DEM.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output file. 
        rmse -- The DEM's root-mean-square-error (RMSE), in z units. This determines error magnitude. 
        range -- The error field's correlation length, in xy-units. 
        iterations -- The number of iterations. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--rmse='{}'".format(rmse))
        args.append("--range='{}'".format(range))
        args.append("--iterations={}".format(iterations))
        return self.run_tool('stochastic_depression_analysis', args, callback) # returns 1 if error

    def strahler_order_basins(self, d8_pntr, streams, output, esri_pntr=False, callback=None):
        """Identifies Strahler-order basins from an input stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('strahler_order_basins', args, callback) # returns 1 if error

    def subbasins(self, d8_pntr, streams, output, esri_pntr=False, callback=None):
        """Identifies the catchments, or sub-basin, draining to each link in a stream network.

        Keyword arguments:

        d8_pntr -- Input D8 pointer raster file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('subbasins', args, callback) # returns 1 if error

    def trace_downslope_flowpaths(self, seed_pts, d8_pntr, output, esri_pntr=False, zero_background=False, callback=None):
        """Traces downslope flowpaths from one or more target sites (i.e. seed points).

        Keyword arguments:

        seed_pts -- Input vector seed points file. 
        d8_pntr -- Input D8 pointer raster file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--seed_pts='{}'".format(seed_pts))
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('trace_downslope_flowpaths', args, callback) # returns 1 if error

    def unnest_basins(self, d8_pntr, pour_pts, output, esri_pntr=False, callback=None):
        """Extract whole watersheds for a set of outlet points.

        Keyword arguments:

        d8_pntr -- Input D8 pointer raster file. 
        pour_pts -- Input vector pour points (outlet) file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--pour_pts='{}'".format(pour_pts))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('unnest_basins', args, callback) # returns 1 if error

    def upslope_depression_storage(self, dem, output, callback=None):
        """Estimates the average upslope depression storage depth.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('upslope_depression_storage', args, callback) # returns 1 if error

    def watershed(self, d8_pntr, pour_pts, output, esri_pntr=False, callback=None):
        """Identifies the watershed, or drainage basin, draining to a set of target cells.

        Keyword arguments:

        d8_pntr -- Input D8 pointer raster file. 
        pour_pts -- Input pour points (outlet) file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--pour_pts='{}'".format(pour_pts))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('watershed', args, callback) # returns 1 if error

    ##########################
    # Image Processing Tools #
    ##########################

    def change_vector_analysis(self, date1, date2, magnitude, direction, callback=None):
        """Performs a change vector analysis on a two-date multi-spectral dataset.

        Keyword arguments:

        date1 -- Input raster files for the earlier date. 
        date2 -- Input raster files for the later date. 
        magnitude -- Output vector magnitude raster file. 
        direction -- Output vector Direction raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--date1='{}'".format(date1))
        args.append("--date2='{}'".format(date2))
        args.append("--magnitude='{}'".format(magnitude))
        args.append("--direction='{}'".format(direction))
        return self.run_tool('change_vector_analysis', args, callback) # returns 1 if error

    def closing(self, i, output, filterx=11, filtery=11, callback=None):
        """A closing is a mathematical morphology operation involving an erosion (min filter) of a dilation (max filter) set.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('closing', args, callback) # returns 1 if error

    def create_colour_composite(self, red, green, blue, output, opacity=None, enhance=True, zeros=False, callback=None):
        """Creates a colour-composite image from three bands of multispectral imagery.

        Keyword arguments:

        red -- Input red band image file. 
        green -- Input green band image file. 
        blue -- Input blue band image file. 
        opacity -- Input opacity band image file (optional). 
        output -- Output colour composite file. 
        enhance -- Optional flag indicating whether a balance contrast enhancement is performed. 
        zeros -- Optional flag to indicate if zeros are nodata values. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--red='{}'".format(red))
        args.append("--green='{}'".format(green))
        args.append("--blue='{}'".format(blue))
        if opacity is not None: args.append("--opacity='{}'".format(opacity))
        args.append("--output='{}'".format(output))
        if enhance: args.append("--enhance")
        if zeros: args.append("--zeros")
        return self.run_tool('create_colour_composite', args, callback) # returns 1 if error

    def flip_image(self, i, output, direction="vertical", callback=None):
        """Reflects an image in the vertical or horizontal axis.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        direction -- Direction of reflection; options include 'v' (vertical), 'h' (horizontal), and 'b' (both). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--direction={}".format(direction))
        return self.run_tool('flip_image', args, callback) # returns 1 if error

    def ihs_to_rgb(self, intensity, hue, saturation, red=None, green=None, blue=None, output=None, callback=None):
        """Converts intensity, hue, and saturation (IHS) images into red, green, and blue (RGB) images.

        Keyword arguments:

        intensity -- Input intensity file. 
        hue -- Input hue file. 
        saturation -- Input saturation file. 
        red -- Output red band file. Optionally specified if colour-composite not specified. 
        green -- Output green band file. Optionally specified if colour-composite not specified. 
        blue -- Output blue band file. Optionally specified if colour-composite not specified. 
        output -- Output colour-composite file. Only used if individual bands are not specified. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--intensity='{}'".format(intensity))
        args.append("--hue='{}'".format(hue))
        args.append("--saturation='{}'".format(saturation))
        if red is not None: args.append("--red='{}'".format(red))
        if green is not None: args.append("--green='{}'".format(green))
        if blue is not None: args.append("--blue='{}'".format(blue))
        if output is not None: args.append("--output='{}'".format(output))
        return self.run_tool('ihs_to_rgb', args, callback) # returns 1 if error

    def image_stack_profile(self, inputs, points, output, callback=None):
        """Plots an image stack profile (i.e. signature) for a set of points and multispectral images.

        Keyword arguments:

        inputs -- Input multispectral image files. 
        points -- Input vector points file. 
        output -- Output HTML file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--points='{}'".format(points))
        args.append("--output='{}'".format(output))
        return self.run_tool('image_stack_profile', args, callback) # returns 1 if error

    def integral_image(self, i, output, callback=None):
        """Transforms an input image (summed area table) into its integral image equivalent.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('integral_image', args, callback) # returns 1 if error

    def k_means_clustering(self, inputs, output, classes, out_html=None, max_iterations=10, class_change=2.0, initialize="diagonal", min_class_size=10, callback=None):
        """Performs a k-means clustering operation on a multi-spectral dataset.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        out_html -- Output HTML report file. 
        classes -- Number of classes. 
        max_iterations -- Maximum number of iterations. 
        class_change -- Minimum percent of cells changed between iterations before completion. 
        initialize -- How to initialize cluster centres?. 
        min_class_size -- Minimum class size, in pixels. 
        callback -- Custom function for handling tool text outputs.
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
        return self.run_tool('k_means_clustering', args, callback) # returns 1 if error

    def line_thinning(self, i, output, callback=None):
        """Performs line thinning a on Boolean raster image; intended to be used with the RemoveSpurs tool.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('line_thinning', args, callback) # returns 1 if error

    def modified_k_means_clustering(self, inputs, output, out_html=None, start_clusters=1000, merge_dist=None, max_iterations=10, class_change=2.0, callback=None):
        """Performs a modified k-means clustering operation on a multi-spectral dataset.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        out_html -- Output HTML report file. 
        start_clusters -- Initial number of clusters. 
        merge_dist -- Cluster merger distance. 
        max_iterations -- Maximum number of iterations. 
        class_change -- Minimum percent of cells changed between iterations before completion. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        if out_html is not None: args.append("--out_html='{}'".format(out_html))
        args.append("--start_clusters={}".format(start_clusters))
        if merge_dist is not None: args.append("--merge_dist='{}'".format(merge_dist))
        args.append("--max_iterations={}".format(max_iterations))
        args.append("--class_change={}".format(class_change))
        return self.run_tool('modified_k_means_clustering', args, callback) # returns 1 if error

    def mosaic(self, inputs, output, method="cc", callback=None):
        """Mosaics two or more images together.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output raster file. 
        method -- Resampling method; options include 'nn' (nearest neighbour), 'bilinear', and 'cc' (cubic convolution). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        args.append("--method={}".format(method))
        return self.run_tool('mosaic', args, callback) # returns 1 if error

    def mosaic_with_feathering(self, input1, input2, output, method="cc", weight=4.0, callback=None):
        """Mosaics two images together using a feathering technique in overlapping areas to reduce edge-effects.

        Keyword arguments:

        input1 -- Input raster file to modify. 
        input2 -- Input reference raster file. 
        output -- Output raster file. 
        method -- Resampling method; options include 'nn' (nearest neighbour), 'bilinear', and 'cc' (cubic convolution). 
        weight -- . 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        args.append("--method={}".format(method))
        args.append("--weight={}".format(weight))
        return self.run_tool('mosaic_with_feathering', args, callback) # returns 1 if error

    def normalized_difference_index(self, input1, input2, output, clip=0.0, correction=0.0, callback=None):
        """Calculate a normalized-difference index (NDI) from two bands of multispectral image data.

        Keyword arguments:

        input1 -- Input image 1 (e.g. near-infrared band). 
        input2 -- Input image 2 (e.g. red band). 
        output -- Output raster file. 
        clip -- Optional amount to clip the distribution tails by, in percent. 
        correction -- Optional adjustment value (e.g. 1, or 0.16 for the optimal soil adjusted vegetation index, OSAVI). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        args.append("--clip={}".format(clip))
        args.append("--correction={}".format(correction))
        return self.run_tool('normalized_difference_index', args, callback) # returns 1 if error

    def opening(self, i, output, filterx=11, filtery=11, callback=None):
        """An opening is a mathematical morphology operation involving a dilation (max filter) of an erosion (min filter) set.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('opening', args, callback) # returns 1 if error

    def remove_spurs(self, i, output, iterations=10, callback=None):
        """Removes the spurs (pruning operation) from a Boolean line image; intended to be used on the output of the LineThinning tool.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        iterations -- Maximum number of iterations. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--iterations={}".format(iterations))
        return self.run_tool('remove_spurs', args, callback) # returns 1 if error

    def resample(self, inputs, destination, method="cc", callback=None):
        """Resamples one or more input images into a destination image.

        Keyword arguments:

        inputs -- Input raster files. 
        destination -- Destination raster file. 
        method -- Resampling method; options include 'nn' (nearest neighbour), 'bilinear', and 'cc' (cubic convolution). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--destination='{}'".format(destination))
        args.append("--method={}".format(method))
        return self.run_tool('resample', args, callback) # returns 1 if error

    def rgb_to_ihs(self, intensity, hue, saturation, red=None, green=None, blue=None, composite=None, callback=None):
        """Converts red, green, and blue (RGB) images into intensity, hue, and saturation (IHS) images.

        Keyword arguments:

        red -- Input red band image file. Optionally specified if colour-composite not specified. 
        green -- Input green band image file. Optionally specified if colour-composite not specified. 
        blue -- Input blue band image file. Optionally specified if colour-composite not specified. 
        composite -- Input colour-composite image file. Only used if individual bands are not specified. 
        intensity -- Output intensity raster file. 
        hue -- Output hue raster file. 
        saturation -- Output saturation raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        if red is not None: args.append("--red='{}'".format(red))
        if green is not None: args.append("--green='{}'".format(green))
        if blue is not None: args.append("--blue='{}'".format(blue))
        if composite is not None: args.append("--composite='{}'".format(composite))
        args.append("--intensity='{}'".format(intensity))
        args.append("--hue='{}'".format(hue))
        args.append("--saturation='{}'".format(saturation))
        return self.run_tool('rgb_to_ihs', args, callback) # returns 1 if error

    def split_colour_composite(self, i, red=None, green=None, blue=None, callback=None):
        """This tool splits an RGB colour composite image into seperate multispectral images.

        Keyword arguments:

        i -- Input colour composite image file. 
        red -- Output red band file. 
        green -- Output green band file. 
        blue -- Output blue band file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        if red is not None: args.append("--red='{}'".format(red))
        if green is not None: args.append("--green='{}'".format(green))
        if blue is not None: args.append("--blue='{}'".format(blue))
        return self.run_tool('split_colour_composite', args, callback) # returns 1 if error

    def thicken_raster_line(self, i, output, callback=None):
        """Thickens single-cell wide lines within a raster image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('thicken_raster_line', args, callback) # returns 1 if error

    def tophat_transform(self, i, output, filterx=11, filtery=11, variant="white", callback=None):
        """Performs either a white or black top-hat transform on an input image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        variant -- Optional variant value. Options include 'white' and 'black'. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        args.append("--variant={}".format(variant))
        return self.run_tool('tophat_transform', args, callback) # returns 1 if error

    def write_function_memory_insertion(self, input1, input2, output, input3=None, callback=None):
        """Performs a write function memory insertion for single-band multi-date change detection.

        Keyword arguments:

        input1 -- Input raster file associated with the first date. 
        input2 -- Input raster file associated with the second date. 
        input3 -- Optional input raster file associated with the third date. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        if input3 is not None: args.append("--input3='{}'".format(input3))
        args.append("--output='{}'".format(output))
        return self.run_tool('write_function_memory_insertion', args, callback) # returns 1 if error

    ##################################
    # Image Processing Tools/Filters #
    ##################################

    def adaptive_filter(self, i, output, filterx=11, filtery=11, threshold=2.0, callback=None):
        """Performs an adaptive filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        threshold -- Difference from mean threshold, in standard deviations. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        args.append("--threshold={}".format(threshold))
        return self.run_tool('adaptive_filter', args, callback) # returns 1 if error

    def bilateral_filter(self, i, output, sigma_dist=0.75, sigma_int=1.0, callback=None):
        """A bilateral filter is an edge-preserving smoothing filter introduced by Tomasi and Manduchi (1998).

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        sigma_dist -- Standard deviation in distance in pixels. 
        sigma_int -- Standard deviation in intensity in pixels. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--sigma_dist={}".format(sigma_dist))
        args.append("--sigma_int={}".format(sigma_int))
        return self.run_tool('bilateral_filter', args, callback) # returns 1 if error

    def conservative_smoothing_filter(self, i, output, filterx=3, filtery=3, callback=None):
        """Performs a conservative-smoothing filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('conservative_smoothing_filter', args, callback) # returns 1 if error

    def corner_detection(self, i, output, callback=None):
        """Identifies corner patterns in boolean images using hit-and-miss pattern matching.

        Keyword arguments:

        i -- Input boolean image. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('corner_detection', args, callback) # returns 1 if error

    def diff_of_gaussian_filter(self, i, output, sigma1=2.0, sigma2=4.0, callback=None):
        """Performs a Difference of Gaussian (DoG) filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        sigma1 -- Standard deviation distance in pixels. 
        sigma2 -- Standard deviation distance in pixels. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--sigma1={}".format(sigma1))
        args.append("--sigma2={}".format(sigma2))
        return self.run_tool('diff_of_gaussian_filter', args, callback) # returns 1 if error

    def diversity_filter(self, i, output, filterx=11, filtery=11, callback=None):
        """Assigns each cell in the output grid the number of different values in a moving window centred on each grid cell in the input raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('diversity_filter', args, callback) # returns 1 if error

    def edge_preserving_mean_filter(self, i, output, threshold, filter=11, callback=None):
        """Performs a simple edge-preserving mean filter on an input image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filter -- Size of the filter kernel. 
        threshold -- Maximum difference in values. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filter={}".format(filter))
        args.append("--threshold='{}'".format(threshold))
        return self.run_tool('edge_preserving_mean_filter', args, callback) # returns 1 if error

    def emboss_filter(self, i, output, direction="n", clip=0.0, callback=None):
        """Performs an emboss filter on an image, similar to a hillshade operation.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        direction -- Direction of reflection; options include 'n', 's', 'e', 'w', 'ne', 'se', 'nw', 'sw'. 
        clip -- Optional amount to clip the distribution tails by, in percent. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--direction={}".format(direction))
        args.append("--clip={}".format(clip))
        return self.run_tool('emboss_filter', args, callback) # returns 1 if error

    def fast_almost_gaussian_filter(self, i, output, sigma=1.8, callback=None):
        """Performs a fast approximate Gaussian filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        sigma -- Standard deviation distance in pixels. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--sigma={}".format(sigma))
        return self.run_tool('fast_almost_gaussian_filter', args, callback) # returns 1 if error

    def gaussian_filter(self, i, output, sigma=0.75, callback=None):
        """Performs a Gaussian filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        sigma -- Standard deviation distance in pixels. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--sigma={}".format(sigma))
        return self.run_tool('gaussian_filter', args, callback) # returns 1 if error

    def high_pass_filter(self, i, output, filterx=11, filtery=11, callback=None):
        """Performs a high-pass filter on an input image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('high_pass_filter', args, callback) # returns 1 if error

    def high_pass_median_filter(self, i, output, filterx=11, filtery=11, sig_digits=2, callback=None):
        """Performs a high pass median filter on an input image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        sig_digits -- Number of significant digits. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        args.append("--sig_digits={}".format(sig_digits))
        return self.run_tool('high_pass_median_filter', args, callback) # returns 1 if error

    def k_nearest_mean_filter(self, i, output, filterx=11, filtery=11, k=5, callback=None):
        """A k-nearest mean filter is a type of edge-preserving smoothing filter.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        k -- k-value in pixels; this is the number of nearest-valued neighbours to use. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        args.append("-k={}".format(k))
        return self.run_tool('k_nearest_mean_filter', args, callback) # returns 1 if error

    def laplacian_filter(self, i, output, variant="3x3(1)", clip=0.0, callback=None):
        """Performs a Laplacian filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        variant -- Optional variant value. Options include 3x3(1), 3x3(2), 3x3(3), 3x3(4), 5x5(1), and 5x5(2) (default is 3x3(1)). 
        clip -- Optional amount to clip the distribution tails by, in percent. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--variant={}".format(variant))
        args.append("--clip={}".format(clip))
        return self.run_tool('laplacian_filter', args, callback) # returns 1 if error

    def laplacian_of_gaussian_filter(self, i, output, sigma=0.75, callback=None):
        """Performs a Laplacian-of-Gaussian (LoG) filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        sigma -- Standard deviation in pixels. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--sigma={}".format(sigma))
        return self.run_tool('laplacian_of_gaussian_filter', args, callback) # returns 1 if error

    def lee_sigma_filter(self, i, output, filterx=11, filtery=11, sigma=10.0, m=5.0, callback=None):
        """Performs a Lee (Sigma) smoothing filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        sigma -- Sigma value should be related to the standarad deviation of the distribution of image speckle noise. 
        m -- M-threshold value the minimum allowable number of pixels within the intensity range. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        args.append("--sigma={}".format(sigma))
        args.append("-m={}".format(m))
        return self.run_tool('lee_sigma_filter', args, callback) # returns 1 if error

    def line_detection_filter(self, i, output, variant="vertical", absvals=False, clip=0.0, callback=None):
        """Performs a line-detection filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        variant -- Optional variant value. Options include 'v' (vertical), 'h' (horizontal), '45', and '135' (default is 'v'). 
        absvals -- Optional flag indicating whether outputs should be absolute values. 
        clip -- Optional amount to clip the distribution tails by, in percent. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--variant={}".format(variant))
        if absvals: args.append("--absvals")
        args.append("--clip={}".format(clip))
        return self.run_tool('line_detection_filter', args, callback) # returns 1 if error

    def majority_filter(self, i, output, filterx=11, filtery=11, callback=None):
        """Assigns each cell in the output grid the most frequently occurring value (mode) in a moving window centred on each grid cell in the input raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('majority_filter', args, callback) # returns 1 if error

    def maximum_filter(self, i, output, filterx=11, filtery=11, callback=None):
        """Assigns each cell in the output grid the maximum value in a moving window centred on each grid cell in the input raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('maximum_filter', args, callback) # returns 1 if error

    def mean_filter(self, i, output, filterx=3, filtery=3, callback=None):
        """Performs a mean filter (low-pass filter) on an input image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('mean_filter', args, callback) # returns 1 if error

    def median_filter(self, i, output, filterx=11, filtery=11, sig_digits=2, callback=None):
        """Performs a median filter on an input image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        sig_digits -- Number of significant digits. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        args.append("--sig_digits={}".format(sig_digits))
        return self.run_tool('median_filter', args, callback) # returns 1 if error

    def minimum_filter(self, i, output, filterx=11, filtery=11, callback=None):
        """Assigns each cell in the output grid the minimum value in a moving window centred on each grid cell in the input raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('minimum_filter', args, callback) # returns 1 if error

    def olympic_filter(self, i, output, filterx=11, filtery=11, callback=None):
        """Performs an olympic smoothing filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('olympic_filter', args, callback) # returns 1 if error

    def percentile_filter(self, i, output, filterx=11, filtery=11, sig_digits=2, callback=None):
        """Performs a percentile filter on an input image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        sig_digits -- Number of significant digits. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        args.append("--sig_digits={}".format(sig_digits))
        return self.run_tool('percentile_filter', args, callback) # returns 1 if error

    def prewitt_filter(self, i, output, clip=0.0, callback=None):
        """Performs a Prewitt edge-detection filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        clip -- Optional amount to clip the distribution tails by, in percent. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--clip={}".format(clip))
        return self.run_tool('prewitt_filter', args, callback) # returns 1 if error

    def range_filter(self, i, output, filterx=11, filtery=11, callback=None):
        """Assigns each cell in the output grid the range of values in a moving window centred on each grid cell in the input raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('range_filter', args, callback) # returns 1 if error

    def roberts_cross_filter(self, i, output, clip=0.0, callback=None):
        """Performs a Robert's cross edge-detection filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        clip -- Optional amount to clip the distribution tails by, in percent. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--clip={}".format(clip))
        return self.run_tool('roberts_cross_filter', args, callback) # returns 1 if error

    def scharr_filter(self, i, output, clip=0.0, callback=None):
        """Performs a Scharr edge-detection filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        clip -- Optional amount to clip the distribution tails by, in percent. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--clip={}".format(clip))
        return self.run_tool('scharr_filter', args, callback) # returns 1 if error

    def sobel_filter(self, i, output, variant="3x3", clip=0.0, callback=None):
        """Performs a Sobel edge-detection filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        variant -- Optional variant value. Options include 3x3 and 5x5 (default is 3x3). 
        clip -- Optional amount to clip the distribution tails by, in percent (default is 0.0). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--variant={}".format(variant))
        args.append("--clip={}".format(clip))
        return self.run_tool('sobel_filter', args, callback) # returns 1 if error

    def standard_deviation_filter(self, i, output, filterx=11, filtery=11, callback=None):
        """Assigns each cell in the output grid the standard deviation of values in a moving window centred on each grid cell in the input raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('standard_deviation_filter', args, callback) # returns 1 if error

    def total_filter(self, i, output, filterx=11, filtery=11, callback=None):
        """Performs a total filter on an input image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        filterx -- Size of the filter kernel in the x-direction. 
        filtery -- Size of the filter kernel in the y-direction. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--filterx={}".format(filterx))
        args.append("--filtery={}".format(filtery))
        return self.run_tool('total_filter', args, callback) # returns 1 if error

    def unsharp_masking(self, i, output, sigma=0.75, amount=100.0, threshold=0.0, callback=None):
        """An image sharpening technique that enhances edges.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        sigma -- Standard deviation distance in pixels. 
        amount -- A percentage and controls the magnitude of each overshoot. 
        threshold -- Controls the minimal brightness change that will be sharpened. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--sigma={}".format(sigma))
        args.append("--amount={}".format(amount))
        args.append("--threshold={}".format(threshold))
        return self.run_tool('unsharp_masking', args, callback) # returns 1 if error

    def user_defined_weights_filter(self, i, weights, output, center="center", normalize=False, callback=None):
        """Performs a user-defined weights filter on an image.

        Keyword arguments:

        i -- Input raster file. 
        weights -- Input weights file. 
        output -- Output raster file. 
        center -- Kernel center cell; options include 'center', 'upper-left', 'upper-right', 'lower-left', 'lower-right'. 
        normalize -- Normalize kernel weights? This can reduce edge effects and lessen the impact of data gaps (nodata) but is not suited when the kernel weights sum to zero. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--weights='{}'".format(weights))
        args.append("--output='{}'".format(output))
        args.append("--center={}".format(center))
        if normalize: args.append("--normalize")
        return self.run_tool('user_defined_weights_filter', args, callback) # returns 1 if error

    ############################################
    # Image Processing Tools/Image Enhancement #
    ############################################

    def balance_contrast_enhancement(self, i, output, band_mean=100.0, callback=None):
        """Performs a balance contrast enhancement on a colour-composite image of multispectral data.

        Keyword arguments:

        i -- Input colour composite image file. 
        output -- Output raster file. 
        band_mean -- Band mean value. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--band_mean={}".format(band_mean))
        return self.run_tool('balance_contrast_enhancement', args, callback) # returns 1 if error

    def correct_vignetting(self, i, pp, output, focal_length=304.8, image_width=228.6, n=4.0, callback=None):
        """Corrects the darkening of images towards corners.

        Keyword arguments:

        i -- Input raster file. 
        pp -- Input principal point file. 
        output -- Output raster file. 
        focal_length -- Camera focal length, in millimeters. 
        image_width -- Distance between photograph edges, in millimeters. 
        n -- The 'n' parameter. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--pp='{}'".format(pp))
        args.append("--output='{}'".format(output))
        args.append("--focal_length={}".format(focal_length))
        args.append("--image_width={}".format(image_width))
        args.append("-n={}".format(n))
        return self.run_tool('correct_vignetting', args, callback) # returns 1 if error

    def direct_decorrelation_stretch(self, i, output, k=0.5, clip=1.0, callback=None):
        """Performs a direct decorrelation stretch enhancement on a colour-composite image of multispectral data.

        Keyword arguments:

        i -- Input colour composite image file. 
        output -- Output raster file. 
        k -- Achromatic factor (k) ranges between 0 (no effect) and 1 (full saturation stretch), although typical values range from 0.3 to 0.7. 
        clip -- Optional percent to clip the upper tail by during the stretch. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("-k={}".format(k))
        args.append("--clip={}".format(clip))
        return self.run_tool('direct_decorrelation_stretch', args, callback) # returns 1 if error

    def gamma_correction(self, i, output, gamma=0.5, callback=None):
        """Performs a gamma correction on an input images.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        gamma -- Gamma value. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--gamma={}".format(gamma))
        return self.run_tool('gamma_correction', args, callback) # returns 1 if error

    def gaussian_contrast_stretch(self, i, output, num_tones=256, callback=None):
        """Performs a Gaussian contrast stretch on input images.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        num_tones -- Number of tones in the output image. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--num_tones={}".format(num_tones))
        return self.run_tool('gaussian_contrast_stretch', args, callback) # returns 1 if error

    def histogram_equalization(self, i, output, num_tones=256, callback=None):
        """Performs a histogram equalization contrast enhancment on an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        num_tones -- Number of tones in the output image. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--num_tones={}".format(num_tones))
        return self.run_tool('histogram_equalization', args, callback) # returns 1 if error

    def histogram_matching(self, i, histo_file, output, callback=None):
        """Alters the statistical distribution of a raster image matching it to a specified PDF.

        Keyword arguments:

        i -- Input raster file. 
        histo_file -- Input reference probability distribution function (pdf) text file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--histo_file='{}'".format(histo_file))
        args.append("--output='{}'".format(output))
        return self.run_tool('histogram_matching', args, callback) # returns 1 if error

    def histogram_matching_two_images(self, input1, input2, output, callback=None):
        """This tool alters the cumulative distribution function of a raster image to that of another image.

        Keyword arguments:

        input1 -- Input raster file to modify. 
        input2 -- Input reference raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('histogram_matching_two_images', args, callback) # returns 1 if error

    def min_max_contrast_stretch(self, i, output, min_val, max_val, num_tones=256, callback=None):
        """Performs a min-max contrast stretch on an input greytone image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        min_val -- Lower tail clip value. 
        max_val -- Upper tail clip value. 
        num_tones -- Number of tones in the output image. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--min_val='{}'".format(min_val))
        args.append("--max_val='{}'".format(max_val))
        args.append("--num_tones={}".format(num_tones))
        return self.run_tool('min_max_contrast_stretch', args, callback) # returns 1 if error

    def panchromatic_sharpening(self, pan, output, red=None, green=None, blue=None, composite=None, method="brovey", callback=None):
        """Increases the spatial resolution of image data by combining multispectral bands with panchromatic data.

        Keyword arguments:

        red -- Input red band image file. Optionally specified if colour-composite not specified. 
        green -- Input green band image file. Optionally specified if colour-composite not specified. 
        blue -- Input blue band image file. Optionally specified if colour-composite not specified. 
        composite -- Input colour-composite image file. Only used if individual bands are not specified. 
        pan -- Input panchromatic band file. 
        output -- Output colour composite file. 
        method -- Options include 'brovey' (default) and 'ihs'. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        if red is not None: args.append("--red='{}'".format(red))
        if green is not None: args.append("--green='{}'".format(green))
        if blue is not None: args.append("--blue='{}'".format(blue))
        if composite is not None: args.append("--composite='{}'".format(composite))
        args.append("--pan='{}'".format(pan))
        args.append("--output='{}'".format(output))
        args.append("--method={}".format(method))
        return self.run_tool('panchromatic_sharpening', args, callback) # returns 1 if error

    def percentage_contrast_stretch(self, i, output, clip=1.0, tail="both", num_tones=256, callback=None):
        """Performs a percentage linear contrast stretch on input images.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        clip -- Optional amount to clip the distribution tails by, in percent. 
        tail -- Specified which tails to clip; options include 'upper', 'lower', and 'both' (default is 'both'). 
        num_tones -- Number of tones in the output image. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--clip={}".format(clip))
        args.append("--tail={}".format(tail))
        args.append("--num_tones={}".format(num_tones))
        return self.run_tool('percentage_contrast_stretch', args, callback) # returns 1 if error

    def sigmoidal_contrast_stretch(self, i, output, cutoff=0.0, gain=1.0, num_tones=256, callback=None):
        """Performs a sigmoidal contrast stretch on input images.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        cutoff -- Cutoff value between 0.0 and 0.95. 
        gain -- Gain value. 
        num_tones -- Number of tones in the output image. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--cutoff={}".format(cutoff))
        args.append("--gain={}".format(gain))
        args.append("--num_tones={}".format(num_tones))
        return self.run_tool('sigmoidal_contrast_stretch', args, callback) # returns 1 if error

    def standard_deviation_contrast_stretch(self, i, output, stdev=2.0, num_tones=256, callback=None):
        """Performs a standard-deviation contrast stretch on input images.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        stdev -- Standard deviation clip value. 
        num_tones -- Number of tones in the output image. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--stdev={}".format(stdev))
        args.append("--num_tones={}".format(num_tones))
        return self.run_tool('standard_deviation_contrast_stretch', args, callback) # returns 1 if error

    ###############
    # LiDAR Tools #
    ###############

    def classify_buildings_in_lidar(self, i, buildings, output, callback=None):
        """Reclassifies a LiDAR points that lie within vector building footprints.

        Keyword arguments:

        i -- Input LiDAR file. 
        buildings -- Input vector polygons file. 
        output -- Output LiDAR file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--buildings='{}'".format(buildings))
        args.append("--output='{}'".format(output))
        return self.run_tool('classify_buildings_in_lidar', args, callback) # returns 1 if error

    def classify_overlap_points(self, i, output, resolution=2.0, filter=False, callback=None):
        """Classifies or filters LAS points in regions of overlapping flight lines.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        resolution -- The size of the square area used to evaluate nearby points in the LiDAR data. 
        filter -- Filter out points from overlapping flightlines? If false, overlaps will simply be classified. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--resolution={}".format(resolution))
        if filter: args.append("--filter")
        return self.run_tool('classify_overlap_points', args, callback) # returns 1 if error

    def clip_lidar_to_polygon(self, i, polygons, output, callback=None):
        """Clips a LiDAR point cloud to a vector polygon or polygons.

        Keyword arguments:

        i -- Input LiDAR file. 
        polygons -- Input vector polygons file. 
        output -- Output LiDAR file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--polygons='{}'".format(polygons))
        args.append("--output='{}'".format(output))
        return self.run_tool('clip_lidar_to_polygon', args, callback) # returns 1 if error

    def erase_polygon_from_lidar(self, i, polygons, output, callback=None):
        """Erases (cuts out) a vector polygon or polygons from a LiDAR point cloud.

        Keyword arguments:

        i -- Input LiDAR file. 
        polygons -- Input vector polygons file. 
        output -- Output LiDAR file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--polygons='{}'".format(polygons))
        args.append("--output='{}'".format(output))
        return self.run_tool('erase_polygon_from_lidar', args, callback) # returns 1 if error

    def filter_lidar_classes(self, i, output, exclude_cls=None, callback=None):
        """Removes points in a LAS file with certain specified class values.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        exclude_cls -- Optional exclude classes from interpolation; Valid class values range from 0 to 18, based on LAS specifications. Example, --exclude_cls='3,4,5,6,7,18'. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if exclude_cls is not None: args.append("--exclude_cls='{}'".format(exclude_cls))
        return self.run_tool('filter_lidar_classes', args, callback) # returns 1 if error

    def filter_lidar_scan_angles(self, i, output, threshold, callback=None):
        """Removes points in a LAS file with scan angles greater than a threshold.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        threshold -- Scan angle threshold. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--threshold='{}'".format(threshold))
        return self.run_tool('filter_lidar_scan_angles', args, callback) # returns 1 if error

    def find_flightline_edge_points(self, i, output, callback=None):
        """Identifies points along a flightline's edge in a LAS file.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('find_flightline_edge_points', args, callback) # returns 1 if error

    def flightline_overlap(self, i=None, output=None, resolution=1.0, callback=None):
        """Reads a LiDAR (LAS) point file and outputs a raster containing the number of overlapping flight lines in each grid cell.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output file. 
        resolution -- Output raster's grid resolution. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        if i is not None: args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        args.append("--resolution={}".format(resolution))
        return self.run_tool('flightline_overlap', args, callback) # returns 1 if error

    def height_above_ground(self, i=None, output=None, callback=None):
        """Normalizes a LiDAR point cloud, providing the height above the nearest ground-classified point.

        Keyword arguments:

        i -- Input LiDAR file (including extension). 
        output -- Output raster file (including extension). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        if i is not None: args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        return self.run_tool('height_above_ground', args, callback) # returns 1 if error

    def las_to_ascii(self, inputs, callback=None):
        """Converts one or more LAS files into ASCII text files.

        Keyword arguments:

        inputs -- Input LiDAR files. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        return self.run_tool('las_to_ascii', args, callback) # returns 1 if error

    def las_to_multipoint_shapefile(self, i=None, callback=None):
        """Converts one or more LAS files into MultipointZ vector Shapefiles. When the input parameter is not specified, the tool grids all LAS files contained within the working directory.

        Keyword arguments:

        i -- Input LiDAR file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        if i is not None: args.append("--input='{}'".format(i))
        return self.run_tool('las_to_multipoint_shapefile', args, callback) # returns 1 if error

    def las_to_shapefile(self, i=None, callback=None):
        """Converts one or more LAS files into a vector Shapefile of POINT ShapeType.

        Keyword arguments:

        i -- Input LiDAR file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        if i is not None: args.append("--input='{}'".format(i))
        return self.run_tool('las_to_shapefile', args, callback) # returns 1 if error

    def lidar_block_maximum(self, i=None, output=None, resolution=1.0, callback=None):
        """Creates a block-maximum raster from an input LAS file. When the input/output parameters are not specified, the tool grids all LAS files contained within the working directory.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output file. 
        resolution -- Output raster's grid resolution. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        if i is not None: args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        args.append("--resolution={}".format(resolution))
        return self.run_tool('lidar_block_maximum', args, callback) # returns 1 if error

    def lidar_block_minimum(self, i=None, output=None, resolution=1.0, callback=None):
        """Creates a block-minimum raster from an input LAS file. When the input/output parameters are not specified, the tool grids all LAS files contained within the working directory.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output file. 
        resolution -- Output raster's grid resolution. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        if i is not None: args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        args.append("--resolution={}".format(resolution))
        return self.run_tool('lidar_block_minimum', args, callback) # returns 1 if error

    def lidar_classify_subset(self, base, subset, output, subset_class, nonsubset_class=None, callback=None):
        """Classifies the values in one LiDAR point cloud that correpond with points in a subset cloud.

        Keyword arguments:

        base -- Input base LiDAR file. 
        subset -- Input subset LiDAR file. 
        output -- Output LiDAR file. 
        subset_class -- Subset point class value (must be 0-18; see LAS specifications). 
        nonsubset_class -- Non-subset point class value (must be 0-18; see LAS specifications). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--base='{}'".format(base))
        args.append("--subset='{}'".format(subset))
        args.append("--output='{}'".format(output))
        args.append("--subset_class='{}'".format(subset_class))
        if nonsubset_class is not None: args.append("--nonsubset_class='{}'".format(nonsubset_class))
        return self.run_tool('lidar_classify_subset', args, callback) # returns 1 if error

    def lidar_colourize(self, in_lidar, in_image, output, callback=None):
        """Adds the red-green-blue colour fields of a LiDAR (LAS) file based on an input image.

        Keyword arguments:

        in_lidar -- Input LiDAR file. 
        in_image -- Input colour image file. 
        output -- Output LiDAR file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--in_lidar='{}'".format(in_lidar))
        args.append("--in_image='{}'".format(in_image))
        args.append("--output='{}'".format(output))
        return self.run_tool('lidar_colourize', args, callback) # returns 1 if error

    def lidar_construct_vector_tin(self, i=None, output=None, returns="all", exclude_cls=None, minz=None, maxz=None, callback=None):
        """Creates a vector triangular irregular network (TIN) fitted to LiDAR points.

        Keyword arguments:

        i -- Input LiDAR file (including extension). 
        output -- Output raster file (including extension). 
        returns -- Point return types to include; options are 'all' (default), 'last', 'first'. 
        exclude_cls -- Optional exclude classes from interpolation; Valid class values range from 0 to 18, based on LAS specifications. Example, --exclude_cls='3,4,5,6,7,18'. 
        minz -- Optional minimum elevation for inclusion in interpolation. 
        maxz -- Optional maximum elevation for inclusion in interpolation. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        if i is not None: args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        args.append("--returns={}".format(returns))
        if exclude_cls is not None: args.append("--exclude_cls='{}'".format(exclude_cls))
        if minz is not None: args.append("--minz='{}'".format(minz))
        if maxz is not None: args.append("--maxz='{}'".format(maxz))
        return self.run_tool('lidar_construct_vector_tin', args, callback) # returns 1 if error

    def lidar_elevation_slice(self, i, output, minz=None, maxz=None, cls=False, inclassval=2, outclassval=1, callback=None):
        """Outputs all of the points within a LiDAR (LAS) point file that lie between a specified elevation range.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        minz -- Minimum elevation value (optional). 
        maxz -- Maximum elevation value (optional). 
        cls -- Optional boolean flag indicating whether points outside the range should be retained in output but reclassified. 
        inclassval -- Optional parameter specifying the class value assigned to points within the slice. 
        outclassval -- Optional parameter specifying the class value assigned to points within the slice. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if minz is not None: args.append("--minz='{}'".format(minz))
        if maxz is not None: args.append("--maxz='{}'".format(maxz))
        if cls: args.append("--class")
        args.append("--inclassval={}".format(inclassval))
        args.append("--outclassval={}".format(outclassval))
        return self.run_tool('lidar_elevation_slice', args, callback) # returns 1 if error

    def lidar_ground_point_filter(self, i, output, radius=2.0, min_neighbours=0, slope_threshold=45.0, height_threshold=1.0, classify=True, slope_norm=True, height_above_ground=False, callback=None):
        """Identifies ground points within LiDAR dataset using a slope-based method.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        radius -- Search Radius. 
        min_neighbours -- The minimum number of neighbouring points within search areas. If fewer points than this threshold are idenfied during the fixed-radius search, a subsequent kNN search is performed to identify the k number of neighbours. 
        slope_threshold -- Maximum inter-point slope to be considered an off-terrain point. 
        height_threshold -- Inter-point height difference to be considered an off-terrain point. 
        classify -- Classify points as ground (2) or off-ground (1). 
        slope_norm -- Perform initial ground slope normalization?. 
        height_above_ground -- Transform output to height above average ground elevation?. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--radius={}".format(radius))
        args.append("--min_neighbours={}".format(min_neighbours))
        args.append("--slope_threshold={}".format(slope_threshold))
        args.append("--height_threshold={}".format(height_threshold))
        if classify: args.append("--classify")
        if slope_norm: args.append("--slope_norm")
        if height_above_ground: args.append("--height_above_ground")
        return self.run_tool('lidar_ground_point_filter', args, callback) # returns 1 if error

    def lidar_hex_binning(self, i, output, width, orientation="horizontal", callback=None):
        """Hex-bins a set of LiDAR points.

        Keyword arguments:

        i -- Input base file. 
        output -- Output vector polygon file. 
        width -- The grid cell width. 
        orientation -- Grid Orientation, 'horizontal' or 'vertical'. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--width='{}'".format(width))
        args.append("--orientation={}".format(orientation))
        return self.run_tool('lidar_hex_binning', args, callback) # returns 1 if error

    def lidar_hillshade(self, i, output, azimuth=315.0, altitude=30.0, radius=1.0, callback=None):
        """Calculates a hillshade value for points within a LAS file and stores these data in the RGB field.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output file. 
        azimuth -- Illumination source azimuth in degrees. 
        altitude -- Illumination source altitude in degrees. 
        radius -- Search Radius. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--azimuth={}".format(azimuth))
        args.append("--altitude={}".format(altitude))
        args.append("--radius={}".format(radius))
        return self.run_tool('lidar_hillshade', args, callback) # returns 1 if error

    def lidar_histogram(self, i, output, parameter="elevation", clip=1.0, callback=None):
        """Creates a histogram of LiDAR data.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        parameter -- Parameter; options are 'elevation' (default), 'intensity', 'scan angle', 'class'. 
        clip -- Amount to clip distribution tails (in percent). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--parameter={}".format(parameter))
        args.append("--clip={}".format(clip))
        return self.run_tool('lidar_histogram', args, callback) # returns 1 if error

    def lidar_idw_interpolation(self, i=None, output=None, parameter="elevation", returns="all", resolution=1.0, weight=1.0, radius=2.5, exclude_cls=None, minz=None, maxz=None, callback=None):
        """Interpolates LAS files using an inverse-distance weighted (IDW) scheme. When the input/output parameters are not specified, the tool interpolates all LAS files contained within the working directory.

        Keyword arguments:

        i -- Input LiDAR file (including extension). 
        output -- Output raster file (including extension). 
        parameter -- Interpolation parameter; options are 'elevation' (default), 'intensity', 'class', 'return_number', 'number_of_returns', 'scan angle', 'rgb', 'user data'. 
        returns -- Point return types to include; options are 'all' (default), 'last', 'first'. 
        resolution -- Output raster's grid resolution. 
        weight -- IDW weight value. 
        radius -- Search Radius. 
        exclude_cls -- Optional exclude classes from interpolation; Valid class values range from 0 to 18, based on LAS specifications. Example, --exclude_cls='3,4,5,6,7,18'. 
        minz -- Optional minimum elevation for inclusion in interpolation. 
        maxz -- Optional maximum elevation for inclusion in interpolation. 
        callback -- Custom function for handling tool text outputs.
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
        return self.run_tool('lidar_idw_interpolation', args, callback) # returns 1 if error

    def lidar_info(self, i, output=None, vlr=True, geokeys=True, callback=None):
        """Prints information about a LiDAR (LAS) dataset, including header, point return frequency, and classification data and information about the variable length records (VLRs) and geokeys.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output HTML file for summary report. 
        vlr -- Flag indicating whether or not to print the variable length records (VLRs). 
        geokeys -- Flag indicating whether or not to print the geokeys. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        if vlr: args.append("--vlr")
        if geokeys: args.append("--geokeys")
        return self.run_tool('lidar_info', args, callback) # returns 1 if error

    def lidar_join(self, inputs, output, callback=None):
        """Joins multiple LiDAR (LAS) files into a single LAS file.

        Keyword arguments:

        inputs -- Input LiDAR files. 
        output -- Output LiDAR file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('lidar_join', args, callback) # returns 1 if error

    def lidar_kappa_index(self, input1, input2, output, class_accuracy, resolution=1.0, callback=None):
        """Performs a kappa index of agreement (KIA) analysis on the classifications of two LAS files.

        Keyword arguments:

        input1 -- Input LiDAR classification file. 
        input2 -- Input LiDAR reference file. 
        output -- Output HTML file. 
        class_accuracy -- Output classification accuracy raster file. 
        resolution -- Output raster's grid resolution. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        args.append("--class_accuracy='{}'".format(class_accuracy))
        args.append("--resolution={}".format(resolution))
        return self.run_tool('lidar_kappa_index', args, callback) # returns 1 if error

    def lidar_nearest_neighbour_gridding(self, i=None, output=None, parameter="elevation", returns="all", resolution=1.0, radius=2.5, exclude_cls=None, minz=None, maxz=None, callback=None):
        """Grids LAS files using nearest-neighbour scheme. When the input/output parameters are not specified, the tool grids all LAS files contained within the working directory.

        Keyword arguments:

        i -- Input LiDAR file (including extension). 
        output -- Output raster file (including extension). 
        parameter -- Interpolation parameter; options are 'elevation' (default), 'intensity', 'class', 'return_number', 'number_of_returns', 'scan angle', 'rgb', 'user data'. 
        returns -- Point return types to include; options are 'all' (default), 'last', 'first'. 
        resolution -- Output raster's grid resolution. 
        radius -- Search Radius. 
        exclude_cls -- Optional exclude classes from interpolation; Valid class values range from 0 to 18, based on LAS specifications. Example, --exclude_cls='3,4,5,6,7,18'. 
        minz -- Optional minimum elevation for inclusion in interpolation. 
        maxz -- Optional maximum elevation for inclusion in interpolation. 
        callback -- Custom function for handling tool text outputs.
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
        return self.run_tool('lidar_nearest_neighbour_gridding', args, callback) # returns 1 if error

    def lidar_point_density(self, i=None, output=None, returns="all", resolution=1.0, radius=2.5, exclude_cls=None, minz=None, maxz=None, callback=None):
        """Calculates the spatial pattern of point density for a LiDAR data set. When the input/output parameters are not specified, the tool grids all LAS files contained within the working directory.

        Keyword arguments:

        i -- Input LiDAR file (including extension). 
        output -- Output raster file (including extension). 
        returns -- Point return types to include; options are 'all' (default), 'last', 'first'. 
        resolution -- Output raster's grid resolution. 
        radius -- Search radius. 
        exclude_cls -- Optional exclude classes from interpolation; Valid class values range from 0 to 18, based on LAS specifications. Example, --exclude_cls='3,4,5,6,7,18'. 
        minz -- Optional minimum elevation for inclusion in interpolation. 
        maxz -- Optional maximum elevation for inclusion in interpolation. 
        callback -- Custom function for handling tool text outputs.
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
        return self.run_tool('lidar_point_density', args, callback) # returns 1 if error

    def lidar_point_stats(self, i=None, resolution=1.0, num_points=True, num_pulses=False, avg_points_per_pulse=True, z_range=False, intensity_range=False, predom_class=False, callback=None):
        """Creates several rasters summarizing the distribution of LAS point data. When the input/output parameters are not specified, the tool works on all LAS files contained within the working directory.

        Keyword arguments:

        i -- Input LiDAR file. 
        resolution -- Output raster's grid resolution. 
        num_points -- Flag indicating whether or not to output the number of points (returns) raster. 
        num_pulses -- Flag indicating whether or not to output the number of pulses raster. 
        avg_points_per_pulse -- Flag indicating whether or not to output the average number of points (returns) per pulse raster. 
        z_range -- Flag indicating whether or not to output the elevation range raster. 
        intensity_range -- Flag indicating whether or not to output the intensity range raster. 
        predom_class -- Flag indicating whether or not to output the predominant classification raster. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        if i is not None: args.append("--input='{}'".format(i))
        args.append("--resolution={}".format(resolution))
        if num_points: args.append("--num_points")
        if num_pulses: args.append("--num_pulses")
        if avg_points_per_pulse: args.append("--avg_points_per_pulse")
        if z_range: args.append("--z_range")
        if intensity_range: args.append("--intensity_range")
        if predom_class: args.append("--predom_class")
        return self.run_tool('lidar_point_stats', args, callback) # returns 1 if error

    def lidar_ransac_planes(self, i, output, radius=2.0, num_iter=50, num_samples=5, threshold=0.35, model_size=8, max_slope=80.0, classify=False, callback=None):
        """Performs a RANSAC analysis to identify points within a LiDAR point cloud that belong to linear planes.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        radius -- Search Radius. 
        num_iter -- Number of iterations. 
        num_samples -- Number of sample points on which to build the model. 
        threshold -- Threshold used to determine inlier points. 
        model_size -- Acceptable model size. 
        max_slope -- Maximum planar slope. 
        classify -- Classify points as ground (2) or off-ground (1). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--radius={}".format(radius))
        args.append("--num_iter={}".format(num_iter))
        args.append("--num_samples={}".format(num_samples))
        args.append("--threshold={}".format(threshold))
        args.append("--model_size={}".format(model_size))
        args.append("--max_slope={}".format(max_slope))
        if classify: args.append("--classify")
        return self.run_tool('lidar_ransac_planes', args, callback) # returns 1 if error

    def lidar_rbf_interpolation(self, i=None, output=None, parameter="elevation", returns="all", resolution=1.0, num_points=20, exclude_cls=None, minz=None, maxz=None, func_type="ThinPlateSpline", poly_order="none", weight=5, callback=None):
        """Interpolates LAS files using a radial basis function (RBF) scheme. When the input/output parameters are not specified, the tool interpolates all LAS files contained within the working directory.

        Keyword arguments:

        i -- Input LiDAR file (including extension). 
        output -- Output raster file (including extension). 
        parameter -- Interpolation parameter; options are 'elevation' (default), 'intensity', 'class', 'return_number', 'number_of_returns', 'scan angle', 'rgb', 'user data'. 
        returns -- Point return types to include; options are 'all' (default), 'last', 'first'. 
        resolution -- Output raster's grid resolution. 
        num_points -- Number of points. 
        exclude_cls -- Optional exclude classes from interpolation; Valid class values range from 0 to 18, based on LAS specifications. Example, --exclude_cls='3,4,5,6,7,18'. 
        minz -- Optional minimum elevation for inclusion in interpolation. 
        maxz -- Optional maximum elevation for inclusion in interpolation. 
        func_type -- Radial basis function type; options are 'ThinPlateSpline' (default), 'PolyHarmonic', 'Gaussian', 'MultiQuadric', 'InverseMultiQuadric'. 
        poly_order -- Polynomial order; options are 'none' (default), 'constant', 'affine'. 
        weight -- Weight parameter used in basis function. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        if i is not None: args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        args.append("--parameter={}".format(parameter))
        args.append("--returns={}".format(returns))
        args.append("--resolution={}".format(resolution))
        args.append("--num_points={}".format(num_points))
        if exclude_cls is not None: args.append("--exclude_cls='{}'".format(exclude_cls))
        if minz is not None: args.append("--minz='{}'".format(minz))
        if maxz is not None: args.append("--maxz='{}'".format(maxz))
        args.append("--func_type={}".format(func_type))
        args.append("--poly_order={}".format(poly_order))
        args.append("--weight={}".format(weight))
        return self.run_tool('lidar_rbf_interpolation', args, callback) # returns 1 if error

    def lidar_remove_duplicates(self, i, output, include_z=False, callback=None):
        """Removes duplicate points from a LiDAR data set.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        include_z -- Include z-values in point comparison?. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if include_z: args.append("--include_z")
        return self.run_tool('lidar_remove_duplicates', args, callback) # returns 1 if error

    def lidar_remove_outliers(self, i, output, radius=2.0, elev_diff=50.0, use_median=False, classify=True, callback=None):
        """Removes outliers (high and low points) in a LiDAR point cloud.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        radius -- Search Radius. 
        elev_diff -- Max. elevation difference. 
        use_median -- Optional flag indicating whether to use the difference from median elevation rather than mean. 
        classify -- Classify points as ground (2) or off-ground (1). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--radius={}".format(radius))
        args.append("--elev_diff={}".format(elev_diff))
        if use_median: args.append("--use_median")
        if classify: args.append("--classify")
        return self.run_tool('lidar_remove_outliers', args, callback) # returns 1 if error

    def lidar_segmentation(self, i, output, radius=2.0, num_iter=50, num_samples=10, threshold=0.15, model_size=15, max_slope=80.0, norm_diff=10.0, maxzdiff=1.0, classes=False, ground=False, callback=None):
        """Segments a LiDAR point cloud based on differences in the orientation of fitted planar surfaces and point proximity.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        radius -- Search Radius. 
        num_iter -- Number of iterations. 
        num_samples -- Number of sample points on which to build the model. 
        threshold -- Threshold used to determine inlier points. 
        model_size -- Acceptable model size. 
        max_slope -- Maximum planar slope. 
        norm_diff -- Maximum difference in normal vectors, in degrees. 
        maxzdiff -- Maximum difference in elevation (z units) between neighbouring points of the same segment. 
        classes -- Segments don't cross class boundaries. 
        ground -- Classify the largest segment as ground points?. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--radius={}".format(radius))
        args.append("--num_iter={}".format(num_iter))
        args.append("--num_samples={}".format(num_samples))
        args.append("--threshold={}".format(threshold))
        args.append("--model_size={}".format(model_size))
        args.append("--max_slope={}".format(max_slope))
        args.append("--norm_diff={}".format(norm_diff))
        args.append("--maxzdiff={}".format(maxzdiff))
        if classes: args.append("--classes")
        if ground: args.append("--ground")
        return self.run_tool('lidar_segmentation', args, callback) # returns 1 if error

    def lidar_segmentation_based_filter(self, i, output, radius=5.0, norm_diff=2.0, maxzdiff=1.0, classify=False, callback=None):
        """Identifies ground points within LiDAR point clouds using a segmentation based approach.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output file. 
        radius -- Search Radius. 
        norm_diff -- Maximum difference in normal vectors, in degrees. 
        maxzdiff -- Maximum difference in elevation (z units) between neighbouring points of the same segment. 
        classify -- Classify points as ground (2) or off-ground (1). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--radius={}".format(radius))
        args.append("--norm_diff={}".format(norm_diff))
        args.append("--maxzdiff={}".format(maxzdiff))
        if classify: args.append("--classify")
        return self.run_tool('lidar_segmentation_based_filter', args, callback) # returns 1 if error

    def lidar_thin(self, i, output, resolution=2.0, method="lowest", save_filtered=False, callback=None):
        """Thins a LiDAR point cloud, reducing point density.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        resolution -- The size of the square area used to evaluate nearby points in the LiDAR data. 
        method -- Point selection method; options are 'first', 'last', 'lowest' (default), 'highest', 'nearest'. 
        save_filtered -- Save filtered points to seperate file?. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--resolution={}".format(resolution))
        args.append("--method={}".format(method))
        if save_filtered: args.append("--save_filtered")
        return self.run_tool('lidar_thin', args, callback) # returns 1 if error

    def lidar_thin_high_density(self, i, output, density, resolution=1.0, save_filtered=False, callback=None):
        """Thins points from high density areas within a LiDAR point cloud.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        resolution -- Output raster's grid resolution. 
        density -- Max. point density (points / m^3). 
        save_filtered -- Save filtered points to seperate file?. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--resolution={}".format(resolution))
        args.append("--density='{}'".format(density))
        if save_filtered: args.append("--save_filtered")
        return self.run_tool('lidar_thin_high_density', args, callback) # returns 1 if error

    def lidar_tile(self, i, width=1000.0, height=1000.0, origin_x=0.0, origin_y=0.0, min_points=2, callback=None):
        """Tiles a LiDAR LAS file into multiple LAS files.

        Keyword arguments:

        i -- Input LiDAR file. 
        width -- Width of tiles in the X dimension; default 1000.0. 
        height -- Height of tiles in the Y dimension. 
        origin_x -- Origin point X coordinate for tile grid. 
        origin_y -- Origin point Y coordinate for tile grid. 
        min_points -- Minimum number of points contained in a tile for it to be saved. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--width={}".format(width))
        args.append("--height={}".format(height))
        args.append("--origin_x={}".format(origin_x))
        args.append("--origin_y={}".format(origin_y))
        args.append("--min_points={}".format(min_points))
        return self.run_tool('lidar_tile', args, callback) # returns 1 if error

    def lidar_tile_footprint(self, output, i=None, hull=False, callback=None):
        """Creates a vector polygon of the convex hull of a LiDAR point cloud. When the input/output parameters are not specified, the tool works with all LAS files contained within the working directory.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output vector polygon file. 
        hull -- Identify the convex hull around points. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        if i is not None: args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if hull: args.append("--hull")
        return self.run_tool('lidar_tile_footprint', args, callback) # returns 1 if error

    def lidar_tin_gridding(self, i=None, output=None, parameter="elevation", returns="all", resolution=1.0, exclude_cls=None, minz=None, maxz=None, max_triangle_edge_length=None, callback=None):
        """Creates a raster grid based on a Delaunay triangular irregular network (TIN) fitted to LiDAR points.

        Keyword arguments:

        i -- Input LiDAR file (including extension). 
        output -- Output raster file (including extension). 
        parameter -- Interpolation parameter; options are 'elevation' (default), 'intensity', 'class', 'return_number', 'number_of_returns', 'scan angle', 'rgb', 'user data'. 
        returns -- Point return types to include; options are 'all' (default), 'last', 'first'. 
        resolution -- Output raster's grid resolution. 
        exclude_cls -- Optional exclude classes from interpolation; Valid class values range from 0 to 18, based on LAS specifications. Example, --exclude_cls='3,4,5,6,7,18'. 
        minz -- Optional minimum elevation for inclusion in interpolation. 
        maxz -- Optional maximum elevation for inclusion in interpolation. 
        max_triangle_edge_length -- Optional maximum triangle edge length; triangles larger than this size will not be gridded. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        if i is not None: args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        args.append("--parameter={}".format(parameter))
        args.append("--returns={}".format(returns))
        args.append("--resolution={}".format(resolution))
        if exclude_cls is not None: args.append("--exclude_cls='{}'".format(exclude_cls))
        if minz is not None: args.append("--minz='{}'".format(minz))
        if maxz is not None: args.append("--maxz='{}'".format(maxz))
        if max_triangle_edge_length is not None: args.append("--max_triangle_edge_length='{}'".format(max_triangle_edge_length))
        return self.run_tool('lidar_tin_gridding', args, callback) # returns 1 if error

    def lidar_tophat_transform(self, i, output, radius=1.0, callback=None):
        """Performs a white top-hat transform on a Lidar dataset; as an estimate of height above ground, this is useful for modelling the vegetation canopy.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        radius -- Search Radius. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--radius={}".format(radius))
        return self.run_tool('lidar_tophat_transform', args, callback) # returns 1 if error

    def normal_vectors(self, i, output, radius=1.0, callback=None):
        """Calculates normal vectors for points within a LAS file and stores these data (XYZ vector components) in the RGB field.

        Keyword arguments:

        i -- Input LiDAR file. 
        output -- Output LiDAR file. 
        radius -- Search Radius. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--radius={}".format(radius))
        return self.run_tool('normal_vectors', args, callback) # returns 1 if error

    def select_tiles_by_polygon(self, indir, outdir, polygons, callback=None):
        """Copies LiDAR tiles overlapping with a polygon into an output directory.

        Keyword arguments:

        indir -- Input LAS file source directory. 
        outdir -- Output directory into which LAS files within the polygon are copied. 
        polygons -- Input vector polygons file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--indir='{}'".format(indir))
        args.append("--outdir='{}'".format(outdir))
        args.append("--polygons='{}'".format(polygons))
        return self.run_tool('select_tiles_by_polygon', args, callback) # returns 1 if error

    ########################
    # Math and Stats Tools #
    ########################

    def And(self, input1, input2, output, callback=None):
        """Performs a logical AND operator on two Boolean raster images.

        Keyword arguments:

        input1 -- Input raster file. 
        input2 -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('and', args, callback) # returns 1 if error

    def Not(self, input1, input2, output, callback=None):
        """Performs a logical NOT operator on two Boolean raster images.

        Keyword arguments:

        input1 -- Input raster file. 
        input2 -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('not', args, callback) # returns 1 if error

    def Or(self, input1, input2, output, callback=None):
        """Performs a logical OR operator on two Boolean raster images.

        Keyword arguments:

        input1 -- Input raster file. 
        input2 -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('or', args, callback) # returns 1 if error

    def absolute_value(self, i, output, callback=None):
        """Calculates the absolute value of every cell in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('absolute_value', args, callback) # returns 1 if error

    def add(self, input1, input2, output, callback=None):
        """Performs an addition operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('add', args, callback) # returns 1 if error

    def anova(self, i, features, output, callback=None):
        """Performs an analysis of variance (ANOVA) test on a raster dataset.

        Keyword arguments:

        i -- Input raster file. 
        features -- Feature definition (or class) raster. 
        output -- Output HTML file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--features='{}'".format(features))
        args.append("--output='{}'".format(output))
        return self.run_tool('anova', args, callback) # returns 1 if error

    def arc_cos(self, i, output, callback=None):
        """Returns the inverse cosine (arccos) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('arc_cos', args, callback) # returns 1 if error

    def arc_sin(self, i, output, callback=None):
        """Returns the inverse sine (arcsin) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('arc_sin', args, callback) # returns 1 if error

    def arc_tan(self, i, output, callback=None):
        """Returns the inverse tangent (arctan) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('arc_tan', args, callback) # returns 1 if error

    def arcosh(self, i, output, callback=None):
        """Returns the inverse hyperbolic cosine (arcosh) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('arcosh', args, callback) # returns 1 if error

    def arsinh(self, i, output, callback=None):
        """Returns the inverse hyperbolic sine (arsinh) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('arsinh', args, callback) # returns 1 if error

    def artanh(self, i, output, callback=None):
        """Returns the inverse hyperbolic tangent (arctanh) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('artanh', args, callback) # returns 1 if error

    def atan2(self, input_y, input_x, output, callback=None):
        """Returns the 2-argument inverse tangent (atan2).

        Keyword arguments:

        input_y -- Input y raster file or constant value (rise). 
        input_x -- Input x raster file or constant value (run). 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input_y='{}'".format(input_y))
        args.append("--input_x='{}'".format(input_x))
        args.append("--output='{}'".format(output))
        return self.run_tool('atan2', args, callback) # returns 1 if error

    def attribute_correlation(self, i, output=None, callback=None):
        """Performs a correlation analysis on attribute fields from a vector database.

        Keyword arguments:

        i -- Input vector file. 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        return self.run_tool('attribute_correlation', args, callback) # returns 1 if error

    def attribute_correlation_neighbourhood_analysis(self, i, field1, field2, radius=None, min_points=None, stat="pearson", callback=None):
        """Performs a correlation on two input vector attributes within a neighbourhood search windows.

        Keyword arguments:

        i -- Input vector file. 
        field1 -- First input field name (dependent variable) in attribute table. 
        field2 -- Second input field name (independent variable) in attribute table. 
        radius -- Search Radius (in map units). 
        min_points -- Minimum number of points. 
        stat -- Correlation type; one of 'pearson' (default) and 'spearman'. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--field1='{}'".format(field1))
        args.append("--field2='{}'".format(field2))
        if radius is not None: args.append("--radius='{}'".format(radius))
        if min_points is not None: args.append("--min_points='{}'".format(min_points))
        args.append("--stat={}".format(stat))
        return self.run_tool('attribute_correlation_neighbourhood_analysis', args, callback) # returns 1 if error

    def attribute_histogram(self, i, field, output, callback=None):
        """Creates a histogram for the field values of a vector's attribute table.

        Keyword arguments:

        i -- Input raster file. 
        field -- Input field name in attribute table. 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--field='{}'".format(field))
        args.append("--output='{}'".format(output))
        return self.run_tool('attribute_histogram', args, callback) # returns 1 if error

    def attribute_scattergram(self, i, fieldx, fieldy, output, trendline=False, callback=None):
        """Creates a scattergram for two field values of a vector's attribute table.

        Keyword arguments:

        i -- Input raster file. 
        fieldx -- Input field name in attribute table for the x-axis. 
        fieldy -- Input field name in attribute table for the y-axis. 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        trendline -- Draw the trendline. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--fieldx='{}'".format(fieldx))
        args.append("--fieldy='{}'".format(fieldy))
        args.append("--output='{}'".format(output))
        if trendline: args.append("--trendline")
        return self.run_tool('attribute_scattergram', args, callback) # returns 1 if error

    def ceil(self, i, output, callback=None):
        """Returns the smallest (closest to negative infinity) value that is greater than or equal to the values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('ceil', args, callback) # returns 1 if error

    def cos(self, i, output, callback=None):
        """Returns the cosine (cos) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('cos', args, callback) # returns 1 if error

    def cosh(self, i, output, callback=None):
        """Returns the hyperbolic cosine (cosh) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('cosh', args, callback) # returns 1 if error

    def crispness_index(self, i, output=None, callback=None):
        """Calculates the Crispness Index, which is used to quantify how crisp (or conversely how fuzzy) a probability image is.

        Keyword arguments:

        i -- Input raster file. 
        output -- Optional output html file (default name will be based on input file if unspecified). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        if output is not None: args.append("--output='{}'".format(output))
        return self.run_tool('crispness_index', args, callback) # returns 1 if error

    def cross_tabulation(self, input1, input2, output, callback=None):
        """Performs a cross-tabulation on two categorical images.

        Keyword arguments:

        input1 -- Input raster file 1. 
        input2 -- Input raster file 1. 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('cross_tabulation', args, callback) # returns 1 if error

    def cumulative_distribution(self, i, output, callback=None):
        """Converts a raster image to its cumulative distribution function.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('cumulative_distribution', args, callback) # returns 1 if error

    def decrement(self, i, output, callback=None):
        """Decreases the values of each grid cell in an input raster by 1.0 (see also InPlaceSubtract).

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('decrement', args, callback) # returns 1 if error

    def divide(self, input1, input2, output, callback=None):
        """Performs a division operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('divide', args, callback) # returns 1 if error

    def equal_to(self, input1, input2, output, callback=None):
        """Performs a equal-to comparison operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('equal_to', args, callback) # returns 1 if error

    def exp(self, i, output, callback=None):
        """Returns the exponential (base e) of values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('exp', args, callback) # returns 1 if error

    def exp2(self, i, output, callback=None):
        """Returns the exponential (base 2) of values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('exp2', args, callback) # returns 1 if error

    def floor(self, i, output, callback=None):
        """Returns the largest (closest to positive infinity) value that is less than or equal to the values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('floor', args, callback) # returns 1 if error

    def greater_than(self, input1, input2, output, incl_equals=False, callback=None):
        """Performs a greater-than comparison operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        incl_equals -- Perform a greater-than-or-equal-to operation. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        if incl_equals: args.append("--incl_equals")
        return self.run_tool('greater_than', args, callback) # returns 1 if error

    def image_autocorrelation(self, inputs, output, contiguity="Rook", callback=None):
        """Performs Moran's I analysis on two or more input images.

        Keyword arguments:

        inputs -- Input raster files. 
        contiguity -- Contiguity type. 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--contiguity={}".format(contiguity))
        args.append("--output='{}'".format(output))
        return self.run_tool('image_autocorrelation', args, callback) # returns 1 if error

    def image_correlation(self, inputs, output=None, callback=None):
        """Performs image correlation on two or more input images.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        if output is not None: args.append("--output='{}'".format(output))
        return self.run_tool('image_correlation', args, callback) # returns 1 if error

    def image_correlation_neighbourhood_analysis(self, input1, input2, output1, output2, filter=11, stat="pearson", callback=None):
        """Performs image correlation on two input images neighbourhood search windows.

        Keyword arguments:

        input1 -- Input raster file. 
        input2 -- Input raster file. 
        output1 -- Output correlation (r-value or rho) raster file. 
        output2 -- Output significance (p-value) raster file. 
        filter -- Size of the filter kernel. 
        stat -- Correlation type; one of 'pearson' (default) and 'spearman'. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output1='{}'".format(output1))
        args.append("--output2='{}'".format(output2))
        args.append("--filter={}".format(filter))
        args.append("--stat={}".format(stat))
        return self.run_tool('image_correlation_neighbourhood_analysis', args, callback) # returns 1 if error

    def image_regression(self, input1, input2, output, out_residuals=None, standardize=False, scattergram=False, num_samples=1000, callback=None):
        """Performs image regression analysis on two input images.

        Keyword arguments:

        input1 -- Input raster file (independent variable, X). 
        input2 -- Input raster file (dependent variable, Y). 
        output -- Output HTML file for regression summary report. 
        out_residuals -- Output raster regression resdidual file. 
        standardize -- Optional flag indicating whether to standardize the residuals map. 
        scattergram -- Optional flag indicating whether to output a scattergram. 
        num_samples -- Number of samples used to create scattergram. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        if out_residuals is not None: args.append("--out_residuals='{}'".format(out_residuals))
        if standardize: args.append("--standardize")
        if scattergram: args.append("--scattergram")
        args.append("--num_samples={}".format(num_samples))
        return self.run_tool('image_regression', args, callback) # returns 1 if error

    def in_place_add(self, input1, input2, callback=None):
        """Performs an in-place addition operation (input1 += input2).

        Keyword arguments:

        input1 -- Input raster file. 
        input2 -- Input raster file or constant value. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        return self.run_tool('in_place_add', args, callback) # returns 1 if error

    def in_place_divide(self, input1, input2, callback=None):
        """Performs an in-place division operation (input1 /= input2).

        Keyword arguments:

        input1 -- Input raster file. 
        input2 -- Input raster file or constant value. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        return self.run_tool('in_place_divide', args, callback) # returns 1 if error

    def in_place_multiply(self, input1, input2, callback=None):
        """Performs an in-place multiplication operation (input1 *= input2).

        Keyword arguments:

        input1 -- Input raster file. 
        input2 -- Input raster file or constant value. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        return self.run_tool('in_place_multiply', args, callback) # returns 1 if error

    def in_place_subtract(self, input1, input2, callback=None):
        """Performs an in-place subtraction operation (input1 -= input2).

        Keyword arguments:

        input1 -- Input raster file. 
        input2 -- Input raster file or constant value. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        return self.run_tool('in_place_subtract', args, callback) # returns 1 if error

    def increment(self, i, output, callback=None):
        """Increases the values of each grid cell in an input raster by 1.0. (see also InPlaceAdd).

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('increment', args, callback) # returns 1 if error

    def integer_division(self, input1, input2, output, callback=None):
        """Performs an integer division operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('integer_division', args, callback) # returns 1 if error

    def is_no_data(self, i, output, callback=None):
        """Identifies NoData valued pixels in an image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('is_no_data', args, callback) # returns 1 if error

    def kappa_index(self, input1, input2, output, callback=None):
        """Performs a kappa index of agreement (KIA) analysis on two categorical raster files.

        Keyword arguments:

        input1 -- Input classification raster file. 
        input2 -- Input reference raster file. 
        output -- Output HTML file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('kappa_index', args, callback) # returns 1 if error

    def ks_test_for_normality(self, i, output, num_samples=None, callback=None):
        """Evaluates whether the values in a raster are normally distributed.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output HTML file. 
        num_samples -- Number of samples. Leave blank to use whole image. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if num_samples is not None: args.append("--num_samples='{}'".format(num_samples))
        return self.run_tool('ks_test_for_normality', args, callback) # returns 1 if error

    def less_than(self, input1, input2, output, incl_equals=False, callback=None):
        """Performs a less-than comparison operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        incl_equals -- Perform a less-than-or-equal-to operation. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        if incl_equals: args.append("--incl_equals")
        return self.run_tool('less_than', args, callback) # returns 1 if error

    def list_unique_values(self, i, field, output, callback=None):
        """Lists the unique values contained in a field witin a vector's attribute table.

        Keyword arguments:

        i -- Input raster file. 
        field -- Input field name in attribute table. 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--field='{}'".format(field))
        args.append("--output='{}'".format(output))
        return self.run_tool('list_unique_values', args, callback) # returns 1 if error

    def ln(self, i, output, callback=None):
        """Returns the natural logarithm of values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('ln', args, callback) # returns 1 if error

    def log10(self, i, output, callback=None):
        """Returns the base-10 logarithm of values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('log10', args, callback) # returns 1 if error

    def log2(self, i, output, callback=None):
        """Returns the base-2 logarithm of values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('log2', args, callback) # returns 1 if error

    def max(self, input1, input2, output, callback=None):
        """Performs a MAX operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('max', args, callback) # returns 1 if error

    def min(self, input1, input2, output, callback=None):
        """Performs a MIN operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('min', args, callback) # returns 1 if error

    def modulo(self, input1, input2, output, callback=None):
        """Performs a modulo operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('modulo', args, callback) # returns 1 if error

    def multiply(self, input1, input2, output, callback=None):
        """Performs a multiplication operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('multiply', args, callback) # returns 1 if error

    def negate(self, i, output, callback=None):
        """Changes the sign of values in a raster or the 0-1 values of a Boolean raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('negate', args, callback) # returns 1 if error

    def not_equal_to(self, input1, input2, output, callback=None):
        """Performs a not-equal-to comparison operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('not_equal_to', args, callback) # returns 1 if error

    def paired_sample_t_test(self, input1, input2, output, num_samples=None, callback=None):
        """Performs a 2-sample K-S test for significant differences on two input rasters.

        Keyword arguments:

        input1 -- First input raster file. 
        input2 -- Second input raster file. 
        output -- Output HTML file. 
        num_samples -- Number of samples. Leave blank to use whole image. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        if num_samples is not None: args.append("--num_samples='{}'".format(num_samples))
        return self.run_tool('paired_sample_t_test', args, callback) # returns 1 if error

    def power(self, input1, input2, output, callback=None):
        """Raises the values in grid cells of one rasters, or a constant value, by values in another raster or constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('power', args, callback) # returns 1 if error

    def principal_component_analysis(self, inputs, output, num_comp=None, standardized=False, callback=None):
        """Performs a principal component analysis (PCA) on a multi-spectral dataset.

        Keyword arguments:

        inputs -- Input raster files. 
        output -- Output HTML report file. 
        num_comp -- Number of component images to output; <= to num. input images. 
        standardized -- Perform standardized PCA?. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        if num_comp is not None: args.append("--num_comp='{}'".format(num_comp))
        if standardized: args.append("--standardized")
        return self.run_tool('principal_component_analysis', args, callback) # returns 1 if error

    def quantiles(self, i, output, num_quantiles=5, callback=None):
        """Transforms raster values into quantiles.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        num_quantiles -- Number of quantiles. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--num_quantiles={}".format(num_quantiles))
        return self.run_tool('quantiles', args, callback) # returns 1 if error

    def random_field(self, base, output, callback=None):
        """Creates an image containing random values.

        Keyword arguments:

        base -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--base='{}'".format(base))
        args.append("--output='{}'".format(output))
        return self.run_tool('random_field', args, callback) # returns 1 if error

    def random_sample(self, base, output, num_samples=1000, callback=None):
        """Creates an image containing randomly located sample grid cells with unique IDs.

        Keyword arguments:

        base -- Input raster file. 
        output -- Output raster file. 
        num_samples -- Number of samples. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--base='{}'".format(base))
        args.append("--output='{}'".format(output))
        args.append("--num_samples={}".format(num_samples))
        return self.run_tool('random_sample', args, callback) # returns 1 if error

    def raster_histogram(self, i, output, callback=None):
        """Creates a histogram from raster values.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output HTML file (default name will be based on input file if unspecified). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('raster_histogram', args, callback) # returns 1 if error

    def raster_summary_stats(self, i, callback=None):
        """Measures a rasters min, max, average, standard deviation, num. non-nodata cells, and total.

        Keyword arguments:

        i -- Input raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        return self.run_tool('raster_summary_stats', args, callback) # returns 1 if error

    def reciprocal(self, i, output, callback=None):
        """Returns the reciprocal (i.e. 1 / z) of values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('reciprocal', args, callback) # returns 1 if error

    def rescale_value_range(self, i, output, out_min_val, out_max_val, clip_min=None, clip_max=None, callback=None):
        """Performs a min-max contrast stretch on an input greytone image.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        out_min_val -- New minimum value in output image. 
        out_max_val -- New maximum value in output image. 
        clip_min -- Optional lower tail clip value. 
        clip_max -- Optional upper tail clip value. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--out_min_val='{}'".format(out_min_val))
        args.append("--out_max_val='{}'".format(out_max_val))
        if clip_min is not None: args.append("--clip_min='{}'".format(clip_min))
        if clip_max is not None: args.append("--clip_max='{}'".format(clip_max))
        return self.run_tool('rescale_value_range', args, callback) # returns 1 if error

    def root_mean_square_error(self, i, base, callback=None):
        """Calculates the RMSE and other accuracy statistics.

        Keyword arguments:

        i -- Input raster file. 
        base -- Input base raster file used for comparison. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--base='{}'".format(base))
        return self.run_tool('root_mean_square_error', args, callback) # returns 1 if error

    def round(self, i, output, callback=None):
        """Rounds the values in an input raster to the nearest integer value.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('round', args, callback) # returns 1 if error

    def sin(self, i, output, callback=None):
        """Returns the sine (sin) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('sin', args, callback) # returns 1 if error

    def sinh(self, i, output, callback=None):
        """Returns the hyperbolic sine (sinh) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('sinh', args, callback) # returns 1 if error

    def square(self, i, output, callback=None):
        """Squares the values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('square', args, callback) # returns 1 if error

    def square_root(self, i, output, callback=None):
        """Returns the square root of the values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('square_root', args, callback) # returns 1 if error

    def subtract(self, input1, input2, output, callback=None):
        """Performs a differencing operation on two rasters or a raster and a constant value.

        Keyword arguments:

        input1 -- Input raster file or constant value. 
        input2 -- Input raster file or constant value. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('subtract', args, callback) # returns 1 if error

    def tan(self, i, output, callback=None):
        """Returns the tangent (tan) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('tan', args, callback) # returns 1 if error

    def tanh(self, i, output, callback=None):
        """Returns the hyperbolic tangent (tanh) of each values in a raster.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('tanh', args, callback) # returns 1 if error

    def to_degrees(self, i, output, callback=None):
        """Converts a raster from radians to degrees.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('to_degrees', args, callback) # returns 1 if error

    def to_radians(self, i, output, callback=None):
        """Converts a raster from degrees to radians.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('to_radians', args, callback) # returns 1 if error

    def trend_surface(self, i, output, order=1, callback=None):
        """Estimates the trend surface of an input raster file.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        order -- Polynomial order (1 to 10). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        args.append("--order={}".format(order))
        return self.run_tool('trend_surface', args, callback) # returns 1 if error

    def trend_surface_vector_points(self, i, field, output, cell_size, order=1, callback=None):
        """Estimates a trend surface from vector points.

        Keyword arguments:

        i -- Input vector Points file. 
        field -- Input field name in attribute table. 
        output -- Output raster file. 
        order -- Polynomial order (1 to 10). 
        cell_size -- Optionally specified cell size of output raster. Not used when base raster is specified. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--field='{}'".format(field))
        args.append("--output='{}'".format(output))
        args.append("--order={}".format(order))
        args.append("--cell_size='{}'".format(cell_size))
        return self.run_tool('trend_surface_vector_points', args, callback) # returns 1 if error

    def truncate(self, i, output, num_decimals=None, callback=None):
        """Truncates the values in a raster to the desired number of decimal places.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        num_decimals -- Number of decimals left after truncation (default is zero). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        if num_decimals is not None: args.append("--num_decimals='{}'".format(num_decimals))
        return self.run_tool('truncate', args, callback) # returns 1 if error

    def turning_bands_simulation(self, base, output, range, iterations=1000, callback=None):
        """Creates an image containing random values based on a turning-bands simulation.

        Keyword arguments:

        base -- Input base raster file. 
        output -- Output file. 
        range -- The field's range, in xy-units, related to the extent of spatial autocorrelation. 
        iterations -- The number of iterations. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--base='{}'".format(base))
        args.append("--output='{}'".format(output))
        args.append("--range='{}'".format(range))
        args.append("--iterations={}".format(iterations))
        return self.run_tool('turning_bands_simulation', args, callback) # returns 1 if error

    def two_sample_ks_test(self, input1, input2, output, num_samples=None, callback=None):
        """Performs a 2-sample K-S test for significant differences on two input rasters.

        Keyword arguments:

        input1 -- First input raster file. 
        input2 -- Second input raster file. 
        output -- Output HTML file. 
        num_samples -- Number of samples. Leave blank to use whole image. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        if num_samples is not None: args.append("--num_samples='{}'".format(num_samples))
        return self.run_tool('two_sample_ks_test', args, callback) # returns 1 if error

    def wilcoxon_signed_rank_test(self, input1, input2, output, num_samples=None, callback=None):
        """Performs a 2-sample K-S test for significant differences on two input rasters.

        Keyword arguments:

        input1 -- First input raster file. 
        input2 -- Second input raster file. 
        output -- Output HTML file. 
        num_samples -- Number of samples. Leave blank to use whole image. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        if num_samples is not None: args.append("--num_samples='{}'".format(num_samples))
        return self.run_tool('wilcoxon_signed_rank_test', args, callback) # returns 1 if error

    def xor(self, input1, input2, output, callback=None):
        """Performs a logical XOR operator on two Boolean raster images.

        Keyword arguments:

        input1 -- Input raster file. 
        input2 -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('xor', args, callback) # returns 1 if error

    def z_scores(self, i, output, callback=None):
        """Standardizes the values in an input raster by converting to z-scores.

        Keyword arguments:

        i -- Input raster file. 
        output -- Output raster file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--output='{}'".format(output))
        return self.run_tool('z_scores', args, callback) # returns 1 if error

    def zonal_statistics(self, i, features, output=None, stat="mean", out_table=None, callback=None):
        """Extracts descriptive statistics for a group of patches in a raster.

        Keyword arguments:

        i -- Input data raster file. 
        features -- Input feature definition raster file. 
        output -- Output raster file. 
        stat -- Statistic to extract, including 'mean', 'median', 'minimum', 'maximum', 'range', 'standard deviation', and 'total'. 
        out_table -- Output HTML Table file. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--input='{}'".format(i))
        args.append("--features='{}'".format(features))
        if output is not None: args.append("--output='{}'".format(output))
        args.append("--stat={}".format(stat))
        if out_table is not None: args.append("--out_table='{}'".format(out_table))
        return self.run_tool('zonal_statistics', args, callback) # returns 1 if error

    ###########################
    # Stream Network Analysis #
    ###########################

    def distance_to_outlet(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=None):
        """Calculates the distance of stream grid cells to the channel network outlet cell.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('distance_to_outlet', args, callback) # returns 1 if error

    def extract_streams(self, flow_accum, output, threshold, zero_background=False, callback=None):
        """Extracts stream grid cells from a flow accumulation raster.

        Keyword arguments:

        flow_accum -- Input raster D8 flow accumulation file. 
        output -- Output raster file. 
        threshold -- Threshold in flow accumulation values for channelization. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--flow_accum='{}'".format(flow_accum))
        args.append("--output='{}'".format(output))
        args.append("--threshold='{}'".format(threshold))
        if zero_background: args.append("--zero_background")
        return self.run_tool('extract_streams', args, callback) # returns 1 if error

    def extract_valleys(self, dem, output, variant="LQ", line_thin=True, filter=5, callback=None):
        """Identifies potential valley bottom grid cells based on local topolography alone.

        Keyword arguments:

        dem -- Input raster DEM file. 
        output -- Output raster file. 
        variant -- Options include 'LQ' (lower quartile), 'JandR' (Johnston and Rosenfeld), and 'PandD' (Peucker and Douglas); default is 'LQ'. 
        line_thin -- Optional flag indicating whether post-processing line-thinning should be performed. 
        filter -- Optional argument (only used when variant='lq') providing the filter size, in grid cells, used for lq-filtering (default is 5). 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--variant={}".format(variant))
        if line_thin: args.append("--line_thin")
        args.append("--filter={}".format(filter))
        return self.run_tool('extract_valleys', args, callback) # returns 1 if error

    def farthest_channel_head(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=None):
        """Calculates the distance to the furthest upstream channel head for each stream cell.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('farthest_channel_head', args, callback) # returns 1 if error

    def find_main_stem(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=None):
        """Finds the main stem, based on stream lengths, of each stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('find_main_stem', args, callback) # returns 1 if error

    def hack_stream_order(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=None):
        """Assigns the Hack stream order to each tributary in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('hack_stream_order', args, callback) # returns 1 if error

    def horton_stream_order(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=None):
        """Assigns the Horton stream order to each tributary in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('horton_stream_order', args, callback) # returns 1 if error

    def length_of_upstream_channels(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=None):
        """Calculates the total length of channels upstream.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('length_of_upstream_channels', args, callback) # returns 1 if error

    def long_profile(self, d8_pntr, streams, dem, output, esri_pntr=False, callback=None):
        """Plots the stream longitudinal profiles for one or more rivers.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        dem -- Input raster DEM file. 
        output -- Output HTML file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('long_profile', args, callback) # returns 1 if error

    def long_profile_from_points(self, d8_pntr, points, dem, output, esri_pntr=False, callback=None):
        """Plots the longitudinal profiles from flow-paths initiating from a set of vector points.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        points -- Input vector points file. 
        dem -- Input raster DEM file. 
        output -- Output HTML file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--points='{}'".format(points))
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('long_profile_from_points', args, callback) # returns 1 if error

    def raster_streams_to_vector(self, streams, d8_pntr, output, esri_pntr=False, callback=None):
        """Converts a raster stream file into a vector file.

        Keyword arguments:

        streams -- Input raster streams file. 
        d8_pntr -- Input raster D8 pointer file. 
        output -- Output vector file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--streams='{}'".format(streams))
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('raster_streams_to_vector', args, callback) # returns 1 if error

    def rasterize_streams(self, streams, base, output, nodata=True, feature_id=False, callback=None):
        """Rasterizes vector streams based on Lindsay (2016) method.

        Keyword arguments:

        streams -- Input vector streams file. 
        base -- Input base raster file. 
        output -- Output raster file. 
        nodata -- Use NoData value for background?. 
        feature_id -- Use feature number as output value?. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--streams='{}'".format(streams))
        args.append("--base='{}'".format(base))
        args.append("--output='{}'".format(output))
        if nodata: args.append("--nodata")
        if feature_id: args.append("--feature_id")
        return self.run_tool('rasterize_streams', args, callback) # returns 1 if error

    def remove_short_streams(self, d8_pntr, streams, output, min_length, esri_pntr=False, callback=None):
        """Removes short first-order streams from a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        min_length -- Minimum tributary length (in map units) used for network prunning. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        args.append("--min_length='{}'".format(min_length))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('remove_short_streams', args, callback) # returns 1 if error

    def shreve_stream_magnitude(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=None):
        """Assigns the Shreve stream magnitude to each link in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('shreve_stream_magnitude', args, callback) # returns 1 if error

    def strahler_stream_order(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=None):
        """Assigns the Strahler stream order to each link in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('strahler_stream_order', args, callback) # returns 1 if error

    def stream_link_class(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=None):
        """Identifies the exterior/interior links and nodes in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('stream_link_class', args, callback) # returns 1 if error

    def stream_link_identifier(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=None):
        """Assigns a unique identifier to each link in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('stream_link_identifier', args, callback) # returns 1 if error

    def stream_link_length(self, d8_pntr, linkid, output, esri_pntr=False, zero_background=False, callback=None):
        """Estimates the length of each link (or tributary) in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        linkid -- Input raster streams link ID (or tributary ID) file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--linkid='{}'".format(linkid))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('stream_link_length', args, callback) # returns 1 if error

    def stream_link_slope(self, d8_pntr, linkid, dem, output, esri_pntr=False, zero_background=False, callback=None):
        """Estimates the average slope of each link (or tributary) in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        linkid -- Input raster streams link ID (or tributary ID) file. 
        dem -- Input raster DEM file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--linkid='{}'".format(linkid))
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('stream_link_slope', args, callback) # returns 1 if error

    def stream_slope_continuous(self, d8_pntr, streams, dem, output, esri_pntr=False, zero_background=False, callback=None):
        """Estimates the slope of each grid cell in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        dem -- Input raster DEM file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('stream_slope_continuous', args, callback) # returns 1 if error

    def topological_stream_order(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=None):
        """Assigns each link in a stream network its topological order.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('topological_stream_order', args, callback) # returns 1 if error

    def tributary_identifier(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=None):
        """Assigns a unique identifier to each tributary in a stream network.

        Keyword arguments:

        d8_pntr -- Input raster D8 pointer file. 
        streams -- Input raster streams file. 
        output -- Output raster file. 
        esri_pntr -- D8 pointer uses the ESRI style scheme. 
        zero_background -- Flag indicating whether a background value of zero should be used. 
        callback -- Custom function for handling tool text outputs.
        """
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('tributary_identifier', args, callback) # returns 1 if error
