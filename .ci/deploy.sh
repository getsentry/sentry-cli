#!/bin/bash
pip install --user requests==2.10.0
python ./.ci/upload-release.py
