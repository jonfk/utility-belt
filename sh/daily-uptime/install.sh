#!/bin/sh

INSTALL_PATH_JOB="$HOME/Library/LaunchAgents/daily-uptime-osx.plist"
INSTALL_PATH_SCRIPT="$HOME/Applications/daily-uptime-osx.sh"

cp daily-uptime-osx.plist "$INSTALL_PATH_JOB"
cp daily-uptime-osx.sh  "$INSTALL_PATH_SCRIPT"
