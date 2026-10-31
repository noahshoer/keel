import { expect, it } from "vitest";
import { deliveryNote, permissionHint, projectNameOf, turnOn } from "./rules";

it("the switch's hint says when the system blocks reminders", () => {
  expect(permissionHint("denied", true)).toBe("Blocked by the system");
  expect(permissionHint("granted", true)).toBe("On");
  expect(permissionHint("prompt", false)).toBe("Off");
});

it("the delivery note describes only this device", () => {
  const phone = deliveryNote(true);
  expect(phone.includes("closed") && !phone.includes("computer")).toBe(true);
  const desktop = deliveryNote(false);
  expect(desktop.includes("open") && !desktop.includes("iPhone")).toBe(true);
});

it("a file's project name is its last path part without .csv", () => {
  expect(projectNameOf("Travel.csv")).toBe("Travel");
  expect(projectNameOf("C:\\exports\\Home.CSV")).toBe("Home");
  expect(projectNameOf("/tmp/a/Work")).toBe("Work");
  expect(projectNameOf("Notes.csv.csv")).toBe("Notes");
  expect(projectNameOf("a.CSV.csv")).toBe("a");
});

it("turning on asks for permission only when not granted", async () => {
  let asked = 0;
  const request = async () => {
    asked += 1;
    return "granted";
  };
  expect(await turnOn("granted", request)).toEqual({ permission: "granted", granted: true });
  expect(asked).toBe(0);
  expect(await turnOn("prompt", request)).toEqual({ permission: "granted", granted: true });
  expect(asked).toBe(1);
  expect(await turnOn("prompt", async () => "denied")).toEqual({ permission: "denied", granted: false });
  expect(
    await turnOn("prompt", async () => {
      throw new Error("no");
    }),
  ).toEqual({ permission: "denied", granted: false });
});
