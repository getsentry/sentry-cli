Configuration and Authentication
================================

For most functionality you need to authenticate with Sentry.  To sign the
CLI tool in you can use the `login` command which will guide you through
it::

    $ sentry-cli login

If you want to manually authenticate ``sentry-cli`` you can to your to
your auth token settings in your user account (User Icon -> API) and
generate a new token.  Afterwards you can export the ``SENTRY_AUTH_TOKEN``
environment variable::

    export SENTRY_AUTH_TOKEN=your-auth-token

Alternatively you can provide the ``--auth-token`` command line parameter
whenever you invoke `sentry-cli` or add it to your `.sentryclirc` config
file.

By default ``sentry-cli`` will connect to sentry.io but for
on-premise you can also sign in elsewhere::

    $ sentry-cli --url https://myserver.invalid/ login

Configuration File
------------------

The `sentry-cli` tool can be configured with a config file named
:file:`.sentryclirc` as well as environment variables.  The config file is
looked for upwards from the current path and defaults from
`~/.sentryclirc` are always loaded.  You can also override these settings
from command line parameters.

The config file uses standard INI syntax.

By default ``sentry-cli`` will connect to sentry.io.  For on-prem you can
export the ``SENTRY_URL`` environment variable and point it to your
installation::

    export SENTRY_URL=https://mysentry.invalid/

Alternatively you can add it to your ``~/.sentryclirc`` config.  This
is also what the `login` command does:

.. sourcecode:: ini

    [defaults]
    url = https://mysentry.invalid/

Configuration Values
--------------------

The following settings are available (first is the environment variable, the
value in the parentheses is the config key in the config file):

``SENTRY_AUTH_TOKEN`` (`auth.token`):
    the authentication token to use for all communication with Sentry.
``SENTRY_API_KEY`` (`auth.api_key`):
    the legacy API key for authentication if you have one.
``SENTRY_URL`` (`defaults.url`):
    The URL to use to connect to sentry.  This defaults to
    ``https://sentry.io/``.
``SENTRY_ORG`` (`defaults.org`):
    the slug of the organization to use for a command.
``SENTRY_PROJECT`` (`defaults.project`):
    the slug of the project to use for a command.
(`http.keepalive`):
    This ini only setting is used to control the behavior of the SDK
    with regards to HTTP keepalives.  The default is `true` but it can
    be set to `false` to disable keepalive support.
``http_proxy`` (`http.proxy_url`):
    The URL that should be used for the HTTP proxy.  The standard
    ``http_proxy`` environment variable is also honored.  Note that it
    is lowercase.
(`http.proxy_username`):
    This ini only setting sets the proxy username in case proxy
    authentication is required.
(`http.proxy_password`):
    This ini only setting sets the proxy password in case proxy
    authentication is required.
(`http.verify_ssl`):
    This can be used to disable SSL verification when set to false.  You
    should never do that unless you are working with a known self signed
    server locally.
(`http.check_ssl_revoke`):
    If this is set to false then SSL revocation checks are disabled on
    Windows.  This can be useful when working with a corporate SSL MITM
    proxy that does not properly implement revocation checks.  Do not use
    this unless absolutely necessary.
``SENTRY_LOG_LEVEL`` (`log.level`):
    Configures the log level for the SDK.  The default is ``warning``.
    If you want to see what the library is doing you can set it to
    ``info`` which will spit out more information which might help to
    debug some issues with permissions.

Validating The Config
---------------------

To make sure everything works you can run ``sentry-cli info`` and it should
print out some basic information about the Sentry installation you connect
to as well as some authentication information.

.. _sentry-cli-working-with-projects:

Working with Projects
---------------------

Many commands require you to specify the organization and project to work
with.  There are multiple ways in which you can specify this.

Config Defaults
```````````````

If you are always working with the same projects you can set it in the
``.sentryclirc`` file:

.. sourcecode:: ini

    [defaults]
    project=my-project
    org=my-org

Environment Variables
`````````````````````

You can also set these defaults in environment variables.  There are two
environment vaiables that control this (``SENTRY_ORG`` and
``SENTRY_PROJECT``)  which you can export::

    export SENTRY_ORG=my-org
    export SENTRY_PROJECT=my-project

Explicit Options
````````````````

Lastly you can provide these values also explicitly with the command you
are executing.  The parameters are always called ``--org`` or ``-o`` for
the organization and ``--project`` or ``-p`` for the project.  For
instance if you are managing releases you can use it like this::

    $ sentry-cli releases -o my-org -p my-project list

Note that if a command has subcommands the parameter needs to go to the
first one (so for instance cannot define the parameter on the ``releases
list`` command).
