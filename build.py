#!/usr/bin/env python
import os
import sys
from subprocess import call

try:
    updateCargo = False
    cleanCode = False
    buildCode = True
    mode = 'check' # 'check', 'debug', or 'release'

    # Change the current directory
    dir_path = os.path.dirname(os.path.realpath(__file__))
    os.chdir(dir_path)

    if updateCargo:
        # Update #
        retcode = call(['cargo', 'update'], shell=False)
        if retcode < 0:
            print >>sys.stderr, "Child was terminated by signal", -retcode
        else:
            print >>sys.stderr, "Update successful"

    if cleanCode:
        # Clean #
        retcode = call(['cargo', 'clean'], shell=False)
        if retcode < 0:
            print >>sys.stderr, "Child was terminated by signal", -retcode
        else:
            print >>sys.stderr, "Clean successful"

    if buildCode:
        # Build #
        if mode == 'release':
            retcode = call(['cargo', 'build', '--release'], shell=False)
        elif mode == 'check':
            retcode = call(['cargo', 'check'], shell=False)
        else:
            retcode = call(['cargo', 'debug'], shell=False)
        #retcode = call(['cargo', 'build'], shell=False)
        if retcode < 0:
            print >>sys.stderr, "Child was terminated by signal", -retcode
        else:
            print >>sys.stderr, "Build executed successfully"

except OSError as e:
    print >>sys.stderr, "Execution failed:", e
