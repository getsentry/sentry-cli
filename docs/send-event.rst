Sending Events
==============

The ``sentry-cli`` tool can also be used for sending events.  If you want to
use it, you need to export the ``SENTRY_DSN`` environment variable and
point it to the DSN of a project of yours::

    $ export SENTRY_DSN=___DSN___

Once that is done, you can start using the ``sentry-cli send-event``
command.

Basic Events
------------

For basic message events, you just need to provide the ``--message`` or
``-m`` parameter to send a message::

    $ sentry-cli send-event -m "Hello from Sentry"

This will send a single message to sentry and record it as an event.
Along with that event, it sends basic information about the machine you are
running ``sentry-cli`` on.  You can provide ``-m`` multiple times to send
multiple lines::

    $ sentry-cli send-event -m "Hello from Sentry" -m "This is more text"

Events with Parameters
----------------------

In addition you can use ``%s`` as placeholder in a message and fill it in
with the ``-a`` parameter.  This helps reviewing them, as all messages will
be grouped together automatically::

    $ sentry-cli send-event -m "Hello %s!" -a "Joe"
    $ sentry-cli send-event -m "Hello %s!" -a "Peter"

Sending Breadcrumbs
-------------------

You can also pass a logfile to the ``send-event`` command which will be
parsed and sent along as breadcrumbs.  The last 100 items will be sent:

    $ sentry-cli send-event -m "task failed" --logfile error.log

The logfile can be in various formats.  If you want to create one yourself
you can do something along those lines::

    $ echo "$(date +%c) This is a log record" >> output.log
    $ echo "$(date +%c) This is another record" >> output.log
    $ sentry-cli send-event -m "Demo Event" --logfile output.log
    $ rm output.log

Extra Data
----------

Extra data can be attached with the ``-e`` parameter as ``KEY:VALUE``.
For instance, you can send some key value pairs like this::

    $ sentry-cli send-event -m "a failure" -e task:create-user -e object:42

Likewise, tags can be sent with ``-t`` using the same format::

    $ sentry-cli send-event -m "a failure" -t task:create-user

Specifying Releases
-------------------

Releases can be sent with the ``--release`` parameter.  A default release
is picked up automatically if you are using sentry-cli from within a git
repository.
