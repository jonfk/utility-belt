#!/bin/sh

DELAY=1
MAX_DELAY=60
RESET_THRESHOLD=300  # Reset delay if connection lasts > 5 minutes
INTERRUPTED=0

# Handle Ctrl-C gracefully
trap 'INTERRUPTED=1' INT

while true; do
  START_TIME=$(date +%s)

  echo "$(date): Starting SSH proxy (retry delay: ${DELAY}s)"

  ssh -vvv -N -D 127.0.0.1:2189 \
    -o ExitOnForwardFailure=yes \
    -o ServerAliveInterval=30 -o ServerAliveCountMax=2 \
    jonfk@um700dev.jonfk.internal

  EXIT_CODE=$?
  END_TIME=$(date +%s)
  DURATION=$((END_TIME - START_TIME))

  echo "$(date): SSH proxy stopped with exit code $EXIT_CODE (ran for ${DURATION}s)"

  # Exit if interrupted by Ctrl-C
  if [ $INTERRUPTED -eq 1 ]; then
    echo "Interrupted by user, exiting"
    exit 130
  fi

  # Reset delay if connection lasted long enough
  if [ $DURATION -ge $RESET_THRESHOLD ]; then
    DELAY=1
    echo "Connection was stable, resetting retry delay"
  fi

  echo "Waiting ${DELAY}s before retry..."
  sleep $DELAY

  # Exponential backoff, capped at MAX_DELAY
  DELAY=$((DELAY * 2))
  if [ $DELAY -gt $MAX_DELAY ]; then
    DELAY=$MAX_DELAY
  fi
done
