#!/bin/sh

# For compatibility with older entrypoints
if [ "${1}" == "sentry-cli" ]; then
  shift
elif [ "${1}" == "sh" ] || [ "${1}" == "/bin/sh" ]; then
  exec "$@"
fi

exec /bin/sentry-cli "$@"
