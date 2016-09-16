#!/usr/bin/env sh

set -e
set -x

mkdir -p "$GOPATH/src/github.com/jonfk"
ln -s `pwd` "$GOPATH/src/github.com/jonfk/utility-belt"
glide install
