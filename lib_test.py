''' This script is intended to experiment with the use of a whitebox_tools shared library (DLL).
It is experimental and is not intended for widespread use.
'''
import os
from sys import platform
from ctypes import cdll, c_int, c_char_p, POINTER, c_size_t


def call_tool(name, args):
    # Change the current directory
    dir_path = os.path.dirname(os.path.realpath(__file__))
    os.chdir(dir_path)

    if platform == 'darwin':
        prefix = 'lib'
        ext = 'dylib'
    elif platform == 'win32':
        prefix = ''
        ext = 'dll'
    else:
        prefix = 'lib'
        ext = 'so'

    wb_tools = cdll.LoadLibrary(
        'target/release/{}whitebox_tools.{}'.format(prefix, ext))

    wb_tools.run_tool.restype = c_int
    wb_tools.run_tool.argtypes = [
        c_char_p, POINTER(c_char_p), c_size_t]

    wb_tools.print_tool.argtypes = [
        c_char_p]

    args_list = (c_char_p * len(args))(*args)
    ret = wb_tools.run_tool(name, args_list, len(args_list))
    print "Return value:", ret


TOOL_NAME = "slope"
TOOL_ARGS = ["--wd=\"/Users/johnlindsay/Documents/data/GarveyGlenWatershed/\"",
             "--input=\"DEM final.dep\"",
             "--output=\"tmp13.dep\"",
             "-v"]
call_tool(TOOL_NAME, TOOL_ARGS)
