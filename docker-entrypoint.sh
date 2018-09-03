#!/bin/sh

show_warning() {
  echo >&2 "======================================================================"
  echo >&2 "=== WARNING: Entrypoint of this Docker image has changed. ============"
  echo >&2 "=== Future versions of the image might not work correctly for you. ==="
  echo >&2 "=== More details: https://github.com/getsentry/sentry-cli#docker ====="
  echo >&2 "======================================================================"
  echo >&2
}


# TODO Remove after 1.37 release
if [ "${1}" == "sentry-cli" ]; then
  show_warning
  shift
elif [ "${1}" == "sh" ] || [ "${1}" == "/bin/sh" ]; then
  show_warning
  exec "$@"
fi

exec /bin/sentry-cli "$@"
