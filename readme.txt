WhiteboxTools

The main tool library is contained in the whitebox_tools (or whitebox_tools.exe on 
MS Windows) file. This is a command-line program that can be run from a terminal, i.e. 
command prompt. For details on usage, change the working directory (cd) to this folder 
and type the following at the command prompt:

./whitebox_tools --help

The following commands are recognized:

--cd, --wd       Changes the working directory; used in conjunction with --run flag.
-h, --help       Prints help information.
-l, --license    Prints the whitebox-tools license.
--listtools      Lists all available tools. Keywords may also be used, --listtools slope.
-r, --run        Runs a tool; used in conjuction with --wd flag; -r="LidarInfo".
--toolhelp       Prints the help associated with a tool; --toolhelp="LidarInfo".
--toolparameters Prints the parameters (in json form) for a specific tool; --toolparameters="LidarInfo".
-v               Verbose mode. Without this flag, tool outputs will not be printed.
--viewcode       Opens the source code of a tool in a web browser; --viewcode="LidarInfo".
--version        Prints the version information.

Example Usage:

./whitebox-tools -r=lidar_info --cd="/path/to/data/" -i=input.las --vlr --geokeys


The WhiteboxTools library may also be called from Python automation scripts. The 
whitebox_tools.py script can be used as an easy way of interfacing with the various 
commands. To use this script, simply use the following import:

from whitebox_tools import WhiteboxTools

See the whitebox_example.py script for more details on how to interface with WhiteboxTools 
from Python.

Additionally, included in this directory is the WhiteboxTools Runner, a simple Tkinter 
user-interface that allows users to run the WhiteboxTools tools, with convenience for 
specifying tool parameters. To run this interface, simply type:

python3 wb_runner.py

At the command prompt (after cd'ing to this folder, which contains the script).

WhiteboxTools is distributed under a permissive MIT open-source license. See LICENSE.txt 
for more details.