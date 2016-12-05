#!/bin/sh

#
# Reference:
# - http://launchd.info/
# - http://stackoverflow.com/questions/16542301/running-a-shell-script-on-shutdown-via-launchd
#
# To Do to run as work:
# - cp daily-uptime-osx.sh ~/Applications
# - Edit the path in daily-uptime-osx.plist to path of script
# - cp daily-uptime-osx.plist ~/Library/LaunchAgents
# - logout and login
#

DAILYUPTIMEFILE="$HOME/daily-uptime.log"


StartService() {
    echo "starting logging your uptime"
    printf '"%s",' `date +%Y-%m-%dT%H:%M:%S%z` >> "$DAILYUPTIMEFILE"
}

StopService() {
    echo "Shutting Down. Logging your uptime :)"
    touch "$DAILYUPTIMEFILE"
    printf '"%s","%s","%s"\n' "`date +%Y-%m-%dT%H:%M:%S%z`" "`uptime`" "`sysctl -n kern.boottime`" >> "$DAILYUPTIMEFILE"
    exit 0
}

trap StopService SIGTERM
while true; do
    sleep 86400 &
    wait
done
