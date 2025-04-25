#!/bin/bash

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Path to the Image Viewer app
APP_PATH="$SCRIPT_DIR/Image Viewer.app"

# Check if a file was provided as an argument
if [ $# -eq 0 ]; then
    # No file provided, just open the app
    open "$APP_PATH"
else
    # File provided, open it with the app
    open -a "$APP_PATH" "$1"
fi 