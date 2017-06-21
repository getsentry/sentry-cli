ProGuard Mapping Upload
=======================

``sentry-cli`` can be used to upload proguard files to Sentry however in
most situations you would use the `gradle plugin
<https://github.com/getsentry/sentry-java>`_ to do that.  There are some
situations however where you would upload proguard files manually (for
instance when you only release some of the builds you are creating).

.. admonition:: Note

    Because proguard files work on projects you will need to specify the
    organization and project you are working with.  For more information
    about this refer to :ref:`sentry-cli-working-with-projects`.

Basic Upload
------------

The ``upload-proguard`` command is the one to use for uploading proguard
files.  It takes the path to one or more proguard mapping files and will
upload them to Sentry.  If you want to associate them with an Android
app you should also point it to a processed `AndroidManifest.xml` from a
build intermediate folder.  Example::

    sentry-cli upload-proguard \
        --android-manifest app/build/intermediates/manifests/full/release/AndroidManifest.xml \
        app/build/outputs/mapping/release/mapping.txt

Since the sentry-java client needs to know the UIUD of the mapping file
you will need to embed it in a ``sentry-debug-meta.properties`` file.  If
you supply ``--write-properties`` that is done automatically::

    sentry-cli upload-proguard \
        --android-manifest app/build/intermediates/manifests/full/release/AndroidManifest.xml \
        --write-properties app/build/intermediates/assets/release/sentry-debug-meta.properties \
        app/build/outputs/mapping/release/mapping.txt

Upload Options
--------------

``--no-reprocessing``
    This parameter prevents Sentry from triggering reprocessing right
    away.  It can be useful in some limited circumstances where you want
    to upload files in multiple batches and you want to ensure that Sentry
    does not start reprocessing before some optional dsyms are uploaded.
    Note though that someone can still in the meantime trigger
    reprocessing from the UI.

``--no-upload``
    Disables the actual upload.  This runs all steps for the processing
    but does not trigger the upload (this also automatically disables
    reprocessing.  This is useful if you just want to verify the mapping
    files and write the proguard UUIDs into a proeprties file.

``--require-one``
    Requires at least one file to upload or the command will error.
