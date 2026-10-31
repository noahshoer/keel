import { describe, expect, it } from "vitest";
import {
  atEdge,
  type DragStart,
  dragFrom,
  dragOffset,
  drawerSwipe,
  EDGE_WIDTH,
  edgeBackAction,
  edgeBackCommits,
  edgeOffset,
  holdsScroll,
  keyboardExpands,
  keyboardInset,
  type Swipe,
  settleOffset,
  sheetDragAction,
  sheetSettle,
  velocity,
} from "./sheet-rules";

const swipe = (dx: number, dy: number, over: Partial<Swipe> = {}): Swipe => ({
  dx,
  dy,
  width: 390,
  blocked: false,
  inSheet: false,
  ...over,
});

const start = (scrollTop: number, over: Partial<DragStart> = {}): DragStart => ({
  scrollTop,
  blocked: false,
  inField: false,
  fieldFocused: false,
  width: 390,
  detent: "medium",
  ...over,
});

const large = (scrollTop: number): DragStart => start(scrollTop, { detent: "large" });

describe("drawer swipes", () => {
  it("swipe_right_opens_a_closed_drawer", () => {
    expect(drawerSwipe(swipe(120, 10), false)).toBe(true);
  });

  it("swipe_left_closes_an_open_drawer", () => {
    expect(drawerSwipe(swipe(-120, 10), true)).toBe(false);
  });

  it("same_direction_as_the_state_does_nothing", () => {
    expect(drawerSwipe(swipe(120, 0), true)).toBeNull();
    expect(drawerSwipe(swipe(-120, 0), false)).toBeNull();
  });

  it("short_or_mostly_vertical_moves_are_ignored", () => {
    expect(drawerSwipe(swipe(40, 0), false)).toBeNull(); // too short
    expect(drawerSwipe(swipe(100, 90), false)).toBeNull(); // a scroll, not a swipe
  });

  it("no_drawer_swipe_from_the_sheet", () => {
    expect(drawerSwipe(swipe(120, 0, { inSheet: true }), false)).toBeNull();
  });

  it("wide_layouts_and_blocked_targets_are_ignored", () => {
    expect(drawerSwipe(swipe(120, 0, { width: 1200 }), false)).toBeNull();
    expect(drawerSwipe(swipe(120, 0, { blocked: true }), false)).toBeNull();
  });
});

describe("keyboard", () => {
  it("the_keyboard_takes_a_medium_sheet_to_large", () => {
    expect(keyboardExpands("medium", 390, 328)).toBe(true);
    expect(keyboardExpands("large", 390, 328)).toBe(false); // already large
    expect(keyboardExpands("medium", 1024, 328)).toBe(false); // the desktop panel has no detents
    expect(keyboardExpands("medium", 390, 0)).toBe(false); // a hardware keyboard covers nothing
  });

  it("a_keyboard_coming_up_holds_until_the_viewport_catches_up", () => {
    // Not resized yet: the keyboard's height.
    expect(keyboardInset(874, 0, 874, 328)).toBe(328);
    // Resized: the viewport's answer, even with no hold.
    expect(keyboardInset(874, 0, 546, null)).toBe(328);
    expect(keyboardInset(874, 0, 874, null)).toBe(0);
  });

  it("a_panned_viewport_wins_over_the_keyboards_height", () => {
    // The description field: WebKit pans 192 px and the keyboard (with its
    // form bar) covers 396; the sheet ends at the keyboard, 204 up.
    expect(keyboardInset(874, 192, 478, 396)).toBe(204);
  });
});

