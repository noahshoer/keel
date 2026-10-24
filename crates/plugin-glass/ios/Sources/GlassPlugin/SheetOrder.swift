/// Orders the task sheet's glass messages (`plugin:glass|sheet`). Each page
/// load numbers its messages from 1 under a new `epoch` (its start time), so
/// a reloaded page starts over instead of being dropped as "old"; within a
/// page, a message overtaken by a later one is dropped. Unnumbered messages
/// (Rust removing the buttons) always apply.
struct SheetOrder {
  private(set) var epoch: Double = 0
  private(set) var seq = 0

  mutating func accepts(epoch e: Double?, seq s: Int?) -> Bool {
    guard let e, let s else { return true }
    if e > epoch {
      epoch = e
      seq = s
      return true
    }
    guard e == epoch, s > seq else { return false }
    seq = s
    return true
  }
}
