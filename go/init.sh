#!/usr/bin/env sh

set -e
#set -x

INGOPATH="$GOPATH/src/github.com/jonfk/utility-belt"

mkdir -p "$GOPATH/src/github.com/jonfk"
if [ ! -d "$INGOPATH" ]; then
    ln -s `pwd` "$INGOPATH"
fi
glide install

echo ""
echo "# Go to $GOPATH to build programs"
echo "cd $INGOPATH"
echo ""
