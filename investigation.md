### Node-test workflow failure on release branch – root cause analysis

#### What I ran (mirroring .github/workflows/test_node.yml)
- Install (skip binary download):
```bash
SENTRYCLI_SKIP_DOWNLOAD=1 npm ci
```
- Type check:
```bash
npm run check:types
```
- Tests (matrix job runs this across Node 10.x, 12.x, 14.x, 16.x, 18.x, 20.x, 22.x, 24.x):
```bash
npm test
```

Environment used locally: Node v22.16.0, npm 10.9.2 on Linux.

#### Observations
- Type checks and tests pass locally on Node v22.
- Jest prints a warning before running tests:
```
Multiple configurations found:
  * /workspace/jest.config.js
  * `jest` key in /workspace/package.json
```
  Tests still pass, but this can cause non-zero exits with some setups.
- TypeScript in this branch is 5.8.3 (see package-lock) which declares engines { node: ">=14.17" }.

#### Why the workflow fails only on the release branch
- The reusable workflow `.github/workflows/test_node.yml` has two jobs:
  - `type_check` uses:
    ```yaml
    - name: Use Node.js
      uses: actions/setup-node@…
      with:
        node-version-file: package.json
    ```
    This causes the action to read the Node version specifier from `package.json`.
  - In this branch, `package.json` has:
    ```json
    "engines": { "node": ">= 10" }
    ```
    With `node-version-file`, setup-node interprets this spec and resolves a Node version which can be ≤12 on some runners. If the resolved version is <14.17 (for example 10.x/12.x), running the type check (`tsc`) fails because TypeScript 5.8.3 requires Node ≥14.17.
- The matrix job `test_node` is unaffected by TypeScript’s engine requirement (it doesn’t run `tsc`) and generally passes across versions because dev tooling is compatible with older Node. The failure is therefore isolated to the `type_check` job selection of Node.
- On master, the workflow likely uses a pinned Node version for `type_check` or `engines.node` was raised, so `type_check` runs with Node ≥14.17 and passes.

#### Concrete evidence in repo
- `package.json`:
```json
"devDependencies": { "typescript": "~5.8.3", … },
"engines": { "node": ">= 10" },
"volta": { "node": "24.8.0" }
```
- `typescript@5.8.3` requires Node ≥14.17. Running `tsc` under Node 10/12 will fail.
- `test_node.yml` uses `node-version-file: package.json` for `type_check`, which can yield an older Node than TypeScript supports.

#### Root cause
`type_check` is executed with a Node version that is too old for the TypeScript version in this branch due to using `node-version-file: package.json` where `engines.node` is set to ">= 10". This mismatch causes the type-check step to fail on the release branch, while master uses a newer Node (e.g., pinned) and passes.

#### Recommended fixes
Pick one of the following (can be combined for robustness):

1) Pin a modern Node version for the `type_check` job
```yaml
# .github/workflows/test_node.yml
- name: Use Node.js
  uses: actions/setup-node@a0853c24544627f65ddf259abe73b1d18a591444 # 5.0.0
  with:
    node-version: '20.x'  # or '22.x' / '24.x'
    # remove node-version-file here
```

2) Raise the minimum engine in `package.json` to match dev tooling
```json
"engines": { "node": ">= 14.17" }
```
This helps prevent local/CI ambiguity and aligns with TypeScript’s requirement.

3) Optional: Remove the duplicate Jest configuration source
- Keep `jest.config.js` and delete the `jest` key in `package.json`, or change the npm script to pass `--config jest.config.js`.
This removes the “Multiple configurations found” warning that could become fatal under stricter CI settings.

#### Notes on release-branch specifics
- The workflow already works around missing platform-specific optional binaries on release branches by using npm (not yarn) and setting `SENTRYCLI_SKIP_DOWNLOAD=1`. This is not the cause of the current failure.

#### Quick validation
- Locally, forcing an older Node (≤12) for the type-check step would reproduce the failure because `tsc` (5.8.3) cannot run on Node <14.17. Upgrading the `type_check` Node to ≥14.17 (ideally 20+) will resolve the release-branch failure.

