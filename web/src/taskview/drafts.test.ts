import { beforeAll, describe, expect, it } from "vitest";
import type { EditorValues } from "../bindings";
import { blankValues } from "../core";
import { loadCoreForTests } from "../test/wasm";
import { commitDrafts, pendingSaveBeforeNav, takesStoredText, typedText } from "./drafts";

beforeAll(() => {
  loadCoreForTests();
});

/** Title and description set; an Inbox task. */
const fixture = (): EditorValues => ({
  ...blankValues("P-IN", null, "2026-10-24", ["travel"]),
  content: "Pack for Boston",
  description: "Leave Friday after work",
});

describe("draft rules", () => {
  it("pending_save_detects_title_and_description", () => {
    const before = fixture();
    expect(pendingSaveBeforeNav(before, { ...before, date: "2026-10-31" })).toBe(false);
    expect(pendingSaveBeforeNav(before, { ...before, content: "Pack for NYC" })).toBe(true);
    expect(pendingSaveBeforeNav(before, { ...before, description: `${before.description}  ` })).toBe(true);
  });

  it("our_own_save_coming_back_keeps_the_draft", () => {
    // Saved "Pack", then typed "Pack bag": the reload brings back "Pack".
    expect(takesStoredText("Pack", "Old", "Pack")).toBe(false);
    // An edit from elsewhere replaces the draft.
    expect(takesStoredText("Moved", "Old", "Pack")).toBe(true);
    // Nothing changed.
    expect(takesStoredText("Old", "Old", "Old")).toBe(false);
  });

  it("a_newline_in_the_text_is_an_enter", () => {
    // iOS can deliver Enter as an inserted newline the keydown missed.
    expect(typedText("Buy milk\n", false)).toEqual({ text: "Buy milk", enter: true });
    expect(typedText("Buy\r\nmilk", false)).toEqual({ text: "Buy milk", enter: true });
    expect(typedText("Buy milk ", false)).toEqual({ text: "Buy milk ", enter: false });
  });

  it("a_multiline_field_keeps_its_line_breaks", () => {
    expect(typedText("Eggs\nMilk\n", true)).toEqual({ text: "Eggs\nMilk\n", enter: false });
    expect(typedText("Eggs\r\nMilk", true)).toEqual({ text: "Eggs\nMilk", enter: false });
  });

  it("commitDrafts trims the title and saves the change", () => {
    const base = fixture();
    const r = commitDrafts(base, "  Pack bag ", base.description);
    expect(r.title).toBe("Pack bag");
    expect(r.save?.before).toBe(base);
    expect(r.save?.after.content).toBe("Pack bag");
  });

  it("commitDrafts reverts an emptied title and saves nothing", () => {
    const base = fixture();
    expect(commitDrafts(base, "   ", base.description)).toEqual({ title: base.content, save: null });
  });

  it("commitDrafts saves nothing when the text is unchanged", () => {
    const base = fixture();
    expect(commitDrafts(base, base.content, base.description).save).toBeNull();
  });

  it("commitDrafts saves a changed description", () => {
    const base = fixture();
    const r = commitDrafts(base, base.content, "Leave Saturday");
    expect(r.save?.after.description).toBe("Leave Saturday");
  });
});
