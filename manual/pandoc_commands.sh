#!/usr/bin/env bash

BASEDIR=$(dirname "$0")
cd "$BASEDIR"
pandoc WhiteboxToolsManual.md --pdf-engine=xelatex -o ../WhiteboxToolsManual.pdf