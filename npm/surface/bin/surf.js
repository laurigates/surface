#!/usr/bin/env node
"use strict";

// Thin launcher: resolve the prebuilt `surf` binary from the platform-specific package that npm
// installed as an optionalDependency, then exec it transparently. There is deliberately NO
// postinstall and NO network download — the binary ships *inside* the platform package, and npm
// installs only the one whose `os`/`cpu` matches the host (the others are skipped, which is why
// they are optional). See npm/README.md.

const path = require("path");
const { spawnSync } = require("child_process");

// Map Node's (platform, arch) to the package that carries the matching binary. Keep in lockstep
// with the targets release.yml builds and with the shim's optionalDependencies.
const PACKAGE_BY_PLATFORM = {
  "darwin-arm64": "@gradientdev/surface-darwin-arm64",
  "linux-x64": "@gradientdev/surface-linux-x64",
};

function resolveBinary() {
  const key = `${process.platform}-${process.arch}`;
  const pkg = PACKAGE_BY_PLATFORM[key];
  if (!pkg) {
    throw new Error(
      `@gradientdev/surface: no prebuilt binary for ${key}.\n` +
        `  Supported: ${Object.keys(PACKAGE_BY_PLATFORM).join(", ")}.\n` +
        `  Install from source instead: cargo install --git https://github.com/Connorrmcd6/surface surf-cli`
    );
  }
  // Resolve the platform package's own package.json (always resolvable, unlike a bare binary
  // subpath), then derive the binary beside it.
  let manifest;
  try {
    manifest = require.resolve(`${pkg}/package.json`);
  } catch {
    throw new Error(
      `@gradientdev/surface: the platform package ${pkg} is not installed.\n` +
        `  If you installed with --no-optional or --omit=optional, reinstall without it.`
    );
  }
  return path.join(path.dirname(manifest), "bin", "surf");
}

let binary;
try {
  binary = resolveBinary();
} catch (err) {
  process.stderr.write(`${err.message}\n`);
  process.exit(1);
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  process.stderr.write(`@gradientdev/surface: failed to run ${binary}: ${result.error.message}\n`);
  process.exit(1);
}
// Mirror the binary's exit code so `surf check` keeps gating CI through the shim.
process.exit(result.status === null ? 1 : result.status);
