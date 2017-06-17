#!/usr/bin/env python
''' This file is intended to be a helper for running whitebox-tools plugins from a Python script.
See whitebox_example.py for an example of how to use it.
'''
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
    # exe_path = path.dirname(path.abspath(__file__))
    # wd = ""
    # verbose = True

    def __init__(self):
        self.exe_path = path.dirname(path.abspath(__file__))
        self.wkdir = ""
        self.verbose = True
        self.cancel_op = False

    if platform == 'win32':
        ext = '.exe'
    else:
        ext = ''

    exe_name = "whitebox_tools{}".format(ext)

    def set_whitebox_dir(self, path_str):
        ''' Sets the directory to the whitebox - tools executable file.
        '''
        self.exe_path = path_str

    def set_working_dir(self, path_str):
        ''' Sets the working directory.
        '''
        self.wkdir = path_str

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

            if self.wkdir.strip() != "":
                args2.append("--wd=\"{}\"".format(self.wkdir))

            for arg in args:
                args2.append(arg)

            # args_str = args_str[:-1]
            # a.append("--args=\"{}\"".format(args_str))

            if self.verbose:
                args2.append("-v")

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
        ''' Retrieves the version information for whitebox - tools.
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

    def tool_help(self, tool_name):
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

    def list_tools(self):
        ''' Lists all available tools in whitebox - tools.
        '''
        try:
            os.chdir(self.exe_path)
            args = []
            args.append("." + os.path.sep + self.exe_name)
            args.append("--listtools")

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
