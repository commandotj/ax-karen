#!/bin/sh
# Pass the compiled monitor-disk unit test binary as the first argument.
# All mounts and writes live in an isolated, temporary /run.
set -eu
exec unshare --user --map-root-user --mount sh -eu -c '
    mount --make-rprivate /
    mount -t tmpfs Karen-test-run /run
    mkdir -p /run/media/Karen-test /run/Karen-scan
    mount -t tmpfs tmpfs /run/media/Karen-test
    mkdir /run/media/Karen-test/nested
    mount -t tmpfs tmpfs /run/media/Karen-test/nested
    for dir in original alias one two; do mkdir /run/Karen-scan/$dir; done
    mkdir /run/Karen-scan/original/nested
    mount --bind /run/Karen-scan/original /run/Karen-scan/alias
    for dir in one two; do mount -t tmpfs tmpfs /run/Karen-scan/$dir; done
    for dir in original one two; do printf data > /run/Karen-scan/$dir/file; done
    "$1" --exact removal::tests::mounted_media_and_repeated_views --ignored --nocapture
' sh "$1"
