#!/bin/sh

# TODO Remove after 1.37 release
if [ "${1}" == "sentry-cli" ]; then
  echo >&2 "======================================================================"
  echo >&2 "=== WARNING: Entrypoint of this Docker image has changed. ============"
  echo >&2 "=== Future versions of the image might not work correctly for you. ==="
  echo >&2 "=== More details: https://github.com/getsentry/sentry-cli#docker ====="
  echo >&2 "======================================================================"
  echo >&2
  shift
fi

exec /bin/sentry-cli "$@"
