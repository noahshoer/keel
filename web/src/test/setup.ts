import { cleanup } from "@testing-library/react";
import { afterEach, beforeEach } from "vitest";
import { resetMock } from "../ipc/mock";

beforeEach(() => {
  resetMock();
});

afterEach(() => {
  cleanup();
});
