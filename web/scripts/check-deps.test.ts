import assert from "node:assert/strict";
import { test } from "node:test";
import { checkDeps } from "./check-deps.ts";

test("flags a dependency missing from the allowlist", () => {
  const errors = checkDeps(
    { dependencies: { react: "19.2.0", leftpad: "1.0.0" } },
    { react: { reason: "UI" } },
  );
  assert.ok(errors.includes("leftpad is not in deps-allowlist.json"));
});

test("flags a range version", () => {
  const errors = checkDeps(
    { dependencies: { react: "^19.2.0" } },
    { react: { reason: "UI" } },
  );
  assert.ok(errors.includes("react must be pinned exactly (got ^19.2.0)"));
});

test("flags an unused allowlist entry", () => {
  const errors = checkDeps(
    { dependencies: { react: "19.2.0" } },
    { react: { reason: "UI" }, vite: { reason: "bundler" } },
  );
  assert.ok(errors.includes("vite is in deps-allowlist.json but not used"));
});
