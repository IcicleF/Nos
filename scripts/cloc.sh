#!/bin/bash

SCRIPT_DIR=$(dirname $(readlink -f "$0"))
cloc $SCRIPT_DIR/.. --exclude-dir=isal-sys,data,config,target