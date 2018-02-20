Debug Information Files
=======================

``sentry-cli`` can be used to validate and upload debug information files
(dSYM, Proguard files, etc.).

Debug information files are additional files that help us provide better
information about your crash reports.  We currently support the following
formats:

*   :doc:`dSYM files <dsym>` for iOS, tvOS and macOS
*   :doc:`ELF symbols <elf>` for Linux and Android
*   :doc:`Breakpad symbols <breakpad>` for Breakpad or Crashpad
*   :doc:`Proguard mappings <proguard>` for Android

Note that sourcemaps, while also being debug information files, are handled
differently in Sentry.  For more information see
:ref:`Sourcemaps in sentry-cli <sentry-cli-sourcemaps>`.

File Assocations
----------------

Generally, Sentry associates debug information files with events through
their unique ID.  Each debug information file has at least one unique ID.
As a special case, dSYM files can contain symbols for more than one ID.
If you have a debug information file you can use the ``sentry-cli difutil
check`` command to print the contained IDs. The ID depends on the file type:
dSYMs and proguard files use UUIDs, Linux symbols use longer hash values
(e.g. SHA256) and PDBs use UUIDs and an age field.

Likewise, the upload commands (e.g. ``sentry-cli upload-dif``) allow to search
for specific debug information files by providing their known identifiers.

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
are not sure how to locate them, you can use the ``sentry-cli difutil
find`` command to look for them::

    $ sentry-cli difutil find <identifier>

Additionally, ``sentry-cli upload-dif`` can automatically search for files
in a folder or ZIP archive.

Uploading Files
---------------

Options for the debug file upload depend on the upload environment and
debug format.  For detailed instructions, please refer to the resources
linked below:

*   :doc:`dSYM Upload <dsym>`
*   :doc:`ELF Symbol Upload <elf>`
*   :doc:`Breakpad Symbol Upload <breakpad>`
*   :doc:`Proguard Mapping Upload <proguard>`
