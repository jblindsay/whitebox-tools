import platform, subprocess
import os, sys
from shutil import copyfile, copytree, make_archive, rmtree

# To use this script:
#
# python3 build.py [args]
#
# Script Keyword Arguments:
#
# do_clean           If present, the existing files will be cleaned before compiling.
# exclude_runner     Excludes the WhiteboxTools Runner app from the build. 
# zip                Creates a zip file output in addition to the WBT folder
#
# Notes:
# You will need Rust installed before running the script. The output will be contained within a
# folder named 'WBT'. The WhiteboxTools Runner app often results in an error when compiling on
# Linux. This seems to be related to openssl libraries, which need to be set up correctly. If 
# you are unable to figure out the set-up correctly and you do not need the Runner app, you
# would be advised to use the exclude_runner argument on linux.
#
# Example:
# python3 build.py do_clean exclude_runner zip


def build(do_clean=False, exclude_runner=False, create_zip_artifact=False):
    # Set the directory variables
    app_dir = os.path.dirname(os.path.abspath(__file__))
    output_dir = os.path.join(app_dir, 'WBT')
    output_plugin_dir = os.path.join(app_dir, 'WBT/plugins')
    plugins_dir = os.path.join(app_dir, 'whitebox-plugins/src')

    target_dir = os.path.join(app_dir, 'target/release')
    if platform.system() == "Linux":
        target_dir = os.path.join(app_dir, 'target/x86_64-unknown-linux-musl/release')

    if do_clean:
        print("Cleaning old files...")
        result = subprocess.run(['cargo', 'clean'], stdout=subprocess.PIPE)
        if len(result.stdout) > 0:
            print(result.stdout)

    if os.path.exists(output_dir):
        rmtree(output_dir)


    # Create the Cargo.toml file
    workspace_str = '[workspace]\nmembers = ["whitebox-common", "whitebox-lidar", "whitebox-plugins", "whitebox-raster", "whitebox-runner", "whitebox-tools-app", "whitebox-vector"]\n\n'
    if exclude_runner:
        # Exclude the runner if the second command line arg is set to True or if the platform is linux
        workspace_str = '[workspace]\nmembers = ["whitebox-common", "whitebox-lidar", "whitebox-plugins", "whitebox-raster", "whitebox-tools-app", "whitebox-vector"]\n\n'

    with open('Cargo.toml', "w") as cargo_file:
        cargo_file.write(workspace_str)
        
        cargo_file.write("""[profile.release]
    incremental = true
    strip = true""")


    print("Compiling...")
    # result = subprocess.run(['cargo', 'build', "--release"], stdout=subprocess.PIPE)
    # if len(result.stdout) > 0:
    #     print(result.stdout)
    if platform.system() != 'Linux':
        result = subprocess.run(['cargo', 'build', "--release"], stdout=subprocess.PIPE)
        if len(result.stdout) > 0:
            print(result.stdout)
    else:
        print("Compiling for musl target...")
        result = subprocess.run(['rustup', 'target', "add", "x86_64-unknown-linux-musl"], stdout=subprocess.PIPE)
        if len(result.stdout) > 0:
            print(result.stdout)
            
        result = subprocess.run(['cargo', 'build', "--release", "--target=x86_64-unknown-linux-musl"], stdout=subprocess.PIPE)
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

    # If the Runner app exists, copy it
    exe_file = os.path.join(target_dir, 'whitebox_runner') + ext
    if os.path.exists(exe_file):
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
        if ".DS" not in plugin and "._" not in plugin:
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


    if create_zip_artifact:
        # Make a zip of the WBT folder
        print("Creating zip artifact...")
        proc = "amd64"
        if "arm" in platform.processor().lower() and "darwin" in platform.system().lower():
            proc = "m_series"

        pltfm = platform.system().lower()
        if "windows" in pltfm:
            pltfm = "win"

        zip_name = f"WhiteboxTools_{pltfm}_{proc}"
        # output_zip = os.path.join(app_dir, zip_name, "WBT")
        copytree(output_dir, os.path.join(app_dir, zip_name, "WBT"), dirs_exist_ok=True)
        # output_zip = os.path.join(app_dir, zip_name)
        
        with open(os.path.join(os.path.join(app_dir, zip_name), 'readme.txt'), "w") as readme_file:
            readme_file.write("""Instructions:

Copy the WBT folder and its entire contents to any location on your system. Configure your Whitebox
frontend, whether that is the QGIS or ArcGIS plugin, or the Python package, to point to this WBT
folder location. To access the functionality of WhiteboxTools without the need for a 3rd party
frontend, launch the WhiteboxTools Runner app (whitebox_runner), if it is contained within the WBT 
folder.""")

        # output_zip = os.path.join(app_dir, 'zip_file', zip_name)
        make_archive(os.path.join(app_dir, 'zip_file', zip_name), 'zip', os.path.join(app_dir, zip_name))

        # Delete the folder
        if os.path.exists(os.path.join(app_dir, zip_name)):
            rmtree(os.path.join(app_dir, zip_name))

    print("Done!")

def main():
    # Read in the script arguments
    do_clean = False
    if any("do_clean" in s for s in sys.argv):
        matching = [s for s in sys.argv if "do_clean" in s]
        if len(matching) > 0:
            if "false" not in matching[0].lower():
                do_clean = True

    exclude_runner = False
    if any("exclude_runner" in s for s in sys.argv):
        matching = [s for s in sys.argv if "exclude_runner" in s]
        if len(matching) > 0:
            if "false" not in matching[0].lower():
                exclude_runner = True

    create_zip_artifact = False
    if any("zip" in s for s in sys.argv):
        matching = [s for s in sys.argv if "zip" in s]
        if len(matching) > 0:
            if "false" not in matching[0].lower():
                create_zip_artifact = True
    
    build(do_clean, exclude_runner, create_zip_artifact)

if __name__ == "__main__":
    main()