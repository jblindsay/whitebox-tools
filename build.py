#!/usr/bin/env python
''' this module is used to build whitebox-tools.
'''
import os
import sys
from subprocess import call


def main():
    ''' main function
    '''
    try:
        update_cargo = False
        clean_code = False
        doc_code = False
        build_code = True
        mode = 'debug'  # 'check', 'debug', or 'release'

        # Change the current directory
        dir_path = os.path.dirname(os.path.realpath(__file__))
        os.chdir(dir_path)

        if update_cargo:
            # Update #
            retcode = call(['cargo', 'update'], shell=False)
            if retcode < 0:
                print >>sys.stderr, "Child was terminated by signal", -retcode
            else:
                print >>sys.stderr, "Update successful"

        if clean_code:
            # Clean #
            retcode = call(['cargo', 'clean'], shell=False)
            if retcode < 0:
                print >>sys.stderr, "Child was terminated by signal", -retcode
            else:
                print >>sys.stderr, "Clean successful"

        if doc_code:
            # Clean #
            retcode = call(['cargo', 'doc'], shell=False)
            if retcode < 0:
                print >>sys.stderr, "Child was terminated by signal", -retcode
            else:
                print >>sys.stderr, "Clean successful"

        if build_code:
            # Build #
            if mode == 'release':
                retcode = call(['cargo', 'build', '--release'], shell=False)
            elif mode == 'check':
                retcode = call(['cargo', 'check'], shell=False)
            else:
                retcode = call(['cargo', 'build'], shell=False)

            if retcode < 0:
                print >>sys.stderr, "Child was terminated by signal", -retcode
            else:
                print >>sys.stderr, "Build executed successfully"

    except OSError as err:
        print >>sys.stderr, "Execution failed:", err


main()
