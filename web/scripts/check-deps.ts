import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

export interface PackageJson {
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
}

export type Allowlist = Record<string, { reason: string }>;

const EXACT_VERSION = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$/;

/** Returns the list of violations; an empty list means the check passes. */
export function checkDeps(pkg: PackageJson, allow: Allowlist): string[] {
  const errors: string[] = [];
  const used = { ...pkg.dependencies, ...pkg.devDependencies };
  for (const [name, version] of Object.entries(used)) {
    if (!Object.hasOwn(allow, name)) {
      errors.push(`${name} is not in deps-allowlist.json`);
    }
    if (!EXACT_VERSION.test(version)) {
      errors.push(`${name} must be pinned exactly (got ${version})`);
    }
  }
  for (const name of Object.keys(allow)) {
    if (!Object.hasOwn(used, name)) {
      errors.push(`${name} is in deps-allowlist.json but not used`);
    }
  }
  return errors;
}

function main(): void {
  const read = (file: string): unknown =>
    JSON.parse(readFileSync(fileURLToPath(new URL(file, import.meta.url)), "utf8"));
  const errors = checkDeps(
    read("../package.json") as PackageJson,
    read("../deps-allowlist.json") as Allowlist,
  );
  if (errors.length > 0) {
    for (const error of errors) console.error(error);
    process.exit(1);
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
