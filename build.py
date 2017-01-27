#!/usr/bin/env python
import os
import sys
from subprocess import call

try:
    updateCargo = True
    cleanCode = True
    buildCode = True
    mode = 'release'

    # Change the current directory
    dir_path = os.path.dirname(os.path.realpath(__file__))
    os.chdir(dir_path)

    if updateCargo:
        # Update #
        retcode = call(['cargo', 'update'], shell=False)
        if retcode < 0:
            print >>sys.stderr, "Child was terminated by signal", -retcode
        else:
            print >>sys.stderr, "Process successfully exeicuted"

    if cleanCode:
        # Clean #
        retcode = call(['cargo', 'clean'], shell=False)
        if retcode < 0:
            print >>sys.stderr, "Child was terminated by signal", -retcode
        else:
            print >>sys.stderr, "Process successfully exeicuted"

    if buildCode:
        # Build #
        if mode == 'release':
            retcode = call(['cargo', 'build', '--release'], shell=False)
        else:
            retcode = call(['cargo', 'build'], shell=False)
        #retcode = call(['cargo', 'build'], shell=False)
        if retcode < 0:
            print >>sys.stderr, "Child was terminated by signal", -retcode
        else:
            print >>sys.stderr, "Process executed successfully"

except OSError as e:
    print >>sys.stderr, "Execution failed:", e
