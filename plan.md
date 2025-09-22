### Plan: Enumerate all API endpoints used by sentry-cli

#### Goals
- Identify every HTTP endpoint that sentry-cli can call, including dynamic and region-specific ones.
- Ensure the enumeration is comprehensive and stays maintainable as the code evolves.
- Prefer static analysis from code in `src/api/` (the network boundary), with optional automated checks and CI guardrails.

### Background: Where network calls originate
- The `src/api/mod.rs` module is the single low-level HTTP layer and also defines the high-level API methods used by commands.
  - Low-level wrappers (sinks):
    - `Api::request(method, url, region_url)` → constructs `ApiRequest`
    - `Api::{get, post, put, delete}` → convenience wrappers over `request`
    - `Api::{download, download_with_progress}` → GETs into a file handle (absolute or relative URLs)
    - `Api::upload_chunks(url, ...)` → POSTs multipart to an absolute or relative URL
    - `ApiRequest::{send, send_into}` → actually perform the request
  - High-level API builders:
    - `AuthenticatedApi` and `RegionSpecificApi` implement methods that build paths (mostly starting with `/`) using `format!` and `PathArg`/`QueryArg` encoders, then call `get/post/put/delete`.
  - Absolute URLs: if `url` is absolute, `Api::request` disables auth injection and uses it verbatim (e.g., release registry, downloads, chunk upload servers).
- The `src/api/envelopes_api.rs` module performs ingest calls using DSN-derived absolute URLs with an `X-Sentry-Auth` header.
- Encoding helpers:
  - `PathArg` and `QueryArg` ensure safe interpolation into endpoint templates.

Conclusion: Enumerating endpoints from `src/api/mod.rs` plus `src/api/envelopes_api.rs` is the source of truth; everything else should call into these.

### Invariants to validate up front (so we don’t miss calls)
- No other module should perform raw HTTP on its own (e.g., `curl::easy::Easy::new()` or other HTTP clients) outside `src/api/`.
- If violations exist, add them to the list of sinks and include their endpoints.

Commands to validate the boundary:
```bash
# Scan for direct HTTP clients outside the API module
rg -n --type rust "curl::|reqwest|hyper|ureq|isahc" /workspace/src \
  --glob '!src/api/**' --hidden

# Scan for manual header or URL settings outside API
rg -n --type rust 'custom_request\(|http_headers\(|response_code\(|\.url\("https?://' /workspace/src \
  --glob '!src/api/**' --hidden
```
If these return nothing significant, you can trust that `src/api/` contains all outbound HTTP definitions.

### Comprehensive extraction workflow

1) Extract inline endpoint literals in `src/api/`
- These are direct calls like `.get("/organizations/..."`, `.post("https://...")`.
```bash
rg -n --type rust '\\.(get|post|put|delete)\(\s*"(?:/|https?://)[^"]*"' /workspace/src/api
```
- Record method, literal, and function name where it appears.

2) Extract format!-built endpoints and variables named `path` (and variants)
- Pattern: build a `path` with `format!`, possibly mutate with `push_str`, then pass `&path` into `.get/.post/.put/.delete` or `Api::download`.
```bash
# Assignments of a path with a format!
rg -n --type rust 'let\s+(?:mut\s+)?\w+\s*=\s*format!\(' /workspace/src/api

# Mutations of that path
rg -n --type rust '\.push_str\(' /workspace/src/api

# Calls where a variable is used as the URL arg
rg -n --type rust '\\.(get|post|put|delete)\(\s*&\w+' /workspace/src/api
rg -n --type rust 'download\(\s*&\w+' /workspace/src/api
```
- For each function, reconstruct the final path template by reading the `format!` string(s) and any appended query pieces. Capture the base template (e.g., `"/organizations/{}/releases/{}/files/"`) and note optional query params that may be appended later.

3) Extract `.request(Method::...)` calls
- Some methods call `request` directly (e.g., chunk assembling endpoints) and then add JSON bodies.
```bash
rg -n --type rust 'request\(Method::(Get|Post|Put|Delete),\s*[^,]+' /workspace/src/api
```
- Capture the first URL argument (inline literal, `format!`, or variable) and reconstruct as in step 2.

4) Extract absolute-URL calls for downloads and chunk uploads
- Absolute URLs bypass base URL and auth, so list them explicitly:
```bash
# Inline absolute URLs
rg -n --type rust '\\.(get|post|put|delete)\(\s*"https?://' /workspace/src/api

# Downloads and uploads anywhere in the repo
rg -n --type rust 'download\(|download_with_progress\(|upload_chunks\(' /workspace/src
```
- For `upload_chunks`, the URL may be server-provided (dynamic). Enumerate call sites and annotate them as “server-provided absolute URL” with the context (e.g., chunk upload server from `/chunk-upload/` options).

5) Envelopes API endpoints (ingest)
- In `src/api/envelopes_api.rs`, URLs come from DSN: this yields an absolute ingest endpoint like `https://<ingest-host>/api/<project>/envelope/`.
```bash
rg -n --type rust 'envelope_api_url\(|X-Sentry-Auth' /workspace/src/api/envelopes_api.rs
```
- Document the template as “ingest envelope URL (dynamic host via DSN)”.

6) Cross-check against integration test fixtures
- Fixtures in `tests/integration/_responses/` encode endpoints by filename and contents. Use them to verify coverage.
```bash
# Sample: list unique mocked endpoint stems from fixture filenames
find /workspace/tests/integration/_responses -type f -name '*.json' \
  | sed 's#^.*/_responses/##' \
  | sed 's/\.json$//' \
  | sed 's#__#/#g' \
  | sed 's/^api__0\(.*\)$/\\1/' \
  | sort -u | head -200 | cat
```
- Spot-check a handful against the enumerated list from steps 1–5. Add any misses.

