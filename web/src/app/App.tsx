// The app's frame and state, as the Dioxus `App` (crates/ui/src/main.rs):
// which view is showing, reloads after each change, Show completed, the
// drawer on narrow layouts (swipes open and close it), the Search push / pop
// motion, and the task view's sheet. Liquid Glass is parked, so there are no
// glass buttons; the header's menu button opens the drawer.
import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { localToday } from "../clock";
import { installDrag } from "../gestures/drag";
import { installLongPress } from "../gestures/long-press";
import { installSheet } from "../gestures/sheet";
import { searchBackSwipe, searchSwipe } from "../gestures/sheet-rules";
import { installSwipe } from "../gestures/swipe";
import { CompletionIconContext, savedIcon } from "../lists/completion";
import { ShowSubtasksContext, savedShowSubtasks } from "../lists/showSubtasks";
import { ghost, ghostGo } from "../screens/ghost";
import { MainPane } from "../screens/MainPane";
import { SearchPane } from "../screens/SearchPane";
import { clearPush, installSearchDrag } from "../screens/searchDrag";
import { clearDrag, installDrawerDrag } from "../shell/drawerDrag";
import { Sidebar } from "../shell/Sidebar";
import { applyTheme, savedTheme, type Theme } from "../shell/theme";
import { installUnderPage } from "../shell/underPage";
import { TaskNavProvider } from "../taskview/nav";
import { useTaskNav } from "../taskview/nav-context";
import { prune as pruned, top } from "../taskview/stack";
import { TaskSheet } from "../taskview/TaskSheet";
import { watchDayChange } from "./dayChange";
import { useHeldNames } from "./heldNames";
import { type Loaded, useLoaded } from "./load";
import { holdPage } from "./pageHold";
import { isSearchShortcut, isTyping } from "./shortcuts";
import { type Motion, sameView, type View, viewKey, viewMotion } from "./view";

