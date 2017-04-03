Release Management
==================

The ``sentry-cli`` tool can be used for release management on Sentry.  It
allows you to create, edit and delete releases as well as upload release
artifacts for them.

.. admonition:: Note

    Because releases work on projects you will need to specify the
    organization and project you are working with.  For more information
    about this refer to :ref:`sentry-cli-working-with-projects`.

Creating Releases
-----------------

Releases are created with the ``sentry-cli releases new`` command.  It
takes at the very least a version identifier that uniquely identifies the
relases.  It can be arbitrary but for certain platforms recommendations
exist:

*   for mobile devices use ``VERSION_NUMBER`` or ``VERSION_NUMBER
    (BUILD_NUMBER)``.  So for instance ``1.0.0`` or ``1.0.0 (1234)``.
*   if you use a DVCS we recommed using the identifying hash (eg: the
    commit SHA, ``da39a3ee5e6b4b0d3255bfef95601890afd80709``).  You can
    let sentry-cli automatically determine this hash for supported
    version control systems with ``sentry-cli releases propose-version``.
*   if you tag releases we recommend using the release tag (eg:
    ``v1.0.0``).

Separately to the release version you can provide an optional reference label.
That label can point to a human readable identifier that is a reference to
that version that the time.  As an example if you deploy a specific
revision of the master branch you can set the version to the SHA of the
commit but the reference to ``master``.  This can help you figure out if a
release happened from master, unstable or a feature branch.

Releases can also be auto created by different systems.  For instance upon
uploading a sourcemap a release is automatically created.  Likewise
releases are created by some clients when an event for a release comes in.

Finalizing Releases
-------------------

By default a release is created "unreleased".  This can be changed by
passing either ``--finalize`` to the ``new`` command which will
immediately finalize the release or you can separately later call
``sentry-cli releases finalize VERSION``.  The latter is useful if you are
managing releases as part of a build process::

    #!/bin/sh
    sentry-cli releases new $VERSION
    # do you build steps here
    # once you are done, finalize
    sentry-cli releases finalize $VERSION

Then the UI will reflect the time it took for the release to be created.
You can also finalize it later when you pushed the release live (eg:
deployed to your machines, enabled in the app store etc.).

Managing Release Artifacts
--------------------------

When you are working with JavaScript and other platforms you can upload
release artifacts to Sentry which are then considered during processing.
The most common release artifact are :ref:`source maps <raven-js-sourcemaps>`
for which ``sentry-cli`` has specific support.

To manage release artfacts the ``sentry-cli releases files`` command can
be used which itself provides various sub commands.

Upload Files
````````````

The most common use case is to upload files.  For the generic upload the 
``sentry-cli releases files VERSION upload`` command can be used.  However
since most release artifacts are JavaScript sourcemap related we have a
:ref:`sentry-cli-sourcemaps` convenience method for that.

Files uploaded are typically named with a full (eg:
``http://example.com/foo.js``) or truncated URL (eg: ``~/foo.js``).

Release artifacts are only considered at time of event processing.  So
while it's possible to modify release artifacts after the fact they will
only be considered for future events of that release.

The first argument to ``upload`` is the path to the file, the second is an
optional URL we should associate it with.  Note that if you want to use an
abbreviated URL (eg: ``~/foo.js``) make sure to use single quotes to avoid
the expansion by the shell to your home folder.

::

    $ sentry-cli releases files VERSION upload /path/to/file '~/file.js'

.. _sentry-cli-sourcemaps:

Upload Sourcemaps
`````````````````

For sourcemap upload a separate command is provided which assists you in
uploading and verifying sourcemaps::

    $ sentry-cli releases files VERSION upload-sourcemaps /path/to/sourcemaps

This command provides a bunch of options and attempts as much auto
detection as possible.  By default it will scan the provided path for
files and upload them named by their path with a ``~/`` prefix.  It will
also attempt to figure out references between minified files and
sourcemaps based on the filename.  So if you have a file named
``foo.min.js`` which is a minified JavaScript file and a sourcemap named
``foo.min.map`` for example, it will send a long a ``Sourcemap`` header to
associate them.  This works for files the system can detect a relationship
of.

The following options exist to change the behavior of the upload command:

``--no-sourcemap-reference``
    This prevents the automatic detection of sourcemap references.  It's
    not recommended to use this option since the system falls back to not
    emitting a reference anyways.  It is however useful if you are
    manually adding ``sourceMapURL`` comments to the minified files and
    you know that they are more correct than the autodetection.

``--rewrite``
    When this option is provided ``sentry-cli`` will rewrite the
    sourcemaps before upload.  This does two things:

    1.  it flattens out indexed sourcemaps.  This has the advantage that
        it can compress sourcemaps sometimes which might improve your
        processing times and can work with tools that embed local paths
        for sourcemap references which would not work on the server.  This
        is useful when working with sourcemaps for development purposes in
        particular.
    2.  local file references in sourcemaps for source contents are
        inlined.  This works particularly well with react-native projects
        which might reference thousands of files you probably do not want
        to upload separately.
    3.  It automatically validates sourcemaps before upload very
        accurately which can spot errors you would not find otherwise
        until an event comes in.  This is an improved version of what
        ``--validate`` does otherwise.

``--strip-prefix`` / ``--strip-common-prefix``
    When paired with ``--rewrite`` this will chop-off a prefix from
    uploaded files.  For instance you can use this to remove a path that
    is build machine specific.  The common prefix version will attempt to
    automatically guess what the common prefix is and chop that one off
    automatically.

``--validate``
    This attempts sourcemap validation before upload when rewriting is not
    enabled.  It will spot a variety of issues with sourcemaps and cancel
    the upload if any are found.  This is not the default as this can
    cause false positives.

``--url-prefix``
    This sets an URL prefix in front of all files.  This defaults to
    ``~/`` but you might want to set this to the full URL.  This is also
    useful if your files are stored in a sub folder.  eg: ``--url-prefix
    '~/static/js'``

``--ext``
    Adds an additional file extension to be considered.  By default the
    following file extensions are processed: ``js``, ``map``, ``jsbundle``
    and ``bundle``.  The tool will automatically detect the type of the
    file by the file contents (eg: sources, minified sources, and
    sourcemaps) and act appropriately.

Some example usages::

    $ sentry-cli releases files 0.1 upload-sourcemaps /path/to/sourcemaps
    $ sentry-cli releases files 0.1 upload-sourcemaps /path/to/sourcemaps \
        --url-prefix '~/static/js`
    $ sentry-cli releases files 0.1 upload-sourcemaps /path/to/sourcemaps \
        --url-prefix '~/static/js` --rewrite --strip-common-prefix

List Files
``````````

To list uploaded files the following command can be used::

    $ sentry-cli releases files VERSION list

This will return a list of all uploaded files for that release.

Delete Files
````````````

You can also delete already uploaded files.  Either by name or all files
at once::

    $ sentry-cli releases files VERSION delete NAME_OF_FILE
    $ sentry-cli releases files VERSION delete --all
