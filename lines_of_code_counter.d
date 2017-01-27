import std.stdio;
import std.conv;
import std.string;
import std.file;
import std.path;
import std.algorithm;
import std.algorithm: canFind;
import std.array;
import std.math;

void main() {

  const auto dataDirectory = "/Users/johnlindsay/Documents/Programming/rust/whitebox_tools/";
  auto ext = "*.{rs}";
  // const auto dataDirectory = "/Users/johnlindsay/Documents/Programming/DCode/ElevationSlice/";
  // auto ext = "*.{d}";
  // const auto dataDirectory = "/Users/johnlindsay/Documents/Programming/GoCode/src/github.com/jblindsay/";
  // auto ext = "*.{go}";
  // const auto dataDirectory = "/Users/johnlindsay/Documents/Programming/Whitebox/trunk/";
  // auto ext = "*.{java,groovy,py,js}";
  // auto ext = "*.{groovy,py,js}";

  auto files = dirEntries(dataDirectory, ext, SpanMode.depth)
                  .filter!(a => a.isFile)
                  .map!(a => a).array;

  int overall_count = 0;
  foreach(file; files) {
    if (file.indexOf("source_files/") == -1 &&
    file.indexOf("dist/") == -1 &&
    file.indexOf("build/") == -1) {
      auto stream = File(file,"r+");
      int file_count = 0;
      foreach(line; stream.byLine()) {
        if (line.strip().indexOf("//") != 0 &&
            line.strip().length > 0 &&
            line.strip() != "{" &&
            line.strip() != "}") {
          file_count++;
          overall_count++;
        }
      }
      writefln("File: %s (%s lines)", file.replace(dataDirectory, ""), file_count);
    }
  }

  writeln("\nThere are ", files.length, " files in the directory/subdirectories.");
  writefln("The total number of SLOC is %d", overall_count);

}
