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
        mode = 'release'  # 'check', 'debug', or 'release'

        # Change the current directory
        dir_path = os.path.dirname(os.path.realpath(__file__))
        os.chdir(dir_path)

        if update_cargo:
            # Update #
            retcode = call(['cargo', 'update'], shell=False)
            if retcode < 0:
                print("Child was terminated by signal", -
                      retcode, file=sys.stderr)
            else:
                print("Update successful", file=sys.stderr)

        if clean_code:
            # Clean #
            retcode = call(['cargo', 'clean'], shell=False)
            if retcode < 0:
                print("Child was terminated by signal", -
                      retcode, file=sys.stderr)
            else:
                print("Clean successful", file=sys.stderr)

        if doc_code:
            # Clean #
            retcode = call(['cargo', 'doc'], shell=False)
            if retcode < 0:
                print("Child was terminated by signal", -
                      retcode, file=sys.stderr)
            else:
                print("Clean successful", file=sys.stderr)

        if build_code:
            # Build #
            if mode == 'release':
                retcode = call(['env', 'RUSTFLAGS=-C target-cpu=native', 'CARGO_INCREMENTAL=1',
                                'cargo', 'build', '--release'], shell=False)
            elif mode == 'check':
                retcode = call(['cargo', 'check'], shell=False)
            else:
                retcode = call(['cargo', 'build'], shell=False)

            if retcode < 0:
                print("Child was terminated by signal", -
                      retcode, file=sys.stderr)
            else:
                print("Build executed successfully", file=sys.stderr)

    except OSError as err:
        print("Execution failed:", err, file=sys.stderr)


main()