export function App() {
  const [view, setView] = useState<View>({ kind: "today" });
  const [reloadKey, setReloadKey] = useState(0);
  const [showCompleted, setShowCompleted] = useState(false);
  const [drawer, setDrawer] = useState(false);
  const [icon, setIcon] = useState(savedIcon);
  const [showSubtasks, setShowSubtasks] = useState(savedShowSubtasks);
  const subtasks = useMemo(() => ({ show: showSubtasks, setShow: setShowSubtasks }), [showSubtasks]);
  const [theme, setTheme] = useState<Theme>(savedTheme);
  // Applied and remembered on every change (main.tsx applies the saved one before the first paint).
  // iOS paints past the page's end (under the keyboard's corners) in the list's color.
  const underPage = useRef<ReturnType<typeof installUnderPage> | null>(null);
  // The same for Search dragged in: "drag" while Search is being brought in, "cancel" while the page it came from is wanted back.
  const pushDrag = useRef<"drag" | "cancel" | null>(null);
  useEffect(() => {
    underPage.current = installUnderPage(document, window);
    return underPage.current.uninstall;
  }, []);
  useEffect(() => {
    applyTheme(theme);
    underPage.current?.sync();
  }, [theme]);
  const { loaded, error } = useLoaded(view, showCompleted, reloadKey);
  const reload = useCallback(() => setReloadKey((k) => k + 1), []);
  // Today and Upcoming depend on the date: reload when it rolls over.
  useEffect(() => watchDayChange(reload), [reload]);
  // Kept across visits to the Search view.
  const [searchQuery, setSearchQuery] = useState("");
  // Where the Search screen's Back goes.
  const beforeSearch = useRef(view);
  if (view.kind !== "search") beforeSearch.current = view;

  // Touch: a long press shows a row's actions.
  useEffect(() => installLongPress(document), []);
  useEffect(() => installDrag(document, window), []);
  // Touch: the drawer follows a drag (shell/drawerDrag.ts), and a quick swipe
  // left, with the drawer shut, opens Search on a page that has the button; a
  // swipe right on Search goes back. Installed once, so the handlers read the
  // drawer and the view through refs.
  const drawerNow = useRef(drawer);
  drawerNow.current = drawer;
  const viewNow = useRef(view);
  viewNow.current = view;
  const openSearchNow = useRef<() => void>(() => {});
  const leaveSearchNow = useRef<() => void>(() => {});
  leaveSearchNow.current = () => setView(beforeSearch.current);
  // While the Search screen is dragged back: "drag" until the page behind has landed, "cancel" while it is wanted back.
  const panGhost = useRef<"drag" | "cancel" | null>(null);
  useEffect(
    () =>
      installSearchDrag(document, {
        isSearch: () => viewNow.current.kind === "search",
        drawerOpen: () => drawerNow.current,
        leave: () => {
          panGhost.current = "drag";
          setView(beforeSearch.current);
        },
        cancel: () => {
          panGhost.current = "cancel";
          // Search was never left on screen: when it lands again it doesn't slide in.
          if (landedView.current?.kind !== "search") renamedTo.current = { kind: "search" };
          setView({ kind: "search" });
        },
        push: () => {
          pushDrag.current = "drag";
          setView({ kind: "search" });
        },
        pushed: () => {
          pushDrag.current = null;
          document.querySelector<HTMLInputElement>(".search-input")?.focus();
        },
        pushCancel: () => {
          pushDrag.current = "cancel";
          // The page it came from was never left on screen: when it lands again it doesn't slide in.
          if (landedView.current?.kind === "search") renamedTo.current = beforeSearch.current;
          setView(beforeSearch.current);
        },
      }),
    [],
  );
  useEffect(
    () =>
      installDrawerDrag(document, {
        isOpen: () => drawerNow.current,
        canOpen: () => viewNow.current.kind !== "search",
        setOpen: setDrawer,
      }),
    [],
  );
  useEffect(
    () =>
      installSwipe(document, (s) => {
        const onSearch = viewNow.current.kind === "search";
        const reduced = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
        if (reduced && searchSwipe(s, drawerNow.current, !onSearch)) {
          openSearchNow.current();
          return;
        }
        // With motion the Search screen follows the finger in and out (screens/searchDrag.ts); these are the reduced-motion forms.
        if (reduced && !drawerNow.current && searchBackSwipe(s, onSearch)) {
          leaveSearchNow.current();
          return;
        }
      }),
    [],
  );

  // Opens Search and focuses the box: the screen's, or the sidebar's on
  // desktop, where the screen has none.
  const openSearch = useCallback(() => {
    setView({ kind: "search" });
    setTimeout(
      () =>
        document
          .querySelector<HTMLInputElement>(".search-input, .sidebar-search-input:not([readonly])")
          ?.focus(),
      60,
    );
  }, []);
  openSearchNow.current = openSearch;
  // "/" (when not typing) or Ctrl/Cmd+K does too.
  useEffect(() => {
    const onKey = (ev: KeyboardEvent) => {
      if (!isSearchShortcut(ev, isTyping(document.activeElement))) return;
      ev.preventDefault();
      openSearch();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [openSearch]);

  // Search slides in and out: a still copy of the page being left is taken
  // as soon as the view changes (before the new one loads).
  const requested = useRef<View>(view);
  // Picking a view closes the drawer once that view's data has loaded: the
  // page swaps under the drawer, then the drawer slides away to reveal it.
  const closingFor = useRef<View | null>(null);
  useEffect(() => {
    const before = requested.current;
    // Nothing was picked at startup, so the first load closes nothing.
    if (before === view) return;
    requested.current = view;
    closingFor.current = view;
    const motion = viewMotion(before, view);
    if (motion !== "fade" && panGhost.current === null) ghost(motion);
  }, [view]);

  // Each time a different view's data lands, the page fades in again
  // (`swap` alternates the animation so it restarts).
  const [swap, setSwap] = useState<{ key: string | null; n: number; motion: Motion }>({
    key: null,
    n: 0,
    motion: "fade",
  });
  const held = useHeldNames(loaded);
  const { holdName } = held;
  const landedView = useRef<View | null>(null);
  // The open view under its new name is the same page: it doesn't fade in again.
  const renamedTo = useRef<View | null>(null);
  const renameView = (v: View) => {
    renamedTo.current = v;
    setView(v);
  };
  useEffect(() => {
    const landed = loaded?.view ?? null;
    if (landed !== null) {
      // A dragged Search screen: its copy goes once the page it is wanted back as has landed.
      if (panGhost.current === "cancel" && landed.kind === "search") {
        panGhost.current = null;
        requestAnimationFrame(() => {
          for (const g of document.querySelectorAll(".view-ghost")) g.remove();
        });
      } else if (panGhost.current === "drag" && landed.kind !== "search") panGhost.current = null;
      if (pushDrag.current === "cancel" && landed.kind !== "search") {
        pushDrag.current = null;
        requestAnimationFrame(() => clearPush());
      }
    }
    if (error !== null || landed !== null) {
      const target = closingFor.current;
      if (target !== null && (error !== null || (landed !== null && sameView(target, landed)))) {
        closingFor.current = null;
        setDrawer(false);
      }
    }
    if (landed === null || (landedView.current !== null && sameView(landedView.current, landed))) return;
    const before = landedView.current;
    landedView.current = landed;
    if (renamedTo.current !== null && sameView(renamedTo.current, landed)) {
      renamedTo.current = null;
      return;
    }
    // Search dragged in is placed by the drag as it lands: no slide of its own.
    const pushing = pushDrag.current === "drag" && landed.kind === "search";
    const motion = before === null ? "fade" : pushing ? "still" : viewMotion(before, landed);
    setSwap((s) => ({ key: viewKey(landed), n: s.n + 1, motion }));
    // After the new page renders, the left-behind copy moves off.
    if (motion !== "fade" && motion !== "still") requestAnimationFrame(() => ghostGo());
  }, [loaded, error]);

  let body: React.ReactNode;
  if (error !== null) {
    body = (
      <main className="main">
        <div className="error">{`Error: ${error}`}</div>
      </main>
    );
  } else if (loaded === null) {
    body = (
      <main className="main">
        <div className="content">
          <p className="view-sub">Loading…</p>
        </div>
      </main>
    );
  } else {
    body = (
      <>
        <Sidebar
          view={view}
          setView={setView}
          renameView={renameView}
          holdName={holdName}
          loaded={held.sidebar ?? loaded}
          theme={theme}
          setTheme={setTheme}
          icon={icon}
          setIcon={setIcon}
          searchQuery={searchQuery}
          setSearchQuery={setSearchQuery}
          onChanged={reload}
        />
        <TaskSheet loaded={loaded} onChanged={reload} />
        <MainPane
          loaded={held.page ?? loaded}
          showCompleted={showCompleted}
          onToggleCompleted={() => setShowCompleted((c) => !c)}
          onOpenDrawer={() => setDrawer(true)}
          onLeaveSearch={() => setView(beforeSearch.current)}
          onOpenSearch={openSearch}
          setView={setView}
          renameView={renameView}
          holdName={holdName}
          motion={swap.motion}
          swap={swap.n}
          onChanged={reload}
        >
          {loaded.view.kind === "search" ? (
            <SearchPane
              query={searchQuery}
              setQuery={setSearchQuery}
              today={localToday()}
              showCompleted={showCompleted}
              loaded={loaded}
              reloadKey={reloadKey}
              onChanged={reload}
            />
          ) : null}
        </MainPane>
      </>
    );
  }

  return (
    <CompletionIconContext.Provider value={icon}>
      <ShowSubtasksContext.Provider value={subtasks}>
        <TaskNavProvider all={loaded?.all ?? []}>
          <Frame loaded={loaded} drawer={drawer} onCloseDrawer={() => setDrawer(false)}>
            {body}
          </Frame>
        </TaskNavProvider>
      </ShowSubtasksContext.Provider>
    </CompletionIconContext.Provider>
  );
}

/** The `.app` element, inside the task view's nav: its classes, the drawer scrim, and the sheet's listeners. */
function Frame({
  loaded,
  drawer,
  onCloseDrawer,
  children,
}: {
  loaded: Loaded | null;
  drawer: boolean;
  onCloseDrawer: () => void;
  children: ReactNode;
}) {
  const nav = useTaskNav();
  // Touch: the task sheet follows the finger between its two heights and
  // closes past a threshold (through the view, which saves first). Installed
  // once: removing it would drop a slide-close still under way.
  const askClose = useRef(nav.askClose);
  askClose.current = nav.askClose;
  useEffect(() => installSheet(document, window, () => askClose.current()), []);
  // A deleted task's view closes, and every view above it.
  const { prune } = nav;
  useEffect(() => {
    if (loaded !== null) prune(loaded.all);
  }, [loaded, prune]);

  // Open while a task on the stack still exists, judged from the stack as
  // pruned against this load (the same pass the sheet draws from).
  const viewOpen = loaded !== null && top(pruned(nav.stack, loaded.all)) !== null;
  useEffect(() => (viewOpen ? holdPage() : undefined), [viewOpen]);
  useEffect(() => (drawer ? holdPage() : undefined), [drawer]);
  // Once it is shut, the next scrim fades in as usual (see drawerDrag.ts).
  const appEl = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const app = appEl.current;
    if (drawer || !app) return;
    delete app.dataset["drawerRest"];
    clearDrag(app); // a drag that shut it waited for this
  }, [drawer]);
  return (
    <div ref={appEl} className={`app${drawer ? " drawer-open" : ""}${viewOpen ? " view-open" : ""}`}>
      {drawer ? (
        // biome-ignore lint/a11y/useKeyWithClickEvents: the scrim is a pointer target; Escape isn't part of the Dioxus drawer.
        // biome-ignore lint/a11y/noStaticElementInteractions: same markup as the Dioxus UI.
        <div className="drawer-scrim" onClick={onCloseDrawer} />
      ) : null}
      {children}
    </div>
  );
}
