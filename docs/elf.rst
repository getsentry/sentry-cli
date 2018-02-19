ELF Symbol Upload
=================

``sentry-cli`` can upload ELF symbols generated on various Linux distributions
to Sentry to allow symbolication of Linux and Android app crashes.  ELF stands
for *Executable and Linkable Format*, the file format used for binaries on
Linux.

Unlike other platforms, there is no standardized container for debug symbols.
They are part of the binary (executable or library) and stripped when generating
release builds due to their size.  However, there is a way to retain them in a
separate file (either in a different location or with ``.debug`` extension)::

    # There is an executable called "binary" in the CWD
    objcopy --only-keep-debug binary binary.debug
    strip -g binary
    objcopy --add-gnu-debug-link=binary.debug binary

Shared libraries installed via package managers usually provide their debugging
information in separate ``*-dev`` packages and put it in locations like
``/usr/local/debug/...``.  To receive symbolicated stack traces from those
libraries, make sure to also upload their symbols in addition to your app's
symbols.

Basic Upload
------------

Use ``upload-dif`` to upload ELF symbols and specify the ``elf`` type.  The
command will recurively scan the provided folders or ZIP archives.  If you
stripped debug information into separate files, pass the ``--no-bin`` option
to skip stripped executables or libraries.

.. admonition:: Note

    Because debug files belong to projects, you will need to specify the
    organization and project you are working with.  For more information
    about this refer to :ref:`sentry-cli-working-with-projects`.

All recent compilers and linkers generate a unique build ID and even retain it
while stripping binaries.  ``sentry-cli`` uses this identifier to associate
symbols with crash events.  If this ID is missing for some reason, invoke
``upload-dif`` before stripping so that ``sentry-cli`` can compute a stable
identifier from the unstripped file including debug information.

Example::
    $ sentry-cli upload-dif --no-bin -t elf .

Upload Options
--------------

There are a few options you can supply for the upload process

``--no-bin``
    Exclude executables and libraries from the upload and search for debug files
    only.  Activate this setting if debug information has been stripped and
    moved into separate files.

``--no-debug``
    Exclude files containing only stripped debugging information.  Use this
    option when uploading unstripped binaries to avoid using the wrong files
    when searching folders and archives.

``--no-zips``
    By default, sentry-cli will open and search ZIP archives for files. This is
    especially useful when downloading builds from iTunes Connect. Use this
    switch to disable if your search paths contain large ZIP archives without
    debug information files to speed up the search.

``--no-reprocessing``
    This parameter prevents Sentry from triggering reprocessing right
    away.  It can be useful in some limited circumstances where you want
    to upload files in multiple batches and you want to ensure that Sentry
    does not start reprocessing before some optional dsyms are uploaded.
    Note though that someone can still in the meantime trigger
    reprocessing from the UI.
