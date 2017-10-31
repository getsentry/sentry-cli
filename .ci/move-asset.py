#!/usr/bin/env python

import os
import sys


TARGET = os.environ.get('TARGET')
EXT = '.exe' if sys.platform.startswith('win') else ''
DIST_DIR = 'dist'


def get_executable_name():
    bits = TARGET.split('-')
    platform = bits[2].title()
    arch = bits[0]
    return 'sentry-cli-%s-%s%s' % (platform, arch, EXT)


def main():
    executable = os.path.join('target', TARGET, 'release', 'sentry-cli' + EXT)
    if not os.path.isfile(executable):
        print >> sys.stderr, 'Could not locate executable.  Doing nothing.'

    dist = os.path.join(DIST_DIR, get_executable_name())
    if not os.path.exists(DIST_DIR):
        os.makedirs(DIST_DIR)

    os.rename(executable, dist)


if __name__ == '__main__':
    main()
