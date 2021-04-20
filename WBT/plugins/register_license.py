import json, os, platform, sys
from os import path
from pathlib import Path
from sys import platform as _platform
import tkinter as tk
from tkinter import ttk
from tkinter.scrolledtext import ScrolledText
from tkinter import filedialog
from tkinter import messagebox
from tkinter import PhotoImage
from subprocess import CalledProcessError, Popen, PIPE, STDOUT

class DataInput(tk.Frame):
    def __init__(self, json_str, master=None, tooltip_label=None):
        self.tooltip_label = tooltip_label

        # first make sure that the json data has the correct fields
        j = json.loads(json_str)
        self.name = j['name']
        self.description = j['description']
        self.flag = j['flags'][len(j['flags']) - 1]
        self.parameter_type = j['parameter_type']
        self.optional = j['optional']
        default_value = j['default_value']

        ttk.Frame.__init__(self, master)
        self.grid()
        self['padding'] = '0.1i'

        self.bind("<Enter>", self.onEnter)
        self.bind("<Leave>", self.onLeave)

        self.label = ttk.Label(self, text=self.name, justify=tk.LEFT)
        self.label.grid(row=0, column=0, sticky=tk.W)
        self.label.columnconfigure(0, weight=1)

        self.value = tk.StringVar()
        if default_value:
            self.value.set(default_value)
        else:
            self.value.set("")

        self.entry = ttk.Entry(self, width=45, justify=tk.LEFT, textvariable=self.value)
        self.entry.grid(row=0, column=1, sticky=tk.NSEW)
        self.entry.columnconfigure(1, weight=10)

        if not self.optional:
            self.label['text'] = self.label['text'] + "*"

        if ("Integer" in self.parameter_type or
            "Float" in self.parameter_type or
                "Double" in self.parameter_type):
            self.entry['justify'] = 'right'

        # Add the bindings
        if _platform == "darwin":
            self.entry.bind("<Command-Key-a>", self.select_all)
        else:
            self.entry.bind("<Control-Key-a>", self.select_all)

        # self.pack(fill=tk.BOTH, expand=1)
        self.columnconfigure(0, weight=1)
        self.columnconfigure(1, weight=10)
        self.rowconfigure(0, weight=1)

    def onEnter(self, event=None):
        self.tooltip_label.configure(text=self.description)
        # self.update()  # this is needed for cancelling and updating the progress bar

    def onLeave(self, event=None):
        self.tooltip_label.configure(text="")
        self.update()  # this is needed for cancelling and updating the progress bar

    def RepresentsInt(self, s):
        try:
            int(s)
            return True
        except ValueError:
            return False

    def RepresentsFloat(self, s):
        try:
            float(s)
            return True
        except ValueError:
            return False

    def get_value(self):
        v = self.value.get()
        if v:
            if "Integer" in self.parameter_type:
                if self.RepresentsInt(self.value.get()):
                    return "{}={}".format(self.flag, self.value.get())
                else:
                    messagebox.showinfo(
                        "Error", "Error converting parameter {} to type Integer.".format(self.flag))
            elif "Float" in self.parameter_type:
                if self.RepresentsFloat(self.value.get()):
                    return "{}={}".format(self.flag, self.value.get())
                else:
                    messagebox.showinfo(
                        "Error", "Error converting parameter {} to type Float.".format(self.flag))
            elif "Double" in self.parameter_type:
                if self.RepresentsFloat(self.value.get()):
                    return "{}={}".format(self.flag, self.value.get())
                else:
                    messagebox.showinfo(
                        "Error", "Error converting parameter {} to type Double.".format(self.flag))
            else:  # String or StringOrNumber types
                return "{}='{}'".format(self.flag, self.value.get())
        else:
            if not self.optional:
                messagebox.showinfo(
                    "Error", "Unspecified non-optional parameter {}.".format(self.flag))

        return None

    def select_all(self, event):
        self.entry.select_range(0, tk.END)
        return 'break'


