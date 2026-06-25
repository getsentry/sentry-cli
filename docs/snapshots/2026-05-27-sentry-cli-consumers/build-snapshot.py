#!/usr/bin/env python3
from pathlib import Path
import re

base = Path(__file__).parent
checklist = base / 'checklist.md'
snapshot = base.parent / '2026-05-28-sentry-cli-consumers.md'

repos = re.findall(r'`(getsentry/[^`]+)`', checklist.read_text())
text = snapshot.read_text()
missing = [r for r in repos if f'`{r}`' not in text]
if missing:
    raise SystemExit('missing repos: ' + ', '.join(missing))
print(f'ok: {len(repos)} repos covered in {snapshot.name}')
