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
called ``sentry-cli-binary`` and in the post installation it will download
the appropriate release binary::

    $ npm install sentry-cli-binary

You can then find it in the `.bin` folder::

    $ ./node_modules/.bin/sentry-cli --help

Installation via Homebrew
-------------------------

If you are on OS X you can install `sentry-cli` via homebrew::

    $ brew install getsentry/tools/sentry-cli

Updating and Uninstalling
-------------------------

You can use ``sentry-cli update`` and ``sentry-cli uninstall`` to update
or uninstall the sentry command line interface.  These commands might be
unavailable in certain situations (for instance if you install `sentry-cli`
with homebrew).
