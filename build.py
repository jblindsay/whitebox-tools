import platform, subprocess
from shutil import copyfile

result = subprocess.run(['cargo', 'build', "--release"], stdout=subprocess.PIPE)
print(result.stdout)

ext = ""
if platform.system() == 'Windows':
    ext = '.exe'

exe_name = f"whitebox_tools{ext}"

src = f"./target/release/{exe_name}"
dst = f"./WBT/{exe_name}"
copyfile(src, dst)

# result = subprocess.run(["pwd"], stdout=subprocess.PIPE)
# print("pwd: ", result.stdout)

# result = subprocess.run(["codesign -h"], stdout=subprocess.PIPE)
# print("", result.stdout)

# args = []
# args.append("./WBT/whitebox_tools")
# args.append("-run='recreate_pass_lines'")
# args.append("--inputFile='/Users/johnlindsay/Documents/data/yield/Woodrill_UTM.shp' --yieldFieldName='Yld_Mass_D' --outputFile='/Users/johnlindsay/Documents/data/yield/pass_lines.shp' --outputPointsFile='/Users/johnlindsay/Documents/data/yield/points.shp' --maxChangeInHeading=25.0")
# result = subprocess.run(args, stdout=subprocess.PIPE)
# print(result.stdout)

print("Done!")