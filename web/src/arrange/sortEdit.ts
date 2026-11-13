// The sort panel's edits: pure, each returns a new Arrangement. Which fields
// exist and what their directions are called comes from keel-core (sortChoices).
import type { Arrangement, SortChoice, SortField } from "../bindings";

function moved<T>(list: T[], from: number, to: number): T[] {
  const inRange = (i: number) => Number.isInteger(i) && i >= 0 && i < list.length;
  if (!inRange(from) || !inRange(to) || from === to) return list;
  const next = [...list];
  const [item] = next.splice(from, 1);
  next.splice(to, 0, item as T);
  return next;
}

export function addGroup(a: Arrangement, field: SortField): Arrangement {
  return a.groups.includes(field) ? a : { ...a, groups: [...a.groups, field] };
}

export function removeGroup(a: Arrangement, index: number): Arrangement {
  if (index < 0 || index >= a.groups.length) return a;
  return { ...a, groups: a.groups.filter((_, i) => i !== index) };
}

export function moveGroup(a: Arrangement, from: number, to: number): Arrangement {
  const groups = moved(a.groups, from, to);
  return groups === a.groups ? a : { ...a, groups };
}

export function addOrder(a: Arrangement, field: SortField): Arrangement {
  return a.order.some((o) => o.field === field)
    ? a
    : { ...a, order: [...a.order, { field, reversed: false }] };
}

export function removeOrder(a: Arrangement, index: number): Arrangement {
  if (index < 0 || index >= a.order.length) return a;
  return { ...a, order: a.order.filter((_, i) => i !== index) };
}

export function moveOrder(a: Arrangement, from: number, to: number): Arrangement {
  const order = moved(a.order, from, to);
  return order === a.order ? a : { ...a, order };
}

export function flipOrder(a: Arrangement, index: number): Arrangement {
  if (index < 0 || index >= a.order.length) return a;
  return { ...a, order: a.order.map((o, i) => (i === index ? { ...o, reversed: !o.reversed } : o)) };
}

export function unusedGroups(a: Arrangement, choices: SortChoice[]): SortChoice[] {
  return choices.filter((c) => !a.groups.includes(c.field));
}

export function unusedOrders(a: Arrangement, choices: SortChoice[]): SortChoice[] {
  return choices.filter((c) => !a.order.some((o) => o.field === c.field));
}

/** Clears the headings and the order; the filter stays. */
export function resetSort(a: Arrangement): Arrangement {
  return { ...a, groups: [], order: [] };
}
