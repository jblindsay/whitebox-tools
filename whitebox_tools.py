#!/usr/bin/env python
''' This file is intended to be a helper for running whitebox-tools plugins from a Python script.
See whitebox_example.py for an example of how to use it.
'''

# This script is part of the WhiteboxTools geospatial library.
# Authors: Dr. John Lindsay
# Created: November 28, 2017
# Last Modified: Feb. 17, 2018
# License: MIT

from __future__ import print_function
import os
from os import path
import sys
from sys import platform
from subprocess import CalledProcessError, Popen, PIPE, STDOUT


def default_callback(value):
    ''' A simple default callback that outputs using the print function.
    '''
    print(value)


class WhiteboxTools(object):
    ''' An object for interfacing with the whitebox - tools executable.
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
        ''' Sets the directory to the WhiteboxTools executable file.
        '''
        self.exe_path = path_str

    def set_working_dir(self, path_str):
        ''' Sets the working directory.
        '''
        self.work_dir = path.normpath(path_str)

    def set_verbose_mode(self, val=True):
        ''' Sets verbose mode(i.e. whether a running tool outputs).
        '''
        self.verbose = val

    def run_tool(self, tool_name, args, callback=default_callback):
        ''' Runs a tool and specifies tool arguments.
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
        ''' Retrieve the help description for whitebox - tools.
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
        ''' Retrieves the license information for whitebox - tools.
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
        ''' Retrieves the version information for whitebox-tools.
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
        ''' Retrieve the help description for a specific tool.
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
        ''' Retrieve the tool parameter descriptions for a specific tool.
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
        ''' Retrieve the toolbox for a specific tool.
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
        ''' Opens a web browser to view the source code for a specific tool
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
        ''' Lists all available tools in whitebox - tools.
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
    # whitebox_plugin_generator.py script.
    ########################################################################

    def absolute_value(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('AbsoluteValue', args, callback) # returns 1 if error

    def adaptive_filter(self, input, output, filterx=11, filtery=11, threshold=2.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        args.append("--threshold='{}'".format(threshold))
        return self.run_tool('AdaptiveFilter', args, callback) # returns 1 if error

    def add(self, input1, input2, output, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Add', args, callback) # returns 1 if error

    def aggregate_raster(self, input, output, agg_factor=2, type="mean", callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--agg_factor='{}'".format(agg_factor))
        args.append("--type='{}'".format(type))
        return self.run_tool('AggregateRaster', args, callback) # returns 1 if error

    def And(self, input1, input2, output, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('And', args, callback) # returns 1 if error

    def anova(self, input, features, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--features='{}'".format(features))
        args.append("--output='{}'".format(output))
        return self.run_tool('Anova', args, callback) # returns 1 if error

    def arc_cos(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('ArcCos', args, callback) # returns 1 if error

    def arc_sin(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('ArcSin', args, callback) # returns 1 if error

    def arc_tan(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('ArcTan', args, callback) # returns 1 if error

    def aspect(self, dem, output, zfactor=1.0, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor='{}'".format(zfactor))
        return self.run_tool('Aspect', args, callback) # returns 1 if error

    def atan2(self, input_y, input_x, output, callback=default_callback):
        args = []
        args.append("--input_y='{}'".format(input_y))
        args.append("--input_x='{}'".format(input_x))
        args.append("--output='{}'".format(output))
        return self.run_tool('Atan2', args, callback) # returns 1 if error

    def average_flowpath_slope(self, dem, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('AverageFlowpathSlope', args, callback) # returns 1 if error

    def average_overlay(self, inputs, output, callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('AverageOverlay', args, callback) # returns 1 if error

    def average_upslope_flowpath_length(self, dem, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('AverageUpslopeFlowpathLength', args, callback) # returns 1 if error

    def balance_contrast_enhancement(self, input, output, band_mean=100.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--band_mean='{}'".format(band_mean))
        return self.run_tool('BalanceContrastEnhancement', args, callback) # returns 1 if error

    def basins(self, d8_pntr, output, esri_pntr=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('Basins', args, callback) # returns 1 if error

    def bilateral_filter(self, input, output, sigma_dist=0.75, sigma_int=1.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--sigma_dist='{}'".format(sigma_dist))
        args.append("--sigma_int='{}'".format(sigma_int))
        return self.run_tool('BilateralFilter', args, callback) # returns 1 if error

    def block_maximum(self, input, output, resolution=1.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--resolution='{}'".format(resolution))
        return self.run_tool('BlockMaximum', args, callback) # returns 1 if error

    def block_minimum(self, input, output, resolution=1.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--resolution='{}'".format(resolution))
        return self.run_tool('BlockMinimum', args, callback) # returns 1 if error

    def breach_depressions(self, dem, output, max_depth, max_length, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--max_depth='{}'".format(max_depth))
        args.append("--max_length='{}'".format(max_length))
        return self.run_tool('BreachDepressions', args, callback) # returns 1 if error

    def breach_single_cell_pits(self, dem, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('BreachSingleCellPits', args, callback) # returns 1 if error

    def buffer_raster(self, input, output, size, gridcells=False, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--size='{}'".format(size))
        if gridcells: args.append("--gridcells")
        return self.run_tool('BufferRaster', args, callback) # returns 1 if error

    def ceil(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('Ceil', args, callback) # returns 1 if error

    def centroid(self, input, output, text_output=False, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        if text_output: args.append("--text_output")
        return self.run_tool('Centroid', args, callback) # returns 1 if error

    def closing(self, input, output, filterx=11, filtery=11, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        return self.run_tool('Closing', args, callback) # returns 1 if error

    def clump(self, input, output, diag=True, zero_back=False, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        if diag: args.append("--diag")
        if zero_back: args.append("--zero_back")
        return self.run_tool('Clump', args, callback) # returns 1 if error

    def conservative_smoothing_filter(self, input, output, filterx=11, filtery=11, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        return self.run_tool('ConservativeSmoothingFilter', args, callback) # returns 1 if error

    def convert_nodata_to_zero(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('ConvertNodataToZero', args, callback) # returns 1 if error

    def convert_raster_format(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('ConvertRasterFormat', args, callback) # returns 1 if error

    def cos(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('Cos', args, callback) # returns 1 if error

    def cosh(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('Cosh', args, callback) # returns 1 if error

    def cost_allocation(self, source, backlink, output, callback=default_callback):
        args = []
        args.append("--source='{}'".format(source))
        args.append("--backlink='{}'".format(backlink))
        args.append("--output='{}'".format(output))
        return self.run_tool('CostAllocation', args, callback) # returns 1 if error

    def cost_distance(self, source, cost, out_accum, out_backlink, callback=default_callback):
        args = []
        args.append("--source='{}'".format(source))
        args.append("--cost='{}'".format(cost))
        args.append("--out_accum='{}'".format(out_accum))
        args.append("--out_backlink='{}'".format(out_backlink))
        return self.run_tool('CostDistance', args, callback) # returns 1 if error

    def cost_pathway(self, destination, backlink, output, zero_background=False, callback=default_callback):
        args = []
        args.append("--destination='{}'".format(destination))
        args.append("--backlink='{}'".format(backlink))
        args.append("--output='{}'".format(output))
        if zero_background: args.append("--zero_background")
        return self.run_tool('CostPathway', args, callback) # returns 1 if error

    def create_colour_composite(self, red, green, blue, opacity, output, enhance=True, callback=default_callback):
        args = []
        args.append("--red='{}'".format(red))
        args.append("--green='{}'".format(green))
        args.append("--blue='{}'".format(blue))
        args.append("--opacity='{}'".format(opacity))
        args.append("--output='{}'".format(output))
        if enhance: args.append("--enhance")
        return self.run_tool('CreateColourComposite', args, callback) # returns 1 if error

    def create_plane(self, base, output, gradient=15.0, aspect=90.0, constant=0.0, callback=default_callback):
        args = []
        args.append("--base='{}'".format(base))
        args.append("--output='{}'".format(output))
        args.append("--gradient='{}'".format(gradient))
        args.append("--aspect='{}'".format(aspect))
        args.append("--constant='{}'".format(constant))
        return self.run_tool('CreatePlane', args, callback) # returns 1 if error

    def crispness_index(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('CrispnessIndex', args, callback) # returns 1 if error

    def cross_tabulation(self, input1, input2, output, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('CrossTabulation', args, callback) # returns 1 if error

    def cumulative_distribution(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('CumulativeDistribution', args, callback) # returns 1 if error

    def d8_flow_accumulation(self, dem, output, out_type="specific contributing area", log=False, clip=False, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--out_type='{}'".format(out_type))
        if log: args.append("--log")
        if clip: args.append("--clip")
        return self.run_tool('D8FlowAccumulation', args, callback) # returns 1 if error

    def d8_mass_flux(self, dem, loading, efficiency, absorption, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--loading='{}'".format(loading))
        args.append("--efficiency='{}'".format(efficiency))
        args.append("--absorption='{}'".format(absorption))
        args.append("--output='{}'".format(output))
        return self.run_tool('D8MassFlux', args, callback) # returns 1 if error

    def d8_pointer(self, dem, output, esri_pntr=False, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('D8Pointer', args, callback) # returns 1 if error

    def d_inf_flow_accumulation(self, dem, output, threshold, out_type="Specific Contributing Area", log=False, clip=False, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--out_type='{}'".format(out_type))
        args.append("--threshold='{}'".format(threshold))
        if log: args.append("--log")
        if clip: args.append("--clip")
        return self.run_tool('DInfFlowAccumulation', args, callback) # returns 1 if error

    def d_inf_mass_flux(self, dem, loading, efficiency, absorption, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--loading='{}'".format(loading))
        args.append("--efficiency='{}'".format(efficiency))
        args.append("--absorption='{}'".format(absorption))
        args.append("--output='{}'".format(output))
        return self.run_tool('DInfMassFlux', args, callback) # returns 1 if error

    def d_inf_pointer(self, dem, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('DInfPointer', args, callback) # returns 1 if error

    def decrement(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('Decrement', args, callback) # returns 1 if error

    def depth_in_sink(self, dem, output, zero_background=False, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if zero_background: args.append("--zero_background")
        return self.run_tool('DepthInSink', args, callback) # returns 1 if error

    def dev_from_mean_elev(self, dem, output, filterx=11, filtery=11, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        return self.run_tool('DevFromMeanElev', args, callback) # returns 1 if error

    def diff_from_mean_elev(self, dem, output, filterx=11, filtery=11, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        return self.run_tool('DiffFromMeanElev', args, callback) # returns 1 if error

    def diff_of_gaussian_filter(self, input, output, sigma1=2.0, sigma2=4.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--sigma1='{}'".format(sigma1))
        args.append("--sigma2='{}'".format(sigma2))
        return self.run_tool('DiffOfGaussianFilter', args, callback) # returns 1 if error

    def direct_decorrelation_stretch(self, input, output, k=0.5, clip=1.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("-k='{}'".format(k))
        args.append("--clip='{}'".format(clip))
        return self.run_tool('DirectDecorrelationStretch', args, callback) # returns 1 if error

    def directional_relief(self, dem, output, max_dist, azimuth=0.0, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--azimuth='{}'".format(azimuth))
        args.append("--max_dist='{}'".format(max_dist))
        return self.run_tool('DirectionalRelief', args, callback) # returns 1 if error

    def distance_to_outlet(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('DistanceToOutlet', args, callback) # returns 1 if error

    def diversity_filter(self, input, output, filterx=11, filtery=11, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        return self.run_tool('DiversityFilter', args, callback) # returns 1 if error

    def divide(self, input1, input2, output, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Divide', args, callback) # returns 1 if error

    def downslope_distance_to_stream(self, dem, streams, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        return self.run_tool('DownslopeDistanceToStream', args, callback) # returns 1 if error

    def downslope_flowpath_length(self, d8_pntr, watersheds, weights, output, esri_pntr=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--watersheds='{}'".format(watersheds))
        args.append("--weights='{}'".format(weights))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('DownslopeFlowpathLength', args, callback) # returns 1 if error

    def downslope_index(self, dem, output, drop=2.0, out_type="tangent", callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--drop='{}'".format(drop))
        args.append("--out_type='{}'".format(out_type))
        return self.run_tool('DownslopeIndex', args, callback) # returns 1 if error

    def edge_proportion(self, input, output, output_text=False, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        if output_text: args.append("--output_text")
        return self.run_tool('EdgeProportion', args, callback) # returns 1 if error

    def elev_above_pit(self, dem, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('ElevAbovePit', args, callback) # returns 1 if error

    def elev_percentile(self, dem, output, filterx=11, filtery=11, sig_digits=2, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        args.append("--sig_digits='{}'".format(sig_digits))
        return self.run_tool('ElevPercentile', args, callback) # returns 1 if error

    def elev_relative_to_min_max(self, dem, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('ElevRelativeToMinMax', args, callback) # returns 1 if error

    def elev_relative_to_watershed_min_max(self, dem, watersheds, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--watersheds='{}'".format(watersheds))
        args.append("--output='{}'".format(output))
        return self.run_tool('ElevRelativeToWatershedMinMax', args, callback) # returns 1 if error

    def elevation_above_stream(self, dem, streams, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        return self.run_tool('ElevationAboveStream', args, callback) # returns 1 if error

    def emboss_filter(self, input, output, direction="n", clip=0.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--direction='{}'".format(direction))
        args.append("--clip='{}'".format(clip))
        return self.run_tool('EmbossFilter', args, callback) # returns 1 if error

    def equal_to(self, input1, input2, output, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('EqualTo', args, callback) # returns 1 if error

    def euclidean_allocation(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('EuclideanAllocation', args, callback) # returns 1 if error

    def euclidean_distance(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('EuclideanDistance', args, callback) # returns 1 if error

    def exp(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('Exp', args, callback) # returns 1 if error

    def exp2(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('Exp2', args, callback) # returns 1 if error

    def extract_raster_statistics(self, input, features, output, out_table, stat="average", callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--features='{}'".format(features))
        args.append("--output='{}'".format(output))
        args.append("--stat='{}'".format(stat))
        args.append("--out_table='{}'".format(out_table))
        return self.run_tool('ExtractRasterStatistics', args, callback) # returns 1 if error

    def extract_streams(self, flow_accum, output, threshold, zero_background=False, callback=default_callback):
        args = []
        args.append("--flow_accum='{}'".format(flow_accum))
        args.append("--output='{}'".format(output))
        args.append("--threshold='{}'".format(threshold))
        if zero_background: args.append("--zero_background")
        return self.run_tool('ExtractStreams', args, callback) # returns 1 if error

    def extract_valleys(self, dem, output, variant="Lower Quartile", line_thin=True, filter=5, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--variant='{}'".format(variant))
        if line_thin: args.append("--line_thin")
        args.append("--filter='{}'".format(filter))
        return self.run_tool('ExtractValleys', args, callback) # returns 1 if error

    def fd8_flow_accumulation(self, dem, output, threshold, out_type="specific contributing area", exponent=1.1, log=False, clip=False, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--out_type='{}'".format(out_type))
        args.append("--exponent='{}'".format(exponent))
        args.append("--threshold='{}'".format(threshold))
        if log: args.append("--log")
        if clip: args.append("--clip")
        return self.run_tool('FD8FlowAccumulation', args, callback) # returns 1 if error

    def fd8_pointer(self, dem, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('FD8Pointer', args, callback) # returns 1 if error

    def farthest_channel_head(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('FarthestChannelHead', args, callback) # returns 1 if error

    def feature_preserving_denoise(self, dem, output, filter=11, norm_diff=15.0, num_iter=5, zfactor=1.0, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filter='{}'".format(filter))
        args.append("--norm_diff='{}'".format(norm_diff))
        args.append("--num_iter='{}'".format(num_iter))
        args.append("--zfactor='{}'".format(zfactor))
        return self.run_tool('FeaturePreservingDenoise', args, callback) # returns 1 if error

    def fetch_analysis(self, dem, output, azimuth=0.0, hgt_inc=0.05, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--azimuth='{}'".format(azimuth))
        args.append("--hgt_inc='{}'".format(hgt_inc))
        return self.run_tool('FetchAnalysis', args, callback) # returns 1 if error

    def fill_depressions(self, dem, output, fix_flats=True, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if fix_flats: args.append("--fix_flats")
        return self.run_tool('FillDepressions', args, callback) # returns 1 if error

    def fill_missing_data(self, input, output, filter=11, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--filter='{}'".format(filter))
        return self.run_tool('FillMissingData', args, callback) # returns 1 if error

    def fill_single_cell_pits(self, dem, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('FillSingleCellPits', args, callback) # returns 1 if error

    def filter_lidar_scan_angles(self, input, output, threshold, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--threshold='{}'".format(threshold))
        return self.run_tool('FilterLidarScanAngles', args, callback) # returns 1 if error

    def find_flightline_edge_points(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('FindFlightlineEdgePoints', args, callback) # returns 1 if error

    def find_main_stem(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('FindMainStem', args, callback) # returns 1 if error

    def find_no_flow_cells(self, dem, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('FindNoFlowCells', args, callback) # returns 1 if error

    def find_parallel_flow(self, d8_pntr, streams, output, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        return self.run_tool('FindParallelFlow', args, callback) # returns 1 if error

    def find_patch_or_class_edge_cells(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('FindPatchOrClassEdgeCells', args, callback) # returns 1 if error

    def find_ridges(self, dem, output, line_thin=True, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if line_thin: args.append("--line_thin")
        return self.run_tool('FindRidges', args, callback) # returns 1 if error

    def flightline_overlap(self, input, output, resolution=1.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--resolution='{}'".format(resolution))
        return self.run_tool('FlightlineOverlap', args, callback) # returns 1 if error

    def flip_image(self, input, output, direction="vertical", callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--direction='{}'".format(direction))
        return self.run_tool('FlipImage', args, callback) # returns 1 if error

    def flood_order(self, dem, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('FloodOrder', args, callback) # returns 1 if error

    def floor(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('Floor', args, callback) # returns 1 if error

    def flow_accumulation_full_workflow(self, dem, out_dem, out_pntr, out_accum, out_type="Specific Contributing Area", log=False, clip=False, esri_pntr=False, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--out_dem='{}'".format(out_dem))
        args.append("--out_pntr='{}'".format(out_pntr))
        args.append("--out_accum='{}'".format(out_accum))
        args.append("--out_type='{}'".format(out_type))
        if log: args.append("--log")
        if clip: args.append("--clip")
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('FlowAccumulationFullWorkflow', args, callback) # returns 1 if error

    def flow_length_diff(self, d8_pntr, output, esri_pntr=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('FlowLengthDiff', args, callback) # returns 1 if error

    def gamma_correction(self, input, output, gamma=0.5, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--gamma='{}'".format(gamma))
        return self.run_tool('GammaCorrection', args, callback) # returns 1 if error

    def gaussian_filter(self, input, output, sigma=0.75, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--sigma='{}'".format(sigma))
        return self.run_tool('GaussianFilter', args, callback) # returns 1 if error

    def greater_than(self, input1, input2, output, incl_equals=False, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        if incl_equals: args.append("--incl_equals")
        return self.run_tool('GreaterThan', args, callback) # returns 1 if error

    def hack_stream_order(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('HackStreamOrder', args, callback) # returns 1 if error

    def high_pass_filter(self, input, output, filterx=11, filtery=11, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        return self.run_tool('HighPassFilter', args, callback) # returns 1 if error

    def highest_position(self, inputs, output, callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('HighestPosition', args, callback) # returns 1 if error

    def hillshade(self, dem, output, azimuth=315.0, altitude=30.0, zfactor=1.0, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--azimuth='{}'".format(azimuth))
        args.append("--altitude='{}'".format(altitude))
        args.append("--zfactor='{}'".format(zfactor))
        return self.run_tool('Hillshade', args, callback) # returns 1 if error

    def hillslopes(self, d8_pntr, streams, output, esri_pntr=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('Hillslopes', args, callback) # returns 1 if error

    def histogram_equalization(self, input, output, num_tones=256, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--num_tones='{}'".format(num_tones))
        return self.run_tool('HistogramEqualization', args, callback) # returns 1 if error

    def histogram_matching(self, input, histo_file, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--histo_file='{}'".format(histo_file))
        args.append("--output='{}'".format(output))
        return self.run_tool('HistogramMatching', args, callback) # returns 1 if error

    def histogram_matching_two_images(self, input1, input2, output, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('HistogramMatchingTwoImages', args, callback) # returns 1 if error

    def horizon_angle(self, dem, output, max_dist, azimuth=0.0, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--azimuth='{}'".format(azimuth))
        args.append("--max_dist='{}'".format(max_dist))
        return self.run_tool('HorizonAngle', args, callback) # returns 1 if error

    def horton_stream_order(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('HortonStreamOrder', args, callback) # returns 1 if error

    def hypsometric_analysis(self, inputs, watershed, output, callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--watershed='{}'".format(watershed))
        args.append("--output='{}'".format(output))
        return self.run_tool('HypsometricAnalysis', args, callback) # returns 1 if error

    def image_autocorrelation(self, inputs, output, contiguity="Rook", callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--contiguity='{}'".format(contiguity))
        args.append("--output='{}'".format(output))
        return self.run_tool('ImageAutocorrelation', args, callback) # returns 1 if error

    def image_correlation(self, inputs, output, callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('ImageCorrelation', args, callback) # returns 1 if error

    def image_regression(self, input1, input2, output, out_residuals, standardize=False, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        args.append("--out_residuals='{}'".format(out_residuals))
        if standardize: args.append("--standardize")
        return self.run_tool('ImageRegression', args, callback) # returns 1 if error

    def increment(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('Increment', args, callback) # returns 1 if error

    def integer_division(self, input1, input2, output, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('IntegerDivision', args, callback) # returns 1 if error

    def integral_image(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('IntegralImage', args, callback) # returns 1 if error

    def is_no_data(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('IsNoData', args, callback) # returns 1 if error

    def isobasins(self, dem, output, size, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--size='{}'".format(size))
        return self.run_tool('Isobasins', args, callback) # returns 1 if error

    def jenson_snap_pour_points(self, pour_pts, streams, output, snap_dist, callback=default_callback):
        args = []
        args.append("--pour_pts='{}'".format(pour_pts))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        args.append("--snap_dist='{}'".format(snap_dist))
        return self.run_tool('JensonSnapPourPoints', args, callback) # returns 1 if error

    def k_means_clustering(self, inputs, output, out_html, classes, max_iterations=10, class_change=2.0, initialize="diagonal", min_class_size=10, callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        args.append("--out_html='{}'".format(out_html))
        args.append("--classes='{}'".format(classes))
        args.append("--max_iterations='{}'".format(max_iterations))
        args.append("--class_change='{}'".format(class_change))
        args.append("--initialize='{}'".format(initialize))
        args.append("--min_class_size='{}'".format(min_class_size))
        return self.run_tool('KMeansClustering', args, callback) # returns 1 if error

    def k_nearest_mean_filter(self, input, output, filterx=11, filtery=11, k=5, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        args.append("-k='{}'".format(k))
        return self.run_tool('KNearestMeanFilter', args, callback) # returns 1 if error

    def ks_test_for_normality(self, input, output, num_samples, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--num_samples='{}'".format(num_samples))
        return self.run_tool('KSTestForNormality', args, callback) # returns 1 if error

    def kappa_index(self, input1, input2, output, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('KappaIndex', args, callback) # returns 1 if error

    def laplacian_filter(self, input, output, variant="3x3(1)", clip=0.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--variant='{}'".format(variant))
        args.append("--clip='{}'".format(clip))
        return self.run_tool('LaplacianFilter', args, callback) # returns 1 if error

    def laplacian_of_gaussian_filter(self, input, output, sigma=0.75, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--sigma='{}'".format(sigma))
        return self.run_tool('LaplacianOfGaussianFilter', args, callback) # returns 1 if error

    def las_to_ascii(self, inputs, callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        return self.run_tool('LasToAscii', args, callback) # returns 1 if error

    def lee_filter(self, input, output, filterx=11, filtery=11, sigma=10.0, m=5.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        args.append("--sigma='{}'".format(sigma))
        args.append("-m='{}'".format(m))
        return self.run_tool('LeeFilter', args, callback) # returns 1 if error

    def length_of_upstream_channels(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('LengthOfUpstreamChannels', args, callback) # returns 1 if error

    def less_than(self, input1, input2, output, incl_equals=False, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        if incl_equals: args.append("--incl_equals")
        return self.run_tool('LessThan', args, callback) # returns 1 if error

    def lidar_elevation_slice(self, input, output, minz, maxz, cls=False, inclassval=2, outclassval=1, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--minz='{}'".format(minz))
        args.append("--maxz='{}'".format(maxz))
        if cls: args.append("--class")
        args.append("--inclassval='{}'".format(inclassval))
        args.append("--outclassval='{}'".format(outclassval))
        return self.run_tool('LidarElevationSlice', args, callback) # returns 1 if error

    def lidar_ground_point_filter(self, input, output, radius=2.0, slope_threshold=45.0, height_threshold=1.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--radius='{}'".format(radius))
        args.append("--slope_threshold='{}'".format(slope_threshold))
        args.append("--height_threshold='{}'".format(height_threshold))
        return self.run_tool('LidarGroundPointFilter', args, callback) # returns 1 if error

    def lidar_hillshade(self, input, output, azimuth=315.0, altitude=30.0, radius=1.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--azimuth='{}'".format(azimuth))
        args.append("--altitude='{}'".format(altitude))
        args.append("--radius='{}'".format(radius))
        return self.run_tool('LidarHillshade', args, callback) # returns 1 if error

    def lidar_histogram(self, input, output, parameter="elevation", clip=1.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--parameter='{}'".format(parameter))
        args.append("--clip='{}'".format(clip))
        return self.run_tool('LidarHistogram', args, callback) # returns 1 if error

    def lidar_idw_interpolation(self, input, output, exclude_cls, minz, maxz, parameter="elevation", returns="all", resolution=1.0, weight=1.0, radius=2.5, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--parameter='{}'".format(parameter))
        args.append("--returns='{}'".format(returns))
        args.append("--resolution='{}'".format(resolution))
        args.append("--weight='{}'".format(weight))
        args.append("--radius='{}'".format(radius))
        args.append("--exclude_cls='{}'".format(exclude_cls))
        args.append("--minz='{}'".format(minz))
        args.append("--maxz='{}'".format(maxz))
        return self.run_tool('LidarIdwInterpolation', args, callback) # returns 1 if error

    def lidar_info(self, input, output, vlr=False, geokeys=False, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        if vlr: args.append("--vlr")
        if geokeys: args.append("--geokeys")
        return self.run_tool('LidarInfo', args, callback) # returns 1 if error

    def lidar_join(self, inputs, output, callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('LidarJoin', args, callback) # returns 1 if error

    def lidar_kappa_index(self, input1, input2, output, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('LidarKappaIndex', args, callback) # returns 1 if error

    def lidar_nearest_neighbour_gridding(self, input, output, exclude_cls, minz, maxz, parameter="elevation", returns="all", resolution=1.0, radius=2.5, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--parameter='{}'".format(parameter))
        args.append("--returns='{}'".format(returns))
        args.append("--resolution='{}'".format(resolution))
        args.append("--radius='{}'".format(radius))
        args.append("--exclude_cls='{}'".format(exclude_cls))
        args.append("--minz='{}'".format(minz))
        args.append("--maxz='{}'".format(maxz))
        return self.run_tool('LidarNearestNeighbourGridding', args, callback) # returns 1 if error

    def lidar_point_density(self, input, output, exclude_cls, minz, maxz, returns="all", resolution=1.0, radius=2.5, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--returns='{}'".format(returns))
        args.append("--resolution='{}'".format(resolution))
        args.append("--radius='{}'".format(radius))
        args.append("--exclude_cls='{}'".format(exclude_cls))
        args.append("--minz='{}'".format(minz))
        args.append("--maxz='{}'".format(maxz))
        return self.run_tool('LidarPointDensity', args, callback) # returns 1 if error

    def lidar_remove_outliers(self, input, output, radius=2.0, elev_diff=50.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--radius='{}'".format(radius))
        args.append("--elev_diff='{}'".format(elev_diff))
        return self.run_tool('LidarRemoveOutliers', args, callback) # returns 1 if error

    def lidar_segmentation(self, input, output, radius=5.0, norm_diff=10.0, maxzdiff=1.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--radius='{}'".format(radius))
        args.append("--norm_diff='{}'".format(norm_diff))
        args.append("--maxzdiff='{}'".format(maxzdiff))
        return self.run_tool('LidarSegmentation', args, callback) # returns 1 if error

    def lidar_segmentation_based_filter(self, input, output, radius=5.0, norm_diff=2.0, maxzdiff=1.0, classify=False, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--radius='{}'".format(radius))
        args.append("--norm_diff='{}'".format(norm_diff))
        args.append("--maxzdiff='{}'".format(maxzdiff))
        if classify: args.append("--classify")
        return self.run_tool('LidarSegmentationBasedFilter', args, callback) # returns 1 if error

    def lidar_tile(self, input, width_x=1000.0, width_y=1000.0, origin_x=0.0, origin_y=0.0, min_points=0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--width_x='{}'".format(width_x))
        args.append("--width_y='{}'".format(width_y))
        args.append("--origin_x='{}'".format(origin_x))
        args.append("--origin_y='{}'".format(origin_y))
        args.append("--min_points='{}'".format(min_points))
        return self.run_tool('LidarTile', args, callback) # returns 1 if error

    def lidar_tophat_transform(self, input, output, radius=1.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--radius='{}'".format(radius))
        return self.run_tool('LidarTophatTransform', args, callback) # returns 1 if error

    def line_detection_filter(self, input, output, variant="vertical", absvals=False, clip=0.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--variant='{}'".format(variant))
        if absvals: args.append("--absvals")
        args.append("--clip='{}'".format(clip))
        return self.run_tool('LineDetectionFilter', args, callback) # returns 1 if error

    def line_thinning(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('LineThinning', args, callback) # returns 1 if error

    def ln(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('Ln', args, callback) # returns 1 if error

    def log10(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('Log10', args, callback) # returns 1 if error

    def log2(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('Log2', args, callback) # returns 1 if error

    def lowest_position(self, inputs, output, callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('LowestPosition', args, callback) # returns 1 if error

    def majority_filter(self, input, output, filterx=11, filtery=11, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        return self.run_tool('MajorityFilter', args, callback) # returns 1 if error

    def max(self, input1, input2, output, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Max', args, callback) # returns 1 if error

    def max_absolute_overlay(self, inputs, output, callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('MaxAbsoluteOverlay', args, callback) # returns 1 if error

    def max_anisotropy_dev(self, dem, out_mag, out_scale, max_scale, min_scale=3, step=2, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--out_mag='{}'".format(out_mag))
        args.append("--out_scale='{}'".format(out_scale))
        args.append("--min_scale='{}'".format(min_scale))
        args.append("--max_scale='{}'".format(max_scale))
        args.append("--step='{}'".format(step))
        return self.run_tool('MaxAnisotropyDev', args, callback) # returns 1 if error

    def max_branch_length(self, dem, output, log=False, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if log: args.append("--log")
        return self.run_tool('MaxBranchLength', args, callback) # returns 1 if error

    def max_downslope_elev_change(self, dem, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('MaxDownslopeElevChange', args, callback) # returns 1 if error

    def max_elevation_deviation(self, dem, out_mag, out_scale, min_scale, max_scale, step=10, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--out_mag='{}'".format(out_mag))
        args.append("--out_scale='{}'".format(out_scale))
        args.append("--min_scale='{}'".format(min_scale))
        args.append("--max_scale='{}'".format(max_scale))
        args.append("--step='{}'".format(step))
        return self.run_tool('MaxElevationDeviation', args, callback) # returns 1 if error

    def max_overlay(self, inputs, output, callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('MaxOverlay', args, callback) # returns 1 if error

    def max_upslope_flowpath_length(self, dem, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('MaxUpslopeFlowpathLength', args, callback) # returns 1 if error

    def maximum_filter(self, input, output, filterx=11, filtery=11, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        return self.run_tool('MaximumFilter', args, callback) # returns 1 if error

    def mean_filter(self, input, output, filterx=3, filtery=3, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        return self.run_tool('MeanFilter', args, callback) # returns 1 if error

    def median_filter(self, input, output, filterx=11, filtery=11, sig_digits=2, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        args.append("--sig_digits='{}'".format(sig_digits))
        return self.run_tool('MedianFilter', args, callback) # returns 1 if error

    def min(self, input1, input2, output, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Min', args, callback) # returns 1 if error

    def min_absolute_overlay(self, inputs, output, callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('MinAbsoluteOverlay', args, callback) # returns 1 if error

    def min_downslope_elev_change(self, dem, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('MinDownslopeElevChange', args, callback) # returns 1 if error

    def min_max_contrast_stretch(self, input, output, min_val, max_val, num_tones=256, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--min_val='{}'".format(min_val))
        args.append("--max_val='{}'".format(max_val))
        args.append("--num_tones='{}'".format(num_tones))
        return self.run_tool('MinMaxContrastStretch', args, callback) # returns 1 if error

    def min_overlay(self, inputs, output, callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        return self.run_tool('MinOverlay', args, callback) # returns 1 if error

    def minimum_filter(self, input, output, filterx=11, filtery=11, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        return self.run_tool('MinimumFilter', args, callback) # returns 1 if error

    def modified_k_means_clustering(self, inputs, output, out_html, merger_dist, start_clusters=1000, max_iterations=10, class_change=2.0, callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        args.append("--out_html='{}'".format(out_html))
        args.append("--start_clusters='{}'".format(start_clusters))
        args.append("--merger_dist='{}'".format(merger_dist))
        args.append("--max_iterations='{}'".format(max_iterations))
        args.append("--class_change='{}'".format(class_change))
        return self.run_tool('ModifiedKMeansClustering', args, callback) # returns 1 if error

    def modulo(self, input1, input2, output, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Modulo', args, callback) # returns 1 if error

    def mosaic(self, inputs, output, method="cc", callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        args.append("--method='{}'".format(method))
        return self.run_tool('Mosaic', args, callback) # returns 1 if error

    def multiply(self, input1, input2, output, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Multiply', args, callback) # returns 1 if error

    def multiscale_topographic_position_image(self, local, meso, broad, output, lightness=1.2, callback=default_callback):
        args = []
        args.append("--local='{}'".format(local))
        args.append("--meso='{}'".format(meso))
        args.append("--broad='{}'".format(broad))
        args.append("--output='{}'".format(output))
        args.append("--lightness='{}'".format(lightness))
        return self.run_tool('MultiscaleTopographicPositionImage', args, callback) # returns 1 if error

    def negate(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('Negate', args, callback) # returns 1 if error

    def new_raster_from_base(self, base, output, value="nodata", data_type="float", callback=default_callback):
        args = []
        args.append("--base='{}'".format(base))
        args.append("--output='{}'".format(output))
        args.append("--value='{}'".format(value))
        args.append("--data_type='{}'".format(data_type))
        return self.run_tool('NewRasterFromBase', args, callback) # returns 1 if error

    def normal_vectors(self, input, output, radius=1.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--radius='{}'".format(radius))
        return self.run_tool('NormalVectors', args, callback) # returns 1 if error

    def normalized_difference_vegetation_index(self, nir, red, output, clip=0.0, osavi=False, callback=default_callback):
        args = []
        args.append("--nir='{}'".format(nir))
        args.append("--red='{}'".format(red))
        args.append("--output='{}'".format(output))
        args.append("--clip='{}'".format(clip))
        if osavi: args.append("--osavi")
        return self.run_tool('NormalizedDifferenceVegetationIndex', args, callback) # returns 1 if error

    def Not(self, input1, input2, output, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Not', args, callback) # returns 1 if error

    def not_equal_to(self, input1, input2, output, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('NotEqualTo', args, callback) # returns 1 if error

    def num_downslope_neighbours(self, dem, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('NumDownslopeNeighbours', args, callback) # returns 1 if error

    def num_inflowing_neighbours(self, dem, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('NumInflowingNeighbours', args, callback) # returns 1 if error

    def num_upslope_neighbours(self, dem, output, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        return self.run_tool('NumUpslopeNeighbours', args, callback) # returns 1 if error

    def olympic_filter(self, input, output, filterx=11, filtery=11, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        return self.run_tool('OlympicFilter', args, callback) # returns 1 if error

    def opening(self, input, output, filterx=11, filtery=11, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        return self.run_tool('Opening', args, callback) # returns 1 if error

    def Or(self, input1, input2, output, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Or', args, callback) # returns 1 if error

    def panchromatic_sharpening(self, red, green, blue, composite, pan, output, method="brovey", callback=default_callback):
        args = []
        args.append("--red='{}'".format(red))
        args.append("--green='{}'".format(green))
        args.append("--blue='{}'".format(blue))
        args.append("--composite='{}'".format(composite))
        args.append("--pan='{}'".format(pan))
        args.append("--output='{}'".format(output))
        args.append("--method='{}'".format(method))
        return self.run_tool('PanchromaticSharpening', args, callback) # returns 1 if error

    def pennock_landform_class(self, dem, output, slope=3.0, prof=0.1, plan=0.0, zfactor=1.0, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--slope='{}'".format(slope))
        args.append("--prof='{}'".format(prof))
        args.append("--plan='{}'".format(plan))
        args.append("--zfactor='{}'".format(zfactor))
        return self.run_tool('PennockLandformClass', args, callback) # returns 1 if error

    def percent_elev_range(self, dem, output, filterx=3, filtery=3, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        return self.run_tool('PercentElevRange', args, callback) # returns 1 if error

    def percent_equal_to(self, inputs, comparison, output, callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--comparison='{}'".format(comparison))
        args.append("--output='{}'".format(output))
        return self.run_tool('PercentEqualTo', args, callback) # returns 1 if error

    def percent_greater_than(self, inputs, comparison, output, callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--comparison='{}'".format(comparison))
        args.append("--output='{}'".format(output))
        return self.run_tool('PercentGreaterThan', args, callback) # returns 1 if error

    def percent_less_than(self, inputs, comparison, output, callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--comparison='{}'".format(comparison))
        args.append("--output='{}'".format(output))
        return self.run_tool('PercentLessThan', args, callback) # returns 1 if error

    def percentage_contrast_stretch(self, input, output, clip=0.0, tail="both", num_tones=256, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--clip='{}'".format(clip))
        args.append("--tail='{}'".format(tail))
        args.append("--num_tones='{}'".format(num_tones))
        return self.run_tool('PercentageContrastStretch', args, callback) # returns 1 if error

    def percentile_filter(self, input, output, filterx=11, filtery=11, sig_digits=2, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        args.append("--sig_digits='{}'".format(sig_digits))
        return self.run_tool('PercentileFilter', args, callback) # returns 1 if error

    def pick_from_list(self, inputs, pos_input, output, callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--pos_input='{}'".format(pos_input))
        args.append("--output='{}'".format(output))
        return self.run_tool('PickFromList', args, callback) # returns 1 if error

    def plan_curvature(self, dem, output, zfactor=1.0, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor='{}'".format(zfactor))
        return self.run_tool('PlanCurvature', args, callback) # returns 1 if error

    def power(self, input1, input2, output, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Power', args, callback) # returns 1 if error

    def prewitt_filter(self, input, output, clip=0.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--clip='{}'".format(clip))
        return self.run_tool('PrewittFilter', args, callback) # returns 1 if error

    def profile_curvature(self, dem, output, zfactor=1.0, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor='{}'".format(zfactor))
        return self.run_tool('ProfileCurvature', args, callback) # returns 1 if error

    def quantiles(self, input, output, num_quantiles=4, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--num_quantiles='{}'".format(num_quantiles))
        return self.run_tool('Quantiles', args, callback) # returns 1 if error

    def radius_of_gyration(self, input, output, text_output=False, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        if text_output: args.append("--text_output")
        return self.run_tool('RadiusOfGyration', args, callback) # returns 1 if error

    def random_field(self, base, output, callback=default_callback):
        args = []
        args.append("--base='{}'".format(base))
        args.append("--output='{}'".format(output))
        return self.run_tool('RandomField', args, callback) # returns 1 if error

    def random_sample(self, base, output, num_samples=1000, callback=default_callback):
        args = []
        args.append("--base='{}'".format(base))
        args.append("--output='{}'".format(output))
        args.append("--num_samples='{}'".format(num_samples))
        return self.run_tool('RandomSample', args, callback) # returns 1 if error

    def range_filter(self, input, output, filterx=11, filtery=11, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        return self.run_tool('RangeFilter', args, callback) # returns 1 if error

    def raster_cell_assignment(self, input, output, assign="column", callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--assign='{}'".format(assign))
        return self.run_tool('RasterCellAssignment', args, callback) # returns 1 if error

    def raster_histogram(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('RasterHistogram', args, callback) # returns 1 if error

    def raster_summary_stats(self, input, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        return self.run_tool('RasterSummaryStats', args, callback) # returns 1 if error

    def reciprocal(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('Reciprocal', args, callback) # returns 1 if error

    def reclass(self, input, output, reclass_vals, assign_mode=False, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--reclass_vals='{}'".format(reclass_vals))
        if assign_mode: args.append("--assign_mode")
        return self.run_tool('Reclass', args, callback) # returns 1 if error

    def reclass_equal_interval(self, input, output, start_val, end_val, interval=10.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--interval='{}'".format(interval))
        args.append("--start_val='{}'".format(start_val))
        args.append("--end_val='{}'".format(end_val))
        return self.run_tool('ReclassEqualInterval', args, callback) # returns 1 if error

    def reclass_from_file(self, input, reclass_file, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--reclass_file='{}'".format(reclass_file))
        args.append("--output='{}'".format(output))
        return self.run_tool('ReclassFromFile', args, callback) # returns 1 if error

    def relative_aspect(self, dem, output, azimuth=0.0, zfactor=1.0, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--azimuth='{}'".format(azimuth))
        args.append("--zfactor='{}'".format(zfactor))
        return self.run_tool('RelativeAspect', args, callback) # returns 1 if error

    def relative_stream_power_index(self, sca, slope, output, exponent=1.0, callback=default_callback):
        args = []
        args.append("--sca='{}'".format(sca))
        args.append("--slope='{}'".format(slope))
        args.append("--output='{}'".format(output))
        args.append("--exponent='{}'".format(exponent))
        return self.run_tool('RelativeStreamPowerIndex', args, callback) # returns 1 if error

    def relative_topographic_position(self, dem, output, filterx=11, filtery=11, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        return self.run_tool('RelativeTopographicPosition', args, callback) # returns 1 if error

    def remove_off_terrain_objects(self, dem, output, filter=11, slope=15.0, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--filter='{}'".format(filter))
        args.append("--slope='{}'".format(slope))
        return self.run_tool('RemoveOffTerrainObjects', args, callback) # returns 1 if error

    def remove_short_streams(self, d8_pntr, streams, output, min_length, esri_pntr=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        args.append("--min_length='{}'".format(min_length))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('RemoveShortStreams', args, callback) # returns 1 if error

    def remove_spurs(self, input, output, iterations=10, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--iterations='{}'".format(iterations))
        return self.run_tool('RemoveSpurs', args, callback) # returns 1 if error

    def resample(self, inputs, destination, method="cc", callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--destination='{}'".format(destination))
        args.append("--method='{}'".format(method))
        return self.run_tool('Resample', args, callback) # returns 1 if error

    def rescale_value_range(self, input, output, out_min_val, out_max_val, clip_min, clip_max, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--out_min_val='{}'".format(out_min_val))
        args.append("--out_max_val='{}'".format(out_max_val))
        args.append("--clip_min='{}'".format(clip_min))
        args.append("--clip_max='{}'".format(clip_max))
        return self.run_tool('RescaleValueRange', args, callback) # returns 1 if error

    def rgb_to_ihs(self, red, green, blue, composite, intensity, hue, saturation, callback=default_callback):
        args = []
        args.append("--red='{}'".format(red))
        args.append("--green='{}'".format(green))
        args.append("--blue='{}'".format(blue))
        args.append("--composite='{}'".format(composite))
        args.append("--intensity='{}'".format(intensity))
        args.append("--hue='{}'".format(hue))
        args.append("--saturation='{}'".format(saturation))
        return self.run_tool('RgbToIhs', args, callback) # returns 1 if error

    def rho8_pointer(self, dem, output, esri_pntr=False, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('Rho8Pointer', args, callback) # returns 1 if error

    def roberts_cross_filter(self, input, output, clip=0.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--clip='{}'".format(clip))
        return self.run_tool('RobertsCrossFilter', args, callback) # returns 1 if error

    def root_mean_square_error(self, input, base, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--base='{}'".format(base))
        return self.run_tool('RootMeanSquareError', args, callback) # returns 1 if error

    def round(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('Round', args, callback) # returns 1 if error

    def ruggedness_index(self, dem, output, zfactor=1.0, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor='{}'".format(zfactor))
        return self.run_tool('RuggednessIndex', args, callback) # returns 1 if error

    def scharr_filter(self, input, output, clip=0.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--clip='{}'".format(clip))
        return self.run_tool('ScharrFilter', args, callback) # returns 1 if error

    def sediment_transport_index(self, sca, slope, output, sca_exponent=0.4, slope_exponent=1.3, callback=default_callback):
        args = []
        args.append("--sca='{}'".format(sca))
        args.append("--slope='{}'".format(slope))
        args.append("--output='{}'".format(output))
        args.append("--sca_exponent='{}'".format(sca_exponent))
        args.append("--slope_exponent='{}'".format(slope_exponent))
        return self.run_tool('SedimentTransportIndex', args, callback) # returns 1 if error

    def set_nodata_value(self, input, output, back_value=0.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--back_value='{}'".format(back_value))
        return self.run_tool('SetNodataValue', args, callback) # returns 1 if error

    def shreve_stream_magnitude(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('ShreveStreamMagnitude', args, callback) # returns 1 if error

    def sigmoidal_contrast_stretch(self, input, output, cutoff=0.0, gain=1.0, num_tones=256, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--cutoff='{}'".format(cutoff))
        args.append("--gain='{}'".format(gain))
        args.append("--num_tones='{}'".format(num_tones))
        return self.run_tool('SigmoidalContrastStretch', args, callback) # returns 1 if error

    def sin(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('Sin', args, callback) # returns 1 if error

    def sinh(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('Sinh', args, callback) # returns 1 if error

    def sink(self, dem, output, zero_background=False, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if zero_background: args.append("--zero_background")
        return self.run_tool('Sink', args, callback) # returns 1 if error

    def slope(self, dem, output, zfactor=1.0, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor='{}'".format(zfactor))
        return self.run_tool('Slope', args, callback) # returns 1 if error

    def slope_vs_elevation_plot(self, inputs, watershed, output, callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--watershed='{}'".format(watershed))
        args.append("--output='{}'".format(output))
        return self.run_tool('SlopeVsElevationPlot', args, callback) # returns 1 if error

    def snap_pour_points(self, pour_pts, flow_accum, output, snap_dist, callback=default_callback):
        args = []
        args.append("--pour_pts='{}'".format(pour_pts))
        args.append("--flow_accum='{}'".format(flow_accum))
        args.append("--output='{}'".format(output))
        args.append("--snap_dist='{}'".format(snap_dist))
        return self.run_tool('SnapPourPoints', args, callback) # returns 1 if error

    def sobel_filter(self, input, output, variant="3x3", clip=0.0, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--variant='{}'".format(variant))
        args.append("--clip='{}'".format(clip))
        return self.run_tool('SobelFilter', args, callback) # returns 1 if error

    def split_colour_composite(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('SplitColourComposite', args, callback) # returns 1 if error

    def square(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('Square', args, callback) # returns 1 if error

    def square_root(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('SquareRoot', args, callback) # returns 1 if error

    def standard_deviation_contrast_stretch(self, input, output, stdev=2.0, num_tones=256, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--stdev='{}'".format(stdev))
        args.append("--num_tones='{}'".format(num_tones))
        return self.run_tool('StandardDeviationContrastStretch', args, callback) # returns 1 if error

    def standard_deviation_filter(self, input, output, filterx=11, filtery=11, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        return self.run_tool('StandardDeviationFilter', args, callback) # returns 1 if error

    def strahler_order_basins(self, d8_pntr, streams, output, esri_pntr=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('StrahlerOrderBasins', args, callback) # returns 1 if error

    def strahler_stream_order(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('StrahlerStreamOrder', args, callback) # returns 1 if error

    def stream_link_class(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('StreamLinkClass', args, callback) # returns 1 if error

    def stream_link_identifier(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('StreamLinkIdentifier', args, callback) # returns 1 if error

    def stream_link_length(self, d8_pntr, linkid, output, esri_pntr=False, zero_background=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--linkid='{}'".format(linkid))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('StreamLinkLength', args, callback) # returns 1 if error

    def stream_link_slope(self, d8_pntr, linkid, dem, output, esri_pntr=False, zero_background=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--linkid='{}'".format(linkid))
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('StreamLinkSlope', args, callback) # returns 1 if error

    def stream_slope_continuous(self, d8_pntr, streams, dem, output, esri_pntr=False, zero_background=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('StreamSlopeContinuous', args, callback) # returns 1 if error

    def subbasins(self, d8_pntr, streams, output, esri_pntr=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('Subbasins', args, callback) # returns 1 if error

    def subtract(self, input1, input2, output, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Subtract', args, callback) # returns 1 if error

    def tan(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('Tan', args, callback) # returns 1 if error

    def tangential_curvature(self, dem, output, zfactor=1.0, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor='{}'".format(zfactor))
        return self.run_tool('TangentialCurvature', args, callback) # returns 1 if error

    def tanh(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('Tanh', args, callback) # returns 1 if error

    def thicken_raster_line(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('ThickenRasterLine', args, callback) # returns 1 if error

    def to_degrees(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('ToDegrees', args, callback) # returns 1 if error

    def to_radians(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('ToRadians', args, callback) # returns 1 if error

    def tophat_transform(self, input, output, filterx=11, filtery=11, variant="white", callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        args.append("--variant='{}'".format(variant))
        return self.run_tool('TophatTransform', args, callback) # returns 1 if error

    def topological_stream_order(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('TopologicalStreamOrder', args, callback) # returns 1 if error

    def total_curvature(self, dem, output, zfactor=1.0, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--output='{}'".format(output))
        args.append("--zfactor='{}'".format(zfactor))
        return self.run_tool('TotalCurvature', args, callback) # returns 1 if error

    def total_filter(self, input, output, filterx=11, filtery=11, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--filterx='{}'".format(filterx))
        args.append("--filtery='{}'".format(filtery))
        return self.run_tool('TotalFilter', args, callback) # returns 1 if error

    def trace_downslope_flowpaths(self, seed_pts, d8_pntr, output, esri_pntr=False, zero_background=False, callback=default_callback):
        args = []
        args.append("--seed_pts='{}'".format(seed_pts))
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('TraceDownslopeFlowpaths', args, callback) # returns 1 if error

    def tributary_identifier(self, d8_pntr, streams, output, esri_pntr=False, zero_background=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--streams='{}'".format(streams))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        if zero_background: args.append("--zero_background")
        return self.run_tool('TributaryIdentifier', args, callback) # returns 1 if error

    def truncate(self, input, output, num_decimals, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        args.append("--num_decimals='{}'".format(num_decimals))
        return self.run_tool('Truncate', args, callback) # returns 1 if error

    def turning_bands_simulation(self, base, output, range, iterations=1000, callback=default_callback):
        args = []
        args.append("--base='{}'".format(base))
        args.append("--output='{}'".format(output))
        args.append("--range='{}'".format(range))
        args.append("--iterations='{}'".format(iterations))
        return self.run_tool('TurningBandsSimulation', args, callback) # returns 1 if error

    def viewshed(self, dem, stations, output, height=2.0, callback=default_callback):
        args = []
        args.append("--dem='{}'".format(dem))
        args.append("--stations='{}'".format(stations))
        args.append("--output='{}'".format(output))
        args.append("--height='{}'".format(height))
        return self.run_tool('Viewshed', args, callback) # returns 1 if error

    def watershed(self, d8_pntr, pour_pts, output, esri_pntr=False, callback=default_callback):
        args = []
        args.append("--d8_pntr='{}'".format(d8_pntr))
        args.append("--pour_pts='{}'".format(pour_pts))
        args.append("--output='{}'".format(output))
        if esri_pntr: args.append("--esri_pntr")
        return self.run_tool('Watershed', args, callback) # returns 1 if error

    def weighted_sum(self, inputs, output, weights, callback=default_callback):
        args = []
        args.append("--inputs='{}'".format(inputs))
        args.append("--output='{}'".format(output))
        args.append("--weights='{}'".format(weights))
        return self.run_tool('WeightedSum', args, callback) # returns 1 if error

    def wetness_index(self, sca, slope, output, callback=default_callback):
        args = []
        args.append("--sca='{}'".format(sca))
        args.append("--slope='{}'".format(slope))
        args.append("--output='{}'".format(output))
        return self.run_tool('WetnessIndex', args, callback) # returns 1 if error

    def write_function_memory_insertion(self, input1, input2, input3, output, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--input3='{}'".format(input3))
        args.append("--output='{}'".format(output))
        return self.run_tool('WriteFunctionMemoryInsertion', args, callback) # returns 1 if error

    def xor(self, input1, input2, output, callback=default_callback):
        args = []
        args.append("--input1='{}'".format(input1))
        args.append("--input2='{}'".format(input2))
        args.append("--output='{}'".format(output))
        return self.run_tool('Xor', args, callback) # returns 1 if error

    def z_scores(self, input, output, callback=default_callback):
        args = []
        args.append("--input='{}'".format(input))
        args.append("--output='{}'".format(output))
        return self.run_tool('ZScores', args, callback) # returns 1 if error
