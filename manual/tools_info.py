"""
This script is just used to automatically generate the documentation for each
of the plugin tools in the WhiteboxTools User Manual. It should be run each time new
tools are added to WhiteboxTools.exe and before a public release.
"""
from __future__ import print_function
import os
import re
import json
import sys
sys.path.append(
    '/Users/johnlindsay/Documents/programming/Whitebox/trunk/whitebox_tools/')
from whitebox_tools import WhiteboxTools

_underscorer1 = re.compile(r'(.)([A-Z][a-z]+)')
_underscorer2 = re.compile('([a-z0-9])([A-Z])')


def camel_to_snake(s):
    subbed = _underscorer1.sub(r'\1_\2', s)
    return _underscorer2.sub(r'\1_\2', subbed).lower()


wbt = WhiteboxTools()

# Set the directory containing the whitebox_tools.exe file
wbt.exe_path = r'/Users/johnlindsay/Documents/programming/Whitebox/trunk/whitebox_tools/target/release/'
# wbt.ext_path = r'../target/release/'

toolboxes = wbt.toolbox('')
tb_set = set()
for tb in toolboxes.split('\n'):
    if tb.strip():
        tb_set.add(tb.strip().split(':')[1].strip())

tb_dict = {}
for tb in sorted(tb_set):
    tb_dict[tb] = []

tools = wbt.list_tools()
for t in tools.split("\n"):
    if t.strip() and "Available Tools" not in t:
        tool = t.strip().split(":")[0]
        description = t.strip().split(":")[1].strip().rstrip('.')
        toolbox = wbt.toolbox(tool).strip()

        tool_help = wbt.tool_help(tool)
        flag = False
        example = ''
        for v in tool_help.split('\n'):
            if flag:
                example += v + "\n"
            if "Example usage:" in v:
                flag = True

        if len(example) > 65:
            a = example.split('-')
            example = ''
            count = 0
            b = 0
            for v in a:
                if v.strip():
                    if count + len(v) < 65:
                        if not v.startswith('>>'):
                            example += "-{} ".format(v.strip())
                            count += len(v) + 2
                        else:
                            example += "{} ".format(v.strip())
                            count = len(v) + 1

                    else:
                        example += "^\n-{} ".format(v.strip())
                        count = len(v) + 1
                else:
                    a[b + 1] = "-" + a[b + 1]

                b += 1

        doc_str = ""
        parameters = wbt.tool_parameters(tool)
        j = json.loads(parameters)
        param_num = 0
        default_params = []
        for p in j['parameters']:
            st = r"{}"
            st_val = '        '
            if param_num == 0:
                st_val = ''
            param_num += 1

            json_str = json.dumps(
                p, sort_keys=True, indent=2, separators=(',', ': '))

            flag_str = ""
            for v in p['flags']:
                flag_str += "{}, ".format(v.replace('--', '-\-'))
            flag_str = flag_str.rstrip(', ')
            desc = p['description'].strip().rstrip('.')
            if len(desc) > 80:
                a = desc.split(' ')
                desc = ''
                count = 0
                for v in a:
                    if count + len(v) < 80:
                        desc += "{} ".format(v)
                        count += len(v) + 1
                    else:
                        desc += "\n{}{} ".format(' ' * 21, v)
                        count = len(v) + 1

            doc_str += "{}{}{}\n".format(flag_str, ' ' * (21 - len(flag_str)),
                                         desc)

        fn = """
#### insertNumHere {}

*Description*: 
{}

*Toolbox*: {}

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
{}
*Example Usage*:
```
{}
```""".format(tool, description, toolbox, doc_str, example)
        # print(fn)
        tb_dict[toolbox].append(fn)

f = open("/Users/johnlindsay/Documents/deleteme2.txt", 'w')
num1 = 1
num2 = 1
for key, value in sorted(tb_dict.items()):
    f.write("### 6.{} {}\n".format(num1, key.replace("/", " => ")))
    # print("* 6.{} [{}](#{})".format(num1, key.replace("/", " = "),
    #                                 key.replace("/", " = ").lower().replace(" ", "-")))
    num2 = 1
    for v in value:
        # print(v)
        f.write("{}\n".format(
            v.replace("insertNumHere", "6.{}.{}".format(num1, num2))))
        num2 += 1

    num1 += 1

f.close()
