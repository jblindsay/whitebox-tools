#!/usr/bin/env python

# This file is intended to be a helper for running whitebox-tools plugins from a Python script.
# See whitebox_example.py for an example of how to use it.
import os
import sys
import subprocess
from sys import platform

exe_path = os.path.dirname(os.path.abspath(__file__))
wd = ""
verbose = True

if platform == 'win32':
    ext = '.exe'
else:
    ext = ''

exe_name = "whitebox-tools{}".format(ext)


def set_whitebox_dir(path):
    global exe_path
    exe_path = path

def set_working_dir(path):
    global wd
    wd = path

def set_verbose_mode(val = True):
    global verbose
    verbose = val

def default_callback(str):
    print(str)

def run_tool(tool_name, args, callback = default_callback):
    try:
        os.chdir(exe_path)
        a = []
        a.append("." + os.path.sep + exe_name)
        a.append("--run=\"{}\"".format(tool_name))

        if len(wd) > 0:
            a.append("--wd=\"{}\"".format(wd))

        args_str = ""
        for s in args:
            args_str += s.replace("\"", "") + " "

        args_str = args_str[:-1]
        a.append("--args=\"{}\"".format(args_str))

        if verbose:
            a.append("-v")

        # print a
        ps = subprocess.Popen(a, shell=False, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, bufsize=1, universal_newlines=True)

        while True:
            line = ps.stdout.readline()
            if line != '':
                callback(line.strip())
            else:
                break

        return 0
    except Exception, e:
        return 1
        print e


def help():
    try:
        os.chdir(exe_path)
        a = []
        a.append("." + os.path.sep + exe_name)
        a.append("-h")

        ps = subprocess.Popen(a, shell=False, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, bufsize=1, universal_newlines=True)
        ret = ""
        while True:
            line = ps.stdout.readline()
            if line != '':
                ret += line
            else:
                break

        return ret
    except Exception, e:
        return e

def license():
    try:
        os.chdir(exe_path)
        a = []
        a.append("." + os.path.sep + exe_name)
        a.append("--license")

        ps = subprocess.Popen(a, shell=False, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, bufsize=1, universal_newlines=True)
        ret = ""
        while True:
            line = ps.stdout.readline()
            if line != '':
                ret += line
            else:
                break

        return ret
    except Exception, e:
        return e

def version():
    try:
        os.chdir(exe_path)
        a = []
        a.append("." + os.path.sep + exe_name)
        a.append("--version")

        ps = subprocess.Popen(a, shell=False, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, bufsize=1, universal_newlines=True)
        ret = ""
        while True:
            line = ps.stdout.readline()
            if line != '':
                ret += line
            else:
                break

        return ret
    except Exception, e:
        return e

def tool_help(tool_name):
    try:
        os.chdir(exe_path)
        a = []
        a.append("." + os.path.sep + exe_name)
        a.append("--toolhelp={}".format(tool_name))

        ps = subprocess.Popen(a, shell=False, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, bufsize=1, universal_newlines=True)
        ret = ""
        while True:
            line = ps.stdout.readline()
            if line != '':
                ret += line
            else:
                break

        return ret
    except Exception, e:
        return e

def list_tools():
    try:
        os.chdir(exe_path)
        a = []
        a.append("." + os.path.sep + exe_name)
        a.append("--listtools")

        ps = subprocess.Popen(a, shell=False, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, bufsize=1, universal_newlines=True)
        ret = ""
        while True:
            line = ps.stdout.readline()
            if line != '':
                ret += line
            else:
                break

        return ret
    except Exception, e:
        return e
