dSYM Upload
===========

``sentry-cli`` is the tool to use to upload dSYM files to Sentry when
you want symbolication for your iOS applications to work.  It is also used
behind the scenes if you use systems like fastlane.

For generation information about dSYM handling you can refer to
:ref:`uploading-dsyms` as well as :doc:`dif` for a general introduction,
but we have a reference on the command line interface here.

.. admonition:: Note

    Because dSYM files work on projects you will need to specify the
    organization and project you are working with.  For more information
    about this refer to :ref:`sentry-cli-working-with-projects`.

Basic Upload
------------

The ``upload-dsym`` command is the command to use for uploading debug
symbols.  It automatically picks up the ``DWARF_DSYM_FOLDER_PATH``
environment variable that Xcode exports in case you are using it from
within an Xcode build step, alternatively you need to provide the path to
dSYMs as argument.

Since dSYMs are uniquely identified you do not need to associate them with
a release, however the tool will automatically scan for a ``Info.plist``
in the path provided to find the release.  If a release is found the dSYMS
are associated automatically.  Unassociated dSYMs are still considered for
processing but you won't easily see which go with which releases.

Example::

    $ sentry-cli upload-dsym

Upload Options
--------------

There are a few options you can supply for the upload process

``--force-foreground``
    This option forces the dSYM upload to happen in foreground.  This only
    affects uploads happening from within Xcode.  By default the upload
    process when started from Xcode will detach and finish in the
    background.  If you need to debug the upload process it might be
    useful to force the upload to happen in the foreground.

``--info-plist``
    If your info.plist is at a non standard location you can specify it
    here.

``--no-reprocessing``
    This parameter prevents Sentry from triggering reprocessing right
    away.  It can be useful in some limited circumstances where you want
    to upload files in multiple batches and you want to ensure that Sentry
    does not start reprocessing before some optional dsyms are uploaded.
    Note though that someone can still in the meantime trigger
    reprocessing from the UI.
