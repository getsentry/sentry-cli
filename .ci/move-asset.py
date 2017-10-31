#!/usr/bin/env python

import os
import sys


TARGET = os.environ.get('TARGET')
DIST_DIR = os.environ.get('DIST_DIR', 'dist')
EXT = '.exe' if sys.platform.startswith('win') else ''


def find_executable():
    if TARGET:
        path = os.path.join('target', TARGET, 'release', 'sentry-cli' + EXT)
        if os.path.isfile(path):
            return path

    path = os.path.join('target', 'release', 'sentry-cli' + EXT)
    if os.path.isfile(path):
        return path


def get_executable_name():
    bits = TARGET.split('-')
    platform = bits[2].title()
    arch = bits[0]
    return 'sentry-cli-%s-%s%s' % (platform, arch, EXT)


def main():
    executable = find_executable()
    if executable is None:
        print >> sys.stderr, 'Could not locate executable.  Doing nothing.'
        return

    dist = os.path.join(DIST_DIR, get_executable_name())
    if not os.path.exists(DIST_DIR):
        os.makedirs(DIST_DIR)

    os.rename(executable, dist)
    print 'Asset moved to %s' % dist


if __name__ == '__main__':
    main()
