#!/bin/sh

ssh -vvv -N -D 127.0.0.1:2189 \
  -o ExitOnForwardFailure=yes \
  -o ServerAliveInterval=30 -o ServerAliveCountMax=2 \
  jonfk@um700dev.jonfk.internal
