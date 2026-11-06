// A view's "No tasks yet." message: it collapses and fades away when the first
// task arrives, and grows back in when the last one goes (`usePresence`).
import { usePresence } from "./presence";

export function EmptyState({ show, big, small }: { show: boolean; big: string; small: string }) {
  const state = usePresence(show);
  if (state === "out") return null;
  return (
    <div className={state === "shown" ? "presence" : `presence ${state}`}>
      <div className="presence-inner">
        <div className="empty">
          <div className="big">{big}</div>
          <div>{small}</div>
        </div>
      </div>
    </div>
  );
}
