import { expect, it, vi } from "vitest";
import { mockHaptics } from "../ipc/mock";
import { haptic } from "./haptics";

it("asks Swift for the vibration", async () => {
  haptic("complete");
  haptic("press");
  await vi.waitFor(() => expect(mockHaptics()).toEqual(["complete", "press"]));
});
