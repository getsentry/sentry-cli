#!/bin/bash

# Older travis OS X images might not have pip
if ! hash pip 2> /dev/null; then
  easy_install --user requests==2.10.0
else
  pip install --user requests==2.10.0
fi
python ./.ci/upload-release.py