class Gui(tk.Frame):
    def __init__(self, tool_name=None, master=None):
        if platform.system() == 'Windows':
            self.ext = '.exe'
        else:
            self.ext = ''
        self.exe_name = "./register_license{}".format(self.ext)
        # self.exe_path = os.path.dirname(shutil.which(
        #     self.exe_name) or path.dirname(path.abspath(__file__)))
        # self.exe_path = os.path.dirname(os.path.join(os.path.realpath(__file__)))
        self.exe_path = path.dirname(path.abspath(__file__))

        self.cancel_op = False

        ttk.Frame.__init__(self, master)
        self.script_dir = os.path.dirname(os.path.realpath(__file__))
        self.grid()
        self.tool_name = tool_name
        self.master.title("Register License")
        # if _platform == "darwin":
        #     os.system(
        #         '''/usr/bin/osascript -e 'tell app "Finder" to set frontmost of process "Python" to true' ''')
        
        #########################################################
        #              Overall/Top level Frame                  #
        #########################################################     
        #define left-side frame (toplevel_frame) and right-side frame (overall_frame)
        # toplevel_frame = ttk.Frame(self, padding='0.1i')
        overall_frame = ttk.Frame(self, padding='0.1i')
        #set-up layout
        overall_frame.grid(row=0, column=0, sticky=tk.NSEW)
        # toplevel_frame.grid(row=0, column=0, sticky=tk.NSEW) 

        ##################
        # Tool tip label #
        ##################
        tooltip_frame = ttk.Frame(overall_frame, padding='0.1i')
        self.tt_label = ttk.Label(tooltip_frame, text="")
        style = ttk.Style()
        style.configure("Blue.Label", foreground="dark blue")
        self.tt_label.configure(style="Blue.Label")
        self.tt_label.grid(row=0, column=0, sticky=tk.W)
        tooltip_frame.grid(row=4, column=0, columnspan=2, sticky=tk.NSEW)

        
        # Add GUI elements
        self.elements_frame = ttk.Frame(overall_frame, padding='0.1i')

        param_num = 0

        param_str = '{ "name":"E-mail address of licensee", "description": "The e-mail address of the person to whom the license activation key was issued.", "flags": ["--email"], "parameter_type": "String", "optional": "False", "default_value": null}'
        di1 = DataInput(param_str, self.elements_frame, self.tt_label)
        di1.grid(row=param_num, column=0, sticky=tk.NSEW)
        param_num += 1 

        param_str = '{ "name":"Seat number", "description": "The seat number of this installation. This must be <= the seats in the license.", "flags": ["--seat"], "parameter_type": "Integer", "optional": "False", "default_value": "1"}'
        di2 = DataInput(param_str, self.elements_frame, self.tt_label)
        di2.grid(row=param_num, column=0, sticky=tk.NSEW)
        param_num += 1 
        
        param_str = '{ "name":"License activation key", "description": "The license activation key. This will be a long hex-string.", "flags": ["--key"], "parameter_type": "String", "optional": "False", "default_value": null}'
        di2 = DataInput(param_str, self.elements_frame, self.tt_label)
        di2.grid(row=param_num, column=0, sticky=tk.NSEW)
        param_num += 1        

        self.elements_frame.grid(row=0, column=0, sticky=tk.NSEW)


        #########################################################
        #                   Buttons Frame                       #
        #########################################################
        #Create the elements of the buttons frame
        buttons_frame = ttk.Frame(overall_frame, padding='0.1i')
        self.run_button = ttk.Button(buttons_frame, text="Register", width=8, command=self.run_tool)
        # self.quit_button = ttk.Button(buttons_frame, text="Cancel", width=8, command=self.cancel_operation)
        self.close_button = ttk.Button(buttons_frame, text="Close", width=8, command=self.quit)
        #Define layout of the frame
        self.run_button.grid(row=0, column=0)
        # self.quit_button.grid(row=0, column=1)
        self.close_button.grid(row=0, column=2)
        buttons_frame.grid(row=1, column=0, columnspan=2, sticky=tk.E)

        #########################################################
        #                  Output Frame                         #
        #########################################################                
        #Create the elements of the output frame
        output_frame = ttk.Frame(overall_frame)
        outlabel = ttk.Label(output_frame, text="Output:", justify=tk.LEFT)
        self.out_text = ScrolledText(output_frame, width=79, height=8, wrap=tk.NONE, padx=7, pady=7, exportselection = 0)
        output_scrollbar = ttk.Scrollbar(output_frame, orient=tk.HORIZONTAL, command = self.out_text.xview)
        self.out_text['xscrollcommand'] = output_scrollbar.set
        #Retreive and insert the text for the current tool
        # k = wbt.tool_help(self.tool_name)   
        # self.out_text.insert(tk.END, k)
        #Define layout of the frame
        outlabel.grid(row=0, column=0, sticky=tk.NW)
        self.out_text.grid(row=1, column=0, sticky=tk.NSEW)
        output_frame.grid(row=2, column=0, columnspan = 2, sticky=(tk.NS, tk.E))
        output_scrollbar.grid(row=2, column=0, sticky=(tk.W, tk.E))
        #Configure rows and columns of the frame
        self.out_text.columnconfigure(0, weight=1)
        output_frame.columnconfigure(0, weight=1)
        # Add the binding
        if _platform == "darwin":
            self.out_text.bind("<Command-Key-a>", self.select_all)
        else:
            self.out_text.bind("<Control-Key-a>", self.select_all)
            
        #########################################################
        #                  Progress Frame                       #
        #########################################################        
        #Create the elements of the progress frame
        progress_frame = ttk.Frame(overall_frame, padding='0.1i')
        self.progress_label = ttk.Label(progress_frame, text="Progress:", justify=tk.LEFT)
        self.progress_var = tk.DoubleVar()
        self.progress = ttk.Progressbar(progress_frame, orient="horizontal", variable=self.progress_var, length=200, maximum=100)
        #Define layout of the frame
        self.progress_label.grid(row=0, column=0, sticky=tk.E, padx=5)
        self.progress.grid(row=0, column=1, sticky=tk.E)
        progress_frame.grid(row=3, column=0, columnspan = 2, sticky=tk.SE)

        self.working_dir = str(Path.home())

    def run_tool(self):
        try:
            args = []
            for widget in self.elements_frame.winfo_children():
                v = widget.get_value()
                if v:
                    args.append(v)
                elif not widget.optional:
                    messagebox.showinfo(
                        "Error", "Non-optional tool parameter not specified.")
                    return

            # print(args)

            ''' 
            Runs a tool and specifies tool arguments.
            Returns 0 if completes without error.
            Returns 1 if error encountered (details are sent to callback).
            Returns 2 if process is cancelled by user.
            '''

            # print(self.exe_name)

            os.chdir(self.exe_path)
            args2 = []
            args2.append(self.exe_name)
            args2.append("register")

            for arg in args:
                a = arg.split("=")
                args2.append(a[1].replace("\'", ""))

            # print(args2)

            cl = ""
            for v in args2:
                cl += v + " "
            self.custom_callback(cl.strip() + "\n")

            proc = Popen(args2, shell=False, stdout=PIPE, stderr=STDOUT, bufsize=1, universal_newlines=True)

            while True:
                line = proc.stdout.readline()
                sys.stdout.flush()
                if line != '':
                    if not self.cancel_op:
                        self.custom_callback(line.strip())
                    else:
                        self.cancel_op = False
                        proc.terminate()
                        return 2

                else:
                    break

            return 0
        except (OSError, ValueError, CalledProcessError) as err:
            self.custom_callback(str(err))
            return 1
        finally:
            self.progress_var.set(0)
            self.progress_label['text'] = "Progress:"

    def custom_callback(self, value):
        ''' A custom callback for dealing with tool output.
        '''
        if "%" in value:
            try:
                str_array = value.split(" ")
                label = value.replace(
                    str_array[len(str_array) - 1], "").strip()
                progress = float(
                    str_array[len(str_array) - 1].replace("%", "").strip())
                self.progress_var.set(int(progress))
                self.progress_label['text'] = label
            except ValueError as e:
                print("Problem converting parsed data into number: ", e)
            except Exception as e:
                print(e)
        else:
            self.print_line_to_output(value)

        self.update()  # this is needed for cancelling and updating the progress bar

    def print_to_output(self, value):
        self.out_text.insert(tk.END, value)
        self.out_text.see(tk.END)

    def print_line_to_output(self, value):
        self.out_text.insert(tk.END, value + "\n")
        self.out_text.see(tk.END)
        
    # def cancel_operation(self):
    #     # wbt.cancel_op = True
    #     self.print_line_to_output("Cancelling operation...")
    #     self.progress.update_idletasks()

    def select_all(self, event):
        self.out_text.tag_add(tk.SEL, "1.0", tk.END)
        self.out_text.mark_set(tk.INSERT, "1.0")
        self.out_text.see(tk.INSERT)
        return 'break'

def main():
    gui = Gui()
    gui.mainloop()

main()