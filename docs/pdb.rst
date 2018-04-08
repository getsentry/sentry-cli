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

If you receive ``CoCreateInstance CLSID_DiaSource failed (msdia80.dll
unregistered?)``, download a copy of ``msdia80.dll`` and put it in ``C:\Program
Files\Common Files\Microsoft Shared\VC\``. Then as administrator, run:

::

    > regsvr32 c:\Program Files\Common Files\Microsoft Shared\VC\msdia80.dll

.. _Breakpad Tools for Windows: https://s3.amazonaws.com/getsentry-builds/getsentry/breakpad-tools/windows/breakpad-tools-windows.zip
