import platform, subprocess
import os, sys
from shutil import copyfile, copytree, rmtree

# To use this script:
#
# python3 build.py do_clean
#
# Where 'do_clean' is true or false and determines whether or not to clean existing files first. 
#
# You will need Rust installed before running the script. The output will be contained within a
# folder named 'WBT'.


# Set the directory variables
app_dir = os.path.dirname(os.path.abspath(__file__))
output_dir = os.path.join(app_dir, 'WBT')
output_plugin_dir = os.path.join(app_dir, 'WBT/plugins')
plugins_dir = os.path.join(app_dir, 'whitebox-plugins/src')
target_dir = os.path.join(app_dir, 'target/release')

if len(sys.argv) > 1:
    if "t" in sys.argv[1].lower():
        print("Cleaning old files...")
        result = subprocess.run(['cargo', 'clean'], stdout=subprocess.PIPE)
        if len(result.stdout) > 0:
            print(result.stdout)

if os.path.exists(output_dir):
    rmtree(output_dir)

print("Compiling...")
result = subprocess.run(['cargo', 'build', '--release'], stdout=subprocess.PIPE)
if len(result.stdout) > 0:
    print(result.stdout)

if not os.path.exists(output_plugin_dir):
    os.makedirs(output_plugin_dir)

ext = ''
if platform.system() == 'Windows':
    ext = '.exe'

# Copy the whitebox executable over
exe_file = os.path.join(target_dir, 'whitebox_tools') + ext
dst = os.path.join(output_dir, 'whitebox_tools') + ext
copyfile(exe_file, dst)
if platform.system() != 'Windows':
    result = subprocess.run(['strip', dst], stdout=subprocess.PIPE)
os.system("chmod 755 " + dst) # grant executable permission

# Copy the ancillary files
src = os.path.join(app_dir, 'LICENSE.txt')
dst = os.path.join(output_dir, 'LICENSE.txt')
copyfile(src, dst)

src = os.path.join(app_dir, 'readme.txt')
dst = os.path.join(output_dir, 'readme.txt')
copyfile(src, dst)

src = os.path.join(app_dir, 'settings.json')
dst = os.path.join(output_dir, 'settings.json')
copyfile(src, dst)

src = os.path.join(app_dir, 'UserManual.txt')
dst = os.path.join(output_dir, 'UserManual.txt')
copyfile(src, dst)

# Copy the Runner app
exe_file = os.path.join(target_dir, 'whitebox_runner') + ext
dst = os.path.join(output_dir, 'whitebox_runner') + ext
copyfile(exe_file, dst)
if platform.system() != 'Windows':
    result = subprocess.run(['strip', dst], stdout=subprocess.PIPE)
os.system("chmod 755 " + dst) # grant executable 

src = os.path.join(app_dir, 'whitebox_tools.py')
dst = os.path.join(output_dir, 'whitebox_tools.py')
copyfile(src, dst)
os.system("chmod 755 " + dst) # grant executable permission

src = os.path.join(app_dir, 'img')
dst = os.path.join(output_dir, 'img')
copytree(src, dst)

plugins = os.listdir(plugins_dir)
for plugin in plugins:
    if ".DS" not in plugin:
        print(f'Copying plugin: {plugin}')

        # Copy the json file into the plugins directory
        json_file = os.path.join(plugins_dir, plugin, plugin) + '.json'
        dst = os.path.join(output_plugin_dir, plugin) + '.json'
        copyfile(json_file, dst)

        # Copy the executable file into the plugins directory
        exe_file = os.path.join(target_dir, plugin) + ext
        dst = os.path.join(output_plugin_dir, plugin) + ext
        copyfile(exe_file, dst)
        if platform.system() != 'Windows':
            print("Stripping", plugin)
            result = subprocess.run(['strip', dst], stdout=subprocess.PIPE)
            # print(result)

        os.system("chmod 755 " + dst) # grant executable permission

# Copy the register_license binary into the plugins folder if it is available
os.chdir(app_dir)
if os.path.exists('../GeneralToolsetExtension'):
    # Copy the executable file into the plugins directory
    exe_file = f"../GeneralToolsetExtension/register_license{ext}"
    if os.path.exists(exe_file):
        dst = os.path.join(output_plugin_dir, 'register_license') + ext
        copyfile(exe_file, dst)
        os.system("chmod 755 " + dst) # grant executable permission
    else:
        print("No register_license file found...")
else:
    print("No directory containing the register_license file found...")

print("Done!")