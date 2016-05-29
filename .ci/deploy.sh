#!/bin/bash
virtualenv .venv
.venv/bin/activate
pip install requests
python ./.ci/upload-release.py
