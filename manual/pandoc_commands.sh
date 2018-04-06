#!/usr/bin/env bash

BASEDIR=$(dirname "$0")
cd "$BASEDIR"
# #!/bin/bash
# DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
# if [ "$DIR" != pwd ]; then
#   cd $DIR
# fi

# pandoc --print-highlight-style tango # tango haddock zenburn kate breezedark espresso pygments
# pandoc --list-highlight-styles
# pandoc --list-highlight-languages

pandoc \
--pdf-engine=xelatex \
--variable classoption=twoside \
--variable papersize=letterpaper \
--variable urlcolor=blue \
-s WhiteboxToolsManual.md \
--toc \
--toc-depth=4 \
--highlight-style=my_style.theme \
-o ../WhiteboxToolsManual.pdf