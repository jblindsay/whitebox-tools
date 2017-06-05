# whitebox-tools

This is a library, developed using the Rust programming language, for analyzing geospatial data. Although it intended to
serve as a source of plugin tools for the *Whitebox GAT* open-source GIS project, the tools contained in this library are
stand-alone and can be run outside of the larger Whitebox GAT project.

## Installation

Fork the GitHub repository then run the build.py Python script. The whitebox-tools.exe executable file will be located within
the /target/release/ folder. 


## Usage
For examples of how to call functions and run tools from *whitebox-tools*, see the *whitebox_example.py* Python script, which itself uses the *whitebox_tools.py* script as an interface for interacting with the executable file. The *whitebox_tools.py* script calls
the executable using subprocesses rather than as a dynamic library. Future versions may compile the library as a dynamic shared object
if this is preferred.

## Contributing

1. Fork the larger Whitebox project (in which whitebox-tools exists) ( https://github.com/jblindsay/whitebox-geospatial-analysis-tools )
2. Create your feature branch (git checkout -b my-new-feature)
3. Commit your changes (git commit -am 'Add some feature')
4. Push to the branch (git push origin my-new-feature)
5. Create a new Pull Request

## Contributors

- [jblindsay](https://github.com/jblindsay) John Lindsay - creator, maintainer

## License
MIT
