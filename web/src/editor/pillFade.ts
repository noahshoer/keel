/** Whether a pill still runs past the end of the row, so its fade should show. */
export function pillsRunOn(scrollLeft: number, clientWidth: number, scrollWidth: number): boolean {
  return scrollLeft + clientWidth < scrollWidth - 1;
}