describe("sheet drag", () => {
  it("a_downward_drag_at_the_top_moves_the_sheet", () => {
    expect(sheetDragAction(start(0), 2, 9)).toBe("drag");
    expect(sheetDragAction(large(0), 2, 9)).toBe("drag");
  });

  it("scrolled_content_scrolls_first_on_the_way_down", () => {
    expect(sheetDragAction(start(40), 0, 9)).toBe("scroll");
    expect(sheetDragAction(large(40), 0, 9)).toBe("scroll");
  });

  it("an_upward_drag_at_medium_expands_the_sheet", () => {
    // iOS: the sheet grows to its large height before the content scrolls.
    expect(sheetDragAction(start(0), 0, -9)).toBe("drag");
    expect(sheetDragAction(start(40), 0, -9)).toBe("drag"); // even over scrolled content
  });

  it("an_upward_drag_at_large_scrolls_the_content", () => {
    expect(sheetDragAction(large(0), 0, -9)).toBe("scroll");
  });

  it("a_sideways_move_is_left_alone", () => {
    // The pill row scrolls sideways, rows and circles keep their taps.
    expect(sheetDragAction(start(0), 12, 5)).toBe("ignore");
    expect(sheetDragAction(start(0), -12, -5)).toBe("ignore"); // also sideways-and-up at medium
    expect(sheetDragAction(large(0), -12, 5)).toBe("ignore");
  });

  it("nothing_is_decided_inside_the_slop", () => {
    // A jittery tap on a row, circle or pill stays a tap.
    expect(sheetDragAction(start(0), 2, 7.9)).toBe("pending");
    expect(sheetDragAction(start(0), -7.9, 3)).toBe("pending");
    expect(sheetDragAction(start(0), 0, -7.9)).toBe("pending");
    expect(sheetDragAction(start(0), 0, 8)).toBe("drag"); // past the slop
  });

  it("a_sideways_swipe_that_starts_downward_still_goes_sideways", () => {
    // The first samples look vertical but under the slop, so nothing is decided.
    expect(sheetDragAction(start(0), 1, 3)).toBe("pending");
    expect(sheetDragAction(start(0), 14, 5)).toBe("ignore");
  });

  it("a_diagonal_tie_is_not_a_drag", () => {
    expect(sheetDragAction(start(0), 10, 10)).toBe("ignore");
    expect(sheetDragAction(start(0), 10, -10)).toBe("ignore");
  });

  it("an_unfocused_field_drags_and_a_focused_one_does_not", () => {
    // "Drag from anywhere": the title and description drag the sheet until
    // they're being typed in.
    expect(sheetDragAction(start(0, { inField: true }), 0, 12)).toBe("drag");
    const focused = start(0, { inField: true, fieldFocused: true });
    expect(sheetDragAction(focused, 0, 12)).toBe("ignore");
    expect(sheetDragAction(focused, 0, -12)).toBe("ignore");
  });

  it("inputs_menus_and_the_desktop_panel_never_drag", () => {
    expect(sheetDragAction(start(0, { blocked: true }), 0, 9)).toBe("ignore"); // inside the ••• menu or a popover
    const wide = start(0, { width: 1280 });
    expect(sheetDragAction(wide, 0, 9)).toBe("ignore"); // desktop panel
    expect(sheetDragAction(wide, 0, -9)).toBe("ignore"); // desktop panel
  });

  it("a_pending_move_holds_the_scroll_only_when_it_could_drag", () => {
    // So iOS doesn't start a scroll before the drag is decided.
    expect(holdsScroll(start(0), 1, 3)).toBe(true);
    expect(holdsScroll(start(40), 1, -3)).toBe(true); // upward at medium expands
    expect(holdsScroll(start(0), 4, 3)).toBe(false); // leaning sideways: the pills may scroll
    expect(holdsScroll(start(20), 0, 3)).toBe(false); // scrolled content scrolls
    expect(holdsScroll(large(0), 0, -3)).toBe(false); // upward at large scrolls
    expect(holdsScroll(start(0, { blocked: true }), 0, 3)).toBe(false);
    expect(holdsScroll(start(0), 0, 0)).toBe(false);
  });

  it("the_sheet_follows_the_finger_but_not_above_large", () => {
    expect(dragOffset(300, 50)).toBe(350);
    expect(dragOffset(300, -120)).toBe(180);
    expect(dragOffset(300, -400)).toBe(0); // stops at the large height
    expect(dragOffset(0, -30)).toBe(0);
  });

  it("a_drag_starts_where_the_sheet_is", () => {
    expect(dragFrom("medium", null, 300)).toBe(300); // at rest at medium
    expect(dragFrom("large", null, 300)).toBe(0); // at rest at large
    // Grabbed again while still settling: from where it is right now, not
    // from the detent it was heading to.
    expect(dragFrom("medium", 140, 300)).toBe(140);
    expect(dragFrom("large", -3, 300)).toBe(0);
    expect(dragFrom("medium", null, -20)).toBe(0); // keyboard up: medium meets large
  });
});

