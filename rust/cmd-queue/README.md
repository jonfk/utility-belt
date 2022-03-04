# cmd-queue

`cmdq` allows the queueing of commands with configurable concurrency level and
retries on failure.

The `cmdq` cli starts a daemon if there isn't one started that will handle the
running of the commands.

## features

[x] Retries with exponential backoff and max wait
[x] Commands run in the directory the `cmdq` command was invoked in
[x] Shell completion through clap_complete (see #issues)
[ ] listing commands
[ ] Persistent queue. Commands should not be lost in case of a crash/power outage
    - Queue order will not be guaranteed
    - Queued commands could potentially be run twice if for example the crash occurred right after it completed but before it persisted it's completion
[ ] Improve logging and tracing output
[ ] Progress reporting of running processes
[ ] Queryable output of running/failed processes
[ ] WebUI
[ ] Enable configurable concurrency level
    - Currently hardcoded number of worker threads
[ ] Intelligent workpool, better running efficiency
    - If there are no commands in queue shrink worker pool
    - Size worker pool based on workload
[ ] Auto-shutdown?
[ ] Enable configurable server port
[ ] Improve server startup wait in client
    - Currently sleeps for set time
    - Instead query health endpoint until server is up with exponential backoff and max wait time

## Issues

[ ] Completions are currently broken. Keep an eye on https://github.com/clap-rs/clap/issues/3022

## potential features

These are features would be nice to have but currently for which there isn't a 
pressing need.

- Multiple command groups for which max concurrency is set for each group
