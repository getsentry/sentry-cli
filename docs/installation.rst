Installation
============

Depending on your platform there are different methods available to
install `sentry-cli`.

Manual Download
---------------

You can find the list of releases on `the github release page
<https://github.com/getsentry/sentry-cli/releases/>`__.  We provide
executables for Linux, OS X and Windows.  It's a single file download and
upon receiving the file you can rename it to just ``sentry-cli`` or
``sentry-cli.exe`` to use it.

Automatic Installation
----------------------

If you are on OS X or Linux you can use the automated downloader which
will fetch the latest release version for you and install it::

    curl -sL https://sentry.io/get-cli/ | bash

This will automatically download the correct version of ``sentry-cli`` for
your operating system and install it.  If necessarily it will prompt for
your admin password for ``sudo``.

To verify it's installed correctly you can bring up the help::

    $ sentry-cli --help

Installation via NPM
--------------------

There is also the option to install `sentry-cli` via npm for specialized
use cases.  This for instance is useful for build servers.  The package is
called ``@sentry/cli`` and in the post installation it will download
the appropriate release binary::

    $ npm install @sentry/cli

You can then find it in the `.bin` folder::

    $ ./node_modules/.bin/sentry-cli --help

.. admonition:: sudo Installation

    In case you want to install this with npm system wide with sudo you
    will need to pass `--unsafe-perm` to it::

        sudo npm install -g @sentry/cli --unsafe-perm

    This installation is not recommended however.

.. admonition:: Downloading from a Custom Source

    By default, this package will download sentry-cli from
    `the github release page <https://github.com/getsentry/sentry-cli/releases/>`__.
    This should work fine for most people. If you are experiencing issues with
    downloading from GitHub, you may need to use a different download mirror. To use
    a custom CDN, set the npm config property `sentrycli_cdnurl`. The downloader
    will append ``"/<version>/sentry-cli-<dist>"``.

        $ npm install @sentry/cli --sentrycli_cdnurl=https://mymirror.com/path

    Or add property into your `.npmrc` file (https://www.npmjs.org/doc/files/npmrc.html)

        sentrycli_cdnurl=https://mymirror.com/path

    Another option is to use the environment variable `SENTRYCLI_CDNURL`.

        $ SENTRYCLI_CDNURL=https://mymirror.com/path npm install @sentry/cli

Installation via Homebrew
-------------------------

If you are on OS X you can install `sentry-cli` via homebrew::

    $ brew install getsentry/tools/sentry-cli

Docker Image
------------

For unsupported distributions and CI systems we offer a Docker image that
comes with ``sentry-cli`` preinstalled.  It is recommended to use the
``latest`` tag, but you can also pin to a specific verison.  By default,
the command runs inside the ``/work`` directory. Mount relevant project
folders and build outputs there to allow ``sentry-cli`` to scan for resources::

    $ docker pull getsentry/sentry-cli
    $ docker run --rm -it -v $(pwd):/work getsentry/sentry-cli sentry-cli --help


Updating and Uninstalling
-------------------------

You can use ``sentry-cli update`` and ``sentry-cli uninstall`` to update
or uninstall the sentry command line interface.  These commands might be
unavailable in certain situations (for instance if you install `sentry-cli`
with homebrew).
