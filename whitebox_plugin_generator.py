"""
This script is just used to automatically generate the convenience methods for each
of the plugin tools in the whitebox_tools.py script. It should be run each time new
tools are added to WhiteboxTools.exe and before a public release.
"""
from __future__ import print_function
import os
import re
import json
from whitebox_tools import WhiteboxTools

_underscorer1 = re.compile(r'(.)([A-Z][a-z]+)')
_underscorer2 = re.compile('([a-z0-9])([A-Z])')


def camel_to_snake(s):
    subbed = _underscorer1.sub(r'\1_\2', s)
    return _underscorer2.sub(r'\1_\2', subbed).lower()


wbt = WhiteboxTools()

# Set the directory containing the whitebox_tools.exe file
wbt.exe_path = r'/Users/johnlindsay/Documents/programming/Whitebox/trunk/whitebox_tools/target/release/'

tools = wbt.list_tools()
for t in tools.split("\n"):
    if t.strip() and "Available Tools" not in t:
        tool = t.strip().split(":")[0]
        tool_snaked = camel_to_snake(tool)
        if tool_snaked == "and":
            tool_snaked = "And"
        if tool_snaked == "or":
            tool_snaked = "Or"
        if tool_snaked == "not":
            tool_snaked = "Not"
        fn_def = "def {}(self, ".format(tool_snaked)

        description = t.strip().split(":")[1].rstrip('.')

        arg_append_str = ""

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
            flag = p['flags'][len(p['flags']) - 1].replace('-', '')
            if flag == "class":
                flag = "cls"

            doc_str += "{}{} -- {}. \n".format(st_val,
                                               flag, p['description'].rstrip('.'))

            pt = p['parameter_type']
            if 'Boolean' in pt:
                if p['default_value'] != None and p['default_value'] != 'false':
                    default_params.append(
                        "{}=True, ".format(camel_to_snake(flag)))
                else:
                    default_params.append(
                        "{}=False, ".format(camel_to_snake(flag)))

                arg_append_str += "{}if {}: args.append(\"{}\")\n".format(
                    st_val, camel_to_snake(flag), p['flags'][len(p['flags']) - 1])
            else:
                if p['default_value'] != None:
                    if p['default_value'].replace('.', '', 1).isdigit():
                        default_params.append("{}={}, ".format(
                            camel_to_snake(flag), p['default_value']))
                    else:
                        default_params.append("{}=\"{}\", ".format(
                            camel_to_snake(flag), p['default_value']))

                    arg_append_str += "{}args.append(\"{}={}\".format({}))\n".format(
                        st_val, p['flags'][len(p['flags']) - 1], st, camel_to_snake(flag))
                else:
                    if not p['optional']:
                        fn_def += "{}, ".format(camel_to_snake(flag))
                        arg_append_str += "{}args.append(\"{}='{}'\".format({}))\n".format(
                            st_val, p['flags'][len(p['flags']) - 1], st, camel_to_snake(flag))
                    else:
                        default_params.append(
                            "{}=None, ".format(camel_to_snake(flag)))
                        arg_append_str += "{}if {} is not None: args.append(\"{}='{}'\".format({}))\n".format(
                            st_val, flag, p['flags'][len(p['flags']) - 1], st, camel_to_snake(flag))

                    # arg_append_str += "{}args.append(\"{}='{}'\".format({}))\n".format(
                    #     st_val, p['flags'][len(p['flags']) - 1], st, camel_to_snake(flag))

        for d in default_params:
            fn_def += d

        # fn_def = fn_def.rstrip(', ')
        fn_def += "callback=default_callback):"

        fn = """
    {}
        \"\"\"{}.
        
        Keyword arguments:

        {}
        \"\"\"
        args = []
        {}
        return self.run_tool('{}', args, callback) # returns 1 if error""".format(fn_def, description, doc_str.rstrip(), arg_append_str.rstrip(), tool)
        print(fn)
