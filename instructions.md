### Transform `docs/api_endpoints.md` into YAML

These instructions describe the current format, the desired YAML format, and a simple, manual process to perform the transformation. Keep this high-level; no automation is required to follow these steps.

### Current format (Markdown)
- The source is `docs/api_endpoints.md`.
- Endpoints are grouped under section headers (e.g., `#### Authentication and user info`).
- Each endpoint appears as a block starting with `- method: <HTTP_VERB>` followed by nested bullets:
  - `path_template:` absolute or relative path, sometimes with query strings
  - `location:` Rust call site (sometimes more than one appears across the doc)
  - `region_aware:` yes/no
  - `absolute:` yes/no
  - `notes:` optional, includes pagination hints or special behavior

### Desired format (YAML)
Represent endpoints as a YAML map keyed by the path template (quoted). Use the full absolute URL for absolute endpoints. Do not include query strings in the key; capture them in a `query_params` object.

- Key: quoted path template (absolute URL for absolute endpoints)
- Fields per key:
  - `section`: The section header from the Markdown
  - `path_params`: Names from `{placeholders}` in the key (array)
  - `methods`: Map of HTTP method → object:
    - `locations`: Array of Rust call sites
    - `region_aware`: boolean
    - `absolute`: boolean
    - `auth`: optional, one of `bearer | x-sentry-auth | none` (only when not default)
    - `query_params`: optional map of param → details (e.g., `repeatable`, `fixed`, `notes`)
    - `pagination`: optional object (e.g., `header`, `param`, `stop_statuses`, `notes`)
    - `notes`: optional string or list

### Manual transformation process (conceptual)
1. Identify the current section: read the nearest `#### <Section Name>` for each endpoint block.
2. For each endpoint block (starting at `- method:`):
   - Extract `method`, `path_template`, `location`, `region_aware`, `absolute`, and `notes`.
   - Normalize the key:
     - If `path_template` contains a query string (e.g., `?cursor=...`), remove it for the key and record its parameters under `methods[<VERB>].query_params`.
     - For flags like `download=1`, store them under `query_params` with `fixed: "1"`.
   - Derive `path_params` by collecting all `{placeholders}` from the normalized key.
3. Group by path key:
   - Create (or reuse) the YAML object for that path key.
   - Set `section` (first occurrence wins; subsequent occurrences should match).
   - Ensure `path_params` is present and sorted/stable.
4. Populate the `methods` map for the HTTP verb:
   - Append the `location` into `locations` (use an array; de-duplicate across repeats).
   - Set `region_aware` and `absolute` booleans as listed for that method.
   - If notes indicate pagination (e.g., "Paginated", "link header", `cursor`), add a `pagination` object (keep it brief; include `header` and/or `param` if stated).
   - If an endpoint is absolute and changes auth semantics (e.g., Envelopes API uses `X-Sentry-Auth`), set `auth` accordingly.
   - Copy any remaining freeform `notes` as a string.
5. Repeat for all blocks. When the same path+method appears multiple times, merge `locations` and keep consistent flags.
6. Optionally sort the resulting YAML by path key and method name for readability. Ensure booleans are `true/false` and keys are quoted.

### Small example
```yaml
"/organizations/{org}/releases/":
  section: "Releases"
  path_params: ["org"]
  methods:
    GET:
      locations: ["api::AuthenticatedApi::list_releases"]
      region_aware: false
      absolute: false
    POST:
      locations: ["api::AuthenticatedApi::new_release", "api::AuthenticatedApi::update_release"]
      region_aware: false
      absolute: false
      notes: "Create or replacement when updating version"

"https://{ingest-host}/api/{project_id}/envelope/":
  section: "Ingest (Envelopes API)"
  path_params: ["ingest-host", "project_id"]
  methods:
    POST:
      locations: ["api::envelopes_api::EnvelopesApi::send_envelope"]
      region_aware: false
      absolute: true
      auth: "x-sentry-auth"
      notes: "DSN-derived host"
```

That’s it—the intent is to provide a faithful, normalized YAML view of the Markdown, preserving section grouping, endpoint paths, method-specific semantics, and key notes.


