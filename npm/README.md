# npm distribution

The npm channel for Surface, following the **shim + per-platform `optionalDependencies`** pattern
(as used by esbuild, swc, biome). There is **no `postinstall` downloader** — the binary ships
inside the platform package, and npm installs only the one matching the host.

## Layout

| Package | Role |
| --- | --- |
| `@gradientdev/surface` (`surface/`) | Thin shim. Its `bin/surf.js` launcher resolves the platform package's binary at runtime and execs it. Lists the platform packages as `optionalDependencies`. |
| `@gradientdev/surface-darwin-arm64` | The `surf` binary for macOS (Apple Silicon). `os`/`cpu` pin it so npm installs it only on a match. |
| `@gradientdev/surface-linux-x64` | The `surf` binary for Linux (x86_64). |

When a user runs `npm install @gradientdev/surface`, npm pulls the shim and—via the `os`/`cpu`
gates on the optional deps—exactly one platform package. The other platform packages are skipped
without error (that's why they're *optional*). `npx @gradientdev/surface check` then runs the
real binary.

The platform list is kept in lockstep across three places: the targets `release.yml` builds, the
shim's `optionalDependencies`, and `PACKAGE_BY_PLATFORM` in `surface/bin/surf.js`. (Windows and
Intel macOS are unsupported — the launcher errors with a build-from-source pointer.)

## Versioning & publishing

The committed `package.json` files carry a `0.0.0` placeholder. On each `vX.Y.Z` tag, `release.yml`:

1. `node npm/prepare.mjs X.Y.Z` — stamps the version on every package and pins the shim's
   `optionalDependencies` to the same version (so the shim can't resolve a stale platform build).
2. Downloads the release tarballs and extracts each `surf` binary into the matching
   `surface-<platform>/bin/`.
3. `npm publish --access public` for each platform package, then the shim.

Publishing requires the `NPM_TOKEN` repo secret and ownership of the `@gradientdev` npm org.

## Local smoke test

```sh
node npm/prepare.mjs 0.0.1
mkdir -p npm/surface-darwin-arm64/bin
cp target/release/surf npm/surface-darwin-arm64/bin/surf   # your host's binary
npm pack ./npm/surface-darwin-arm64 && npm pack ./npm/surface   # inspect the tarballs
```
