#!/usr/bin/python
import os
import sys
import json
import shutil
import urllib
import tempfile
import subprocess

if not sys.stdin.isatty():
    sys.stdin = open('/dev/tty', 'r')


INSTALL_PATH = '/usr/local/bin/sentry-cli'
META_URL = 'https://api.github.com/repos/getsentry/sentry-cli/releases/latest'


def fail(message, *args):
    if args:
        message = message % args
    print >> sys.stderr, message
    sys.exit(1)


def prompt(question):
    while 1:
        answer = raw_input(question + ' [y/n] ').lower()
        if answer in ('y', 'yes'):
            return True
        elif answer in ('n', 'no'):
            return False
        print '  invalid input: write yes or no'


def get_asset_name():
    p = subprocess.Popen(['uname', '-sm'], stdout=subprocess.PIPE)
    platform, arch = p.communicate()[0].strip().split(' ', 1)
    return 'sentry-cli-%s-%s' % (platform, arch)


def find_latest_release():
    asset_name = get_asset_name()
    data = json.load(urllib.urlopen(META_URL))
    for asset in data['assets']:
        if asset['name'] == asset_name:
            return asset['browser_download_url'], data['tag_name']


def main():
    if os.getuid() == 0:
        fail('This script will sudo itself if needed. Do not run it as root!')
    if os.path.isfile(INSTALL_PATH):
        fail('sentry-cli is already installed. Use "sentry-cli update" to '
             'perform an update.')

    print 'This script will install sentry-cli to %s' % INSTALL_PATH

    rv = find_latest_release()
    if rv is None:
        fail('Could not find release compatible with this machine.')
    url, version = rv

    if not prompt('Do you want to install sentry-cli %s?' % version):
        print 'Cancelled.'
        sys.exit(0)

    print 'Downloading ...'
    tmp = os.path.join(tempfile.gettempdir(), '.sentry-' +
                       os.urandom(20).encode('hex'))
    try:
        with open(tmp, 'wb') as df:
            sf = urllib.urlopen(url)
            shutil.copyfileobj(sf, df)
            os.chmod(df.name, 0755)
        print 'Downloaded sentry-cli'
        try:
            shutil.move(tmp, INSTALL_PATH)
        except (IOError, OSError):
            print 'Sudoing for installation.'
            os.system('sudo -k mv "%s" "%s"' % (tmp, INSTALL_PATH))
    finally:
        try:
            os.unlink(tmp)
        except OSError:
            pass
    print 'sentry-cli is ready to use.'


if __name__ == '__main__':
    main()
