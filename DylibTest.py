import os
import sys
from sys import platform
from ctypes import cdll, c_int, c_float, c_int32, c_char_p

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

wb_tools = cdll.LoadLibrary('target/release/{}whitebox_tools.{}'.format(prefix, ext))

wb_tools.run_tool.restype = c_int
wb_tools.run_tool.argtypes = [c_char_p, c_char_p]
args = "--i /Users/johnlindsay/Documents/Data/points.las" + " --vlr"
j = wb_tools.run_tool("lidar_info", args)
print("Return value: {}".format(j))

print("done!")
