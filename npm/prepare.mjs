#!/usr/bin/env node
// Stamp the release version across the shim and the per-platform packages before publishing.
// The committed package.json files carry a 0.0.0 placeholder; CI runs `node npm/prepare.mjs
// <version>` (version = the git tag without its leading `v`) to set:
//   - the version of the shim and every platform package, and
//   - the shim's optionalDependencies pins, which must match exactly so npm installs the
//     platform package built from the *same* release.
// Keeps everything in one place so a target can't drift to a stale version.

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+/.test(version)) {
  console.error(`usage: node npm/prepare.mjs <version>  (got: ${version ?? "<none>"})`);
  process.exit(1);
}

const here = dirname(fileURLToPath(import.meta.url));
const shim = "surface";
const platformDirs = ["surface-darwin-arm64", "surface-linux-x64"];

function patch(dir, fn) {
  const file = join(here, dir, "package.json");
  const pkg = JSON.parse(readFileSync(file, "utf8"));
  fn(pkg);
  writeFileSync(file, `${JSON.stringify(pkg, null, 2)}\n`);
  console.log(`  ${dir}/package.json -> ${pkg.version}`);
}

for (const dir of platformDirs) {
  patch(dir, (pkg) => {
    pkg.version = version;
  });
}

patch(shim, (pkg) => {
  pkg.version = version;
  for (const dep of Object.keys(pkg.optionalDependencies ?? {})) {
    pkg.optionalDependencies[dep] = version;
  }
});

console.log(`stamped npm packages to ${version}`);