describe("settling", () => {
  // The medium detent sits 300px below the large one in these tests.
  const M = 300;

  it("from_medium_a_drag_down_closes_past_80px_or_on_a_flick", () => {
    expect(sheetSettle("medium", M + 80, M, 0)).toBe("close"); // exactly 80px
    expect(sheetSettle("medium", M + 79, M, 0)).toBe("medium");
    expect(sheetSettle("medium", M + 40, M, 0.8)).toBe("close"); // a fast flick down
    expect(sheetSettle("medium", M + 6, M, 0.8)).toBe("medium"); // a twitch, not a flick
    expect(sheetSettle("medium", M + 120, M, -0.6)).toBe("medium"); // flung back up
  });

  it("from_medium_a_drag_up_reaches_large_past_a_third_or_on_a_flick", () => {
    expect(sheetSettle("medium", M - 100, M, 0)).toBe("large"); // a third of the way
    expect(sheetSettle("medium", M - 99, M, 0)).toBe("medium");
    expect(sheetSettle("medium", M - 40, M, -0.8)).toBe("large"); // an upward flick
    expect(sheetSettle("medium", M - 200, M, 0.8)).toBe("medium"); // flung back down
  });

  it("from_large_a_short_drag_settles_at_medium_and_a_long_one_closes", () => {
    expect(sheetSettle("large", 100, M, 0)).toBe("medium"); // a third of the way to medium
    expect(sheetSettle("large", 99, M, 0)).toBe("large"); // barely moved
    expect(sheetSettle("large", 40, M, 0.8)).toBe("medium"); // a flick: one step down
    expect(sheetSettle("large", M + 79, M, 0)).toBe("medium"); // short of medium + 80px
    expect(sheetSettle("large", M + 80, M, 0)).toBe("close"); // past medium by 80px
    expect(sheetSettle("large", 60, M, 2)).toBe("close"); // a strong flick
    expect(sheetSettle("large", 200, M, -0.8)).toBe("large"); // flung back up
  });

  it("settle_positions", () => {
    expect(settleOffset("large", M)).toBe(0);
    expect(settleOffset("medium", M)).toBe(M);
    expect(settleOffset("close", M)).toBe(Number.POSITIVE_INFINITY);
    // With the keyboard up the two heights can meet: medium is never above large.
    expect(settleOffset("medium", -20)).toBe(0);
  });
});

describe("edge back", () => {
  it("an_edge_swipe_right_starts_only_at_the_left_edge_with_a_back", () => {
    expect(edgeBackAction(true, 14, 2)).toBe("drag");
    expect(edgeBackAction(true, 5, 2)).toBe("pending"); // inside the slop
    expect(edgeBackAction(false, 14, 2)).toBe("ignore"); // not from the edge, or no Back
  });

  it("an_edge_touch_that_goes_vertical_or_left_is_not_a_back", () => {
    expect(edgeBackAction(true, 3, 14)).toBe("ignore"); // vertical: the sheet drag's
    expect(edgeBackAction(true, -14, 2)).toBe("ignore"); // leftward
    expect(edgeBackAction(true, 10, 10)).toBe("ignore"); // a diagonal tie
  });

  it("only_touches_that_begin_at_the_edge_count", () => {
    expect(atEdge(0) && atEdge(EDGE_WIDTH)).toBe(true);
    expect(atEdge(EDGE_WIDTH + 1)).toBe(false);
  });

  it("release_past_the_threshold_or_with_a_flick_goes_back", () => {
    expect(edgeBackCommits(390 * 0.4, 0, 390)).toBe(true);
    expect(edgeBackCommits(390 * 0.4 - 1, 0, 390)).toBe(false);
    expect(edgeBackCommits(40, 0.6, 390)).toBe(true); // a flick
    expect(edgeBackCommits(10, 0.9, 390)).toBe(false); // too short even when fast
    expect(edgeBackCommits(250, -0.6, 390)).toBe(false); // flung back to the left
  });

  it("the_content_follows_the_finger_to_the_right_only", () => {
    expect(edgeOffset(90, 390)).toBe(90);
    expect(edgeOffset(-20, 390)).toBe(0);
    expect(edgeOffset(500, 390)).toBe(390);
  });
});

describe("velocity", () => {
  it("is px per ms from the first sample to the last", () => {
    expect(
      velocity([
        [0, 0],
        [100, 50],
      ]),
    ).toBe(0.5);
  });

  it("is 0 with fewer than two samples or no time elapsed", () => {
    expect(velocity([[5, 0]])).toBe(0);
    expect(velocity([])).toBe(0);
    expect(
      velocity([
        [5, 0],
        [5, 40],
      ]),
    ).toBe(0);
  });
});
