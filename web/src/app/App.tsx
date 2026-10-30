// The app's frame and state, as the Dioxus `App` (crates/ui/src/main.rs):
// which view is showing, reloads after each change, Show completed, the
// drawer on narrow layouts, and the Search push / pop motion. Liquid Glass
// is parked, so there are no glass buttons; the header's menu button opens
// the drawer.
import { useCallback, useEffect, useRef, useState } from "react";
import { localToday } from "../clock";
import { CompletionIconContext, savedIcon } from "../lists/completion";
import { ghost, ghostGo } from "../screens/ghost";
import { MainPane } from "../screens/MainPane";
import { SearchPane } from "../screens/SearchPane";
import { Sidebar } from "../shell/Sidebar";
import { savedTheme, type Theme } from "../shell/theme";
import { useLoaded } from "./load";
import { isSearchShortcut, isTyping } from "./shortcuts";
import { type Motion, sameView, type View, viewKey, viewMotion } from "./view";

export function App() {
  const [view, setView] = useState<View>({ kind: "today" });
  const [reloadKey, setReloadKey] = useState(0);
  const [showCompleted, setShowCompleted] = useState(false);
  const [drawer, setDrawer] = useState(false);
  const [icon] = useState(savedIcon);
  const [theme, setTheme] = useState<Theme>(savedTheme);
  const { loaded, error } = useLoaded(view, showCompleted, reloadKey);
  const reload = useCallback(() => setReloadKey((k) => k + 1), []);
  // Kept across visits to the Search view.
  const [searchQuery, setSearchQuery] = useState("");

  // "/" (when not typing) or Ctrl/Cmd+K opens Search and focuses the box.
  useEffect(() => {
    const onKey = (ev: KeyboardEvent) => {
      if (!isSearchShortcut(ev, isTyping(document.activeElement))) return;
      ev.preventDefault();
      setView({ kind: "search" });
      setTimeout(() => document.querySelector<HTMLInputElement>(".search-input")?.focus(), 60);
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, []);

  // Search slides in and out: a still copy of the page being left is taken
  // as soon as the view changes (before the new one loads).
  const requested = useRef<View>(view);
  // Picking a view closes the drawer once that view's data has loaded: the
  // page swaps under the drawer, then the drawer slides away to reveal it.
  const closingFor = useRef<View | null>(null);
  useEffect(() => {
    const before = requested.current;
    requested.current = view;
    closingFor.current = view;
    const motion = viewMotion(before, view);
    if (motion !== "fade") ghost(motion);
  }, [view]);

  // Each time a different view's data lands, the page fades in again
  // (`swap` alternates the animation so it restarts).
  const [swap, setSwap] = useState<{ key: string | null; n: number; motion: Motion }>({
    key: null,
    n: 0,
    motion: "fade",
  });
  const landedView = useRef<View | null>(null);
  useEffect(() => {
    const landed = loaded?.view ?? null;
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
    const motion = before === null ? "fade" : viewMotion(before, landed);
    setSwap((s) => ({ key: viewKey(landed), n: s.n + 1, motion }));
    // After the new page renders, the left-behind copy moves off.
    if (motion !== "fade") requestAnimationFrame(() => ghostGo());
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
          loaded={loaded}
          theme={theme}
          setTheme={setTheme}
          onChanged={reload}
        />
        <MainPane
          loaded={loaded}
          showCompleted={showCompleted}
          onToggleCompleted={() => setShowCompleted((c) => !c)}
          onOpenDrawer={() => setDrawer(true)}
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
      <div className={drawer ? "app drawer-open" : "app"}>
        {drawer ? (
          // biome-ignore lint/a11y/useKeyWithClickEvents: the scrim is a pointer target; Escape isn't part of the Dioxus drawer.
          // biome-ignore lint/a11y/noStaticElementInteractions: same markup as the Dioxus UI.
          <div className="drawer-scrim" onClick={() => setDrawer(false)} />
        ) : null}
        {body}
      </div>
    </CompletionIconContext.Provider>
  );
}