7) Produce a structured inventory
- Emit `docs/api_endpoints.md` (or JSON/CSV) with:
  - method: GET/POST/PUT/DELETE
  - path_template: e.g., `/organizations/{org}/releases/{version}/files/`
  - location: fully-qualified Rust function (e.g., `api::AuthenticatedApi::list_release_files`)
  - region_aware: yes/no (whether it can route via `RegionSpecificApi`)
  - absolute: yes/no (absolute URL used)
  - notes: pagination, dynamic query params, error semantics

8) Add guardrails so future endpoints can’t be missed
- CI static checks:
  - Fail if any `curl::easy::Easy` or other HTTP client is used outside `src/api/`:
```bash
rg -n --type rust "curl::|reqwest|hyper|ureq|isahc" /workspace/src --glob '!src/api/**' --quiet || true
# Non-zero exit if matches found
```
  - Optional semgrep rule (below) to flag new sinks or raw HTTP calls outside `src/api/`.

### Optional tooling to improve confidence (justification and examples)
- ripgrep (recommended): fast, zero-dependency, ideal for literal and regex-based searches. The commands above already cover most patterns.
- semgrep (optional, for maintainability): lets us codify patterns for sinks and common path-building flows, catching future changes in CI.
  - Pros: readable rules, works well for Rust for simple patterns; can track variable reuse within a function using ellipses.
  - Cons: dataflow is limited; complex multi-function propagation is out-of-scope (but we don’t need it because paths are built within `src/api/`).
- rust-analyzer (optional): interactive “Find References” on `get/post/put/delete/request` and `ApiRequest::send` to spot new call sites quickly during development.
- tree-sitter (optional): for power users wanting AST-precise queries; overkill for this task.
- We do not need heavyweight static analysis; endpoints are centrally defined and constructed in `src/api/` using clear patterns. `rg` + spot review is sufficient; semgrep adds helpful CI guardrails.

Example semgrep rules (save as `.semgrep.yml`):
```yaml
rules:
  - id: api-endpoint-sinks
    languages: [rust]
    message: API endpoint call
    severity: INFO
    pattern-either:
      - pattern: $RECV.get($URL, ...)
      - pattern: $RECV.post($URL, ...)
      - pattern: $RECV.put($URL, ...)
      - pattern: $RECV.delete($URL, ...)
      - pattern: $RECV.request(Method::$M, $URL, ...)
      - pattern: $RECV.download($URL, ...)
    metadata:
      category: sinks
  - id: api-endpoint-inline-literal
    languages: [rust]
    message: Inline endpoint literal
    severity: INFO
    pattern-either:
      - pattern: $RECV.get("$URLLIT", ...)
      - pattern: $RECV.post("$URLLIT", ...)
      - pattern: $RECV.put("$URLLIT", ...)
      - pattern: $RECV.delete("$URLLIT", ...)
      - pattern: $RECV.request(Method::$M, "$URLLIT", ...)
      - pattern: $RECV.download("$URLLIT", ...)
  - id: api-endpoint-from-format-var
    languages: [rust]
    message: Endpoint built via format! and later used
    severity: INFO
    patterns:
      - pattern-inside: |
          fn $F(..) {
            ...
            let $P = format!($FMT, ...);
            ...
            $CALL
            ...
          }
      - pattern-either:
          - pattern: $RECV.get(&$P, ...)
          - pattern: $RECV.post(&$P, ...)
          - pattern: $RECV.put(&$P, ...)
          - pattern: $RECV.delete(&$P, ...)
          - pattern: $RECV.request(Method::$M, &$P, ...)
          - pattern: $RECV.download(&$P, ...)
```
Run:
```bash
semgrep --config .semgrep.yml --error --include /workspace/src/api
```

### Edge cases and how to account for them
- Server-provided URLs:
  - `upload_chunks`: the upload endpoint may come from server options; mark as “dynamic absolute URL (server-provided)” and do not attempt to hardcode it.
- DSN-based ingest endpoints:
  - `EnvelopesApi::send_envelope`: absolute URL derived from DSN; document as a template rather than a fixed host.
- Region routing:
  - `RegionSpecificApi` uses `region_url` only when base URL is the default `https://sentry.io/`; otherwise, relative paths are used against the configured base. The path templates are identical; annotate endpoints as “region-aware”.

### Execution blueprint (what to do next)
1) Validate the HTTP boundary using the two `rg` commands in the invariants section. If violations exist, add their sinks and enumerate endpoints there too.
2) Run the commands in steps 1–5 to list all literals and reconstructed templates from `src/api/` and `envelopes_api.rs`.
3) Manually review each function where a `path` is constructed to capture the final template and note optional query parameters.
4) Cross-check with the response fixtures in `tests/integration/_responses/`; add any missing endpoints.
5) Generate `docs/api_endpoints.md` with the structured inventory fields described above.
6) Add the CI guardrails (ripgrep or semgrep checks) to prevent future drift.

### Deliverables
- A human-readable list in `docs/api_endpoints.md` (or `.json`) containing all endpoint templates, with metadata (method, location, region-aware, absolute, notes).
- Optional `.semgrep.yml` and CI hook to enforce network boundary and flag new sinks.
- A short `README` in `docs/` explaining how to rerun the extraction.