Debug Information Files
=======================

``sentry-cli`` can be used to validate and upload debug information files
(:doc:`dSYM files <dsym>`, :doc:`proguard files <proguard>`, etc.).

Debug information files are additional files that help us provide better
information about your crash reports.  We currently support the following
formats:

*   :doc:`dSYM files <dsym>` for iOS, tvOS and macOS
*   :doc:`proguard mappings <proguard>` for Android

Note that sourcemaps while also being debug information files are handled
differently in Sentry.  For more information see
:ref:`Source maps in sentry-cli <sentry-cli-sourcemaps>`.

File Assocations
----------------

Generally Sentry associates debug information files with events through
their UUID.  Each debug information file in general at least has one
UUID (thought dSYM files can contain more than one ID).  If you have a
debug information file you can use the ``sentry-cli difutil check``
command to print the contained UUIDs.

Likewise if you know the UUIDs in many cases the upload commands (like
``sentry-cli upload-dsym``) can automatically discover debug information
files based on the UUIDs provided.

Checking Files
--------------

Not all debug information files can be used by Sentry.  To see if they are
usable or not you can use the ``sentry-cli difutil check`` command::

    $ sentry-cli difutil check /path/to/debug/information/file

This will report the UUIDs of the debug information file as well as if it
passes basic requirements for Sentry.

Finding Files
-------------

If you see in Sentry's UI that debug information files are missing but you
are not sure how to best find them you can use the ``sentry-cli difutil
find`` command to look for them::

    $ sentry-cli difutil find UUID

Additionally ``sentry-cli upload-dsym`` can automatically look for them.

Uploading Files
---------------

For uploading debug information files specific commands exist depending
on the type of the file.  The following commands exist:

*   :doc:`upload-dsym <dsym>`
*   :doc:`upload-proguard <proguard>`
