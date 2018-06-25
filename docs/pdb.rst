PDB Upload
==========

Microsoft PDB files are not yet supported directly by Sentry. Until we provide
official support, you can convert them to Breakpad symbols and upload those
instead:

1. Obtain the ``.pdb`` file and put it on a Windows machine
2. Download our `Breakpad Tools for Windows`_ and extract ``dump_syms.exe``
3. Run ``dump_syms foo.pdb > foo.sym``
4. Follow instructions at :doc:`breakpad`.

Troubleshooting
---------------

"CoCreateInstance CLSID_DiaSource failed (msdia80.dll unregistered?)"
`````````````````````````````````````````````````````````````````````

Download a copy of ``msdia80.dll`` and put it in ``C:\Program Files\Common
Files\Microsoft Shared\VC\``. Then as administrator, run:

::

    > regsvr32 "C:\Program Files\Common Files\Microsoft Shared\VC\msdia80.dll"

Then, run the ``dump_syms`` command from a Visual Studio command prompt. This
will also work with later versions, such as ``msdia140.dll``.

"Unsupported file" error or "No debug debug information files found"
````````````````````````````````````````````````````````````````````

Sentry CLI or Sentry do not recognize your Breakpad symbols file, most likely
due to encoding issues. Make sure the file is saved without a byte order mark
(BOM).

Older PowerShell versions used to encode with BOM by default. To prevent this,
set the ``$OutputEncoding`` variable before calling ``dump_syms``.

.. _Breakpad Tools for Windows: https://s3.amazonaws.com/getsentry-builds/getsentry/breakpad-tools/windows/breakpad-tools-windows.zip
