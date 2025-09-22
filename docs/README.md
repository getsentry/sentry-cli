### API endpoint extraction: how to rerun

This folder contains `api_endpoints.md`, a static inventory of HTTP endpoints used by sentry-cli.

To regenerate or validate the inventory:

1) Validate HTTP boundary (no stray HTTP outside `src/api/`):

```bash
rg -n --type rust "curl::|reqwest|hyper|ureq|isahc" /workspace/src --glob '!src/api/**' --hidden | cat
rg -n --type rust 'custom_request\(|http_headers\(|response_code\(|\.url\("https?://' /workspace/src --glob '!src/api/**' --hidden | cat
```

2) Extract inline endpoint literals in `src/api/`:

```bash
rg -n --type rust '\\.(get|post|put|delete)\\(\\s*"(?:/|https?://)[^"]*"' /workspace/src/api | cat
```

3) Locate `format!`-built paths and variable-based URLs:

```bash
rg -n --type rust 'let\\s+(?:mut\\s+)?\\w+\\s*=\\s*format!\\(' /workspace/src/api | cat
rg -n --type rust '\\.push_str\\(' /workspace/src/api | cat
rg -n --type rust '\\.(get|post|put|delete)\\(\\s*&\\w+' /workspace/src/api | cat
rg -n --type rust 'download\\(\\s*&\\w+' /workspace/src/api | cat
```

4) Direct `request(Method::...)` calls:

```bash
rg -n --type rust 'request\\(Method::(Get|Post|Put|Delete),\\s*[^,]+' /workspace/src/api | cat
```

5) Absolute URLs (downloads/uploads/others):

```bash
rg -n --type rust '\\.(get|post|put|delete)\\(\\s*"https?://' /workspace/src/api | cat
rg -n --type rust 'download\\(|download_with_progress\\(|upload_chunks\\(' /workspace/src | cat
```

6) Envelopes ingest:

```bash
rg -n --type rust 'envelope_api_url\\(|X-Sentry-Auth' /workspace/src/api/envelopes_api.rs | cat
```

7) Cross-check with integration fixtures:

```bash
find /workspace/tests/integration/_responses -type f -name '*.json' \
  | sed 's#^.*/_responses/##' \
  | sed 's/\\.json$//' \
  | sed 's#__#/#g' \
  | sed 's/^api__0\\(.*\\)$/\\1/' \
  | sort -u | head -200 | cat
```

8) Manually review changed call sites and update `api_endpoints.md` accordingly.

Requirements:
- ripgrep (`rg`) must be installed. On Ubuntu: `sudo apt-get update && sudo apt-get install -y ripgrep`.

