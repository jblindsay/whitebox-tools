#!/usr/bin/env bash

BASEDIR=$(dirname "$0")
cd "$BASEDIR"
# #!/bin/bash
# DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
# if [ "$DIR" != pwd ]; then
#   cd $DIR
# fi

pandoc \
--pdf-engine=xelatex \
--variable classoption=twoside \
--variable papersize=letterpaper \
-s WhiteboxToolsManual.md \
-o ../WhiteboxToolsManual.pdf