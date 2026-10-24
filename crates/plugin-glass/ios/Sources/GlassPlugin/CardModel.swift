import Foundation
import SwiftUI
import UIKit

// Mirrors `ui::glass` (Rust). The web UI computes every value shown here.

enum CardMode: String, Codable {
  case add, edit
}

enum ChipId: String, Codable {
  case date, priority, labels, reminders, deadline, `repeat`, location
}

struct GlassSpan: Codable, Equatable {
  let start: Int
  let end: Int
  let kind: String
}

struct ChipState: Codable, Equatable {
  let id: ChipId
  let label: String
  let set: Bool
  /// "p1".."p3" (priority flags) or "due-…" (the date's due color);
  /// nil means Keel green.
  let tone: String?
  /// The date repeats (drawn with a ⟳).
  let repeats: Bool

  var color: Color { tone.map(priorityTone) ?? keelGreen }
}

/// Keel's colors for a tone, light and dark: the web's `--p1`..`--p3` flag
/// colors and `--due-*` due colors (style.rs).
func priorityTone(_ tone: String) -> Color {
  let (light, dark): (UInt32, UInt32) =
    switch tone {
    case "p1", "due-overdue": (0xd1453b, 0xff7066)
    case "p2": (0xeb8909, 0xff9a14)
    case "due-today": (0x058527, 0x25b84c)
    case "due-tomorrow": (0xad6200, 0xff9a14)
    case "due-week": (0x692fc2, 0xa970ff)
    case "due-later": (0x808080, 0x9a9a9a)
    default: (0x246fe0, 0x5297ff)
    }
  func ui(_ hex: UInt32) -> UIColor {
    UIColor(red: CGFloat(hex >> 16 & 0xff) / 255, green: CGFloat(hex >> 8 & 0xff) / 255, blue: CGFloat(hex & 0xff) / 255, alpha: 1)
  }
  return Color(UIColor { $0.userInterfaceStyle == .dark ? ui(dark) : ui(light) })
}

struct ReminderDraft: Codable, Equatable {
  var id: String?
  var minuteOffset: Int?
  var at: String?
}

struct PickProject: Codable, Equatable {
  let id: String
  let name: String
  let isInbox: Bool
  /// [id, name] pairs.
  let sections: [[String]]
}

struct PickerData: Codable, Equatable {
  let date: String
  let time: String
  let priority: String
  let `repeat`: String
  /// [value, label] pairs.
  let repeatOptions: [[String]]
  let projectId: String
  let sectionId: String?
  let projects: [PickProject]
  let labels: [String]
  let allLabels: [String]
  let deadline: String
  let reminders: [ReminderDraft]
  let reminderLabels: [String]
  let reminderPresets: [ReminderPreset]
  let hasDue: Bool
  let quickDates: [QuickDate]
  /// The calendar month shown (built in Rust's core; drawn here).
  let calendar: MonthViewData
  /// The month / year chooser's choices.
  let years: [Int]
  let months: [String]
  /// The chosen date's due color (fills the selected day).
  let selectedTone: String?
}

/// `keel_core::calendar::MonthView`.
struct MonthViewData: Codable, Equatable {
  struct Day: Codable, Equatable {
    let date: String
    let day: Int
    let inMonth: Bool
    let today: Bool
    let past: Bool
    let selected: Bool
    let `repeat`: Bool
  }
  let year: Int
  let month: Int
  let title: String
  let weekdays: [String]
  let weeks: [[Day]]
}

struct QuickDate: Codable, Equatable {
  let label: String
  let hint: String
  let date: String
  /// The date's due color, like the chip.
  let tone: String
}

/// A `[minutes, label]` pair.
struct ReminderPreset: Codable, Equatable {
  let minutes: Int
  let label: String

  init(from decoder: Decoder) throws {
    var c = try decoder.unkeyedContainer()
    minutes = try c.decode(Int.self)
    label = try c.decode(String.self)
  }

  func encode(to encoder: Encoder) throws {
    var c = encoder.unkeyedContainer()
    try c.encode(minutes)
    try c.encode(label)
  }
}

struct CardState: Codable, Equatable {
  let mode: CardMode
  let title: String
  let description: String
  /// Rust's reset number for the card; typing is sent back with it, so
  /// keystrokes from before a reset (Return, then fast typing) are dropped.
  let reset: Int
  let spans: [GlassSpan]
  let chips: [ChipState]
  let canSubmit: Bool
  let picker: PickerData
  /// Picker-only (the task view): this chip's picker shows alone, without
  /// the card or the keyboard. Each pick is saved; a single-choice picker
  /// then closes, the others stay open until dismissed.
  let onlyPicker: ChipId?
}

/// A value chosen in a picker (`ui::glass::Pick`).
enum Pick: Encodable {
  case date(date: String, time: String)
  case priority(String)
  case labels([String])
  case reminders([ReminderDraft])
  case deadline(String)
  case `repeat`(String)
  case location(projectId: String, sectionId: String?)

  private enum K: String, CodingKey {
    case type, date, time, value, names, drafts, projectId, sectionId
  }

  func encode(to encoder: Encoder) throws {
    var c = encoder.container(keyedBy: K.self)
    switch self {
    case .date(let date, let time):
      try c.encode("date", forKey: .type); try c.encode(date, forKey: .date); try c.encode(time, forKey: .time)
    case .priority(let v):
      try c.encode("priority", forKey: .type); try c.encode(v, forKey: .value)
    case .labels(let names):
      try c.encode("labels", forKey: .type); try c.encode(names, forKey: .names)
    case .reminders(let drafts):
      try c.encode("reminders", forKey: .type); try c.encode(drafts, forKey: .drafts)
    case .deadline(let date):
      try c.encode("deadline", forKey: .type); try c.encode(date, forKey: .date)
    case .repeat(let v):
      try c.encode("repeat", forKey: .type); try c.encode(v, forKey: .value)
    case .location(let p, let s):
      try c.encode("location", forKey: .type); try c.encode(p, forKey: .projectId); try c.encode(s, forKey: .sectionId)
    }
  }
}

/// What the user did in the card (`ui::glass::GlassEvent`).
enum CardEvent: Encodable {
  /// Typing, with the reset number of the state shown when it was typed.
  case title(String, reset: Int)
  case description(String, reset: Int)
  case pick(Pick)
  /// Backspace right after a highlight: that phrase becomes plain text
  /// (`at`: where it ends, UTF-16).
  case unhighlight(Int)
  /// The calendar's arrows and its month / year chooser.
  case calendarShift(Int)
  case calendarMonth(year: Int, month: Int)
  case submit
  case cancel

  private enum K: String, CodingKey { case type, text, reset, pick, at, delta, year, month }

  func encode(to encoder: Encoder) throws {
    var c = encoder.container(keyedBy: K.self)
    switch self {
    case .title(let t, let reset):
      try c.encode("title", forKey: .type); try c.encode(t, forKey: .text); try c.encode(reset, forKey: .reset)
    case .description(let t, let reset):
      try c.encode("description", forKey: .type); try c.encode(t, forKey: .text); try c.encode(reset, forKey: .reset)
    case .pick(let p): try c.encode("pick", forKey: .type); try c.encode(p, forKey: .pick)
    case .unhighlight(let at): try c.encode("unhighlight", forKey: .type); try c.encode(at, forKey: .at)
    case .calendarShift(let delta): try c.encode("calendarShift", forKey: .type); try c.encode(delta, forKey: .delta)
    case .calendarMonth(let year, let month):
      try c.encode("calendarMonth", forKey: .type); try c.encode(year, forKey: .year); try c.encode(month, forKey: .month)
    case .submit: try c.encode("submit", forKey: .type)
    case .cancel: try c.encode("cancel", forKey: .type)
    }
  }
}

/// A long-pressed row's menu (`plugin:glass|actions`).
struct ActionMenu: Codable, Equatable {
  struct Item: Codable, Equatable {
    let id: String
    let label: String
    let destructive: Bool
  }
  let target: String
  let title: String
  let items: [Item]
}

/// The sheet's top gliding from `from` to `to` on the page's settle curve,
/// cubic-bezier(.32,.72,0,1), starting at `start`.
struct SheetGlide: Equatable {
  let from: CGFloat
  let to: CGFloat
  let start: Date
  let duration: Double

  var end: Date { start.addingTimeInterval(duration) }

  func value(at date: Date) -> CGFloat {
    let t = min(1, max(0, date.timeIntervalSince(start) / duration))
    return from + (to - from) * CGFloat(UnitBezier.keel.solve(t))
  }
}

/// CSS's cubic-bezier timing function (WebKit's UnitBezier).
struct UnitBezier {
  static let keel = UnitBezier(0.32, 0.72, 0, 1)
  let ax, bx, cx, ay, by, cy: Double

  init(_ p1x: Double, _ p1y: Double, _ p2x: Double, _ p2y: Double) {
    cx = 3 * p1x
    bx = 3 * (p2x - p1x) - cx
    ax = 1 - cx - bx
    cy = 3 * p1y
    by = 3 * (p2y - p1y) - cy
    ay = 1 - cy - by
  }

  func x(_ t: Double) -> Double { ((ax * t + bx) * t + cx) * t }
  func y(_ t: Double) -> Double { ((ay * t + by) * t + cy) * t }
  func dx(_ t: Double) -> Double { (3 * ax * t + 2 * bx) * t + cx }

  /// The curve's progress at time fraction `x`.
  func solve(_ x: Double) -> Double {
    var t = x
    for _ in 0..<8 {
      let e = self.x(t) - x
      if abs(e) < 1e-6 { return y(t) }
      let d = dx(t)
      if abs(d) < 1e-6 { break }
      t -= e / d
    }
    var lo = 0.0, hi = 1.0
    t = x
    while lo < hi {
      let v = self.x(t)
      if abs(v - x) < 1e-6 { break }
      if x > v { lo = t } else { hi = t }
      t = (hi - lo) / 2 + lo
      if hi - lo < 1e-7 { break }
    }
    return y(t)
  }
}

/// A web button drawn as glass (the drawer's header buttons, the task
/// sheet's X and •••), at the spot the page drew it.
struct DrawerButton: Codable, Equatable {
  let id: String
  let symbol: String
  let x: CGFloat
  let y: CGFloat
  let w: CGFloat
  let h: CGFloat
}

/// A native text prompt (`plugin:glass|prompt`).
struct PromptState: Codable, Equatable {
  struct Field: Codable, Equatable {
    let placeholder: String
    let value: String
  }
  let id: String
  let title: String
  let message: String
  let fields: [Field]
  let confirm: String
}

final class CardModel: ObservableObject {
  @Published var actionMenu: ActionMenu?
  @Published var prompt: PromptState?
  /// The prompt's field values as typed.
  @Published var promptValues: [String] = []
  /// Set by the plugin: reports a prompt's answer (nil values = cancelled).
  var answerPrompt: (String, [String]?) -> Void = { _, _ in }
  /// When the last prompt closed: a new one presented while the old alert
  /// is still animating away is dropped by SwiftUI, so it waits.
  var promptClosedAt = Date.distantPast
  /// Set by the plugin: reports the chosen action.
  var chooseAction: (String, String) -> Void = { _, _ in }

  /// nil while the card is closed. With `onlyPicker` set, only that chip's
  /// sheet shows; Rust saves each pick, and a single-choice pick closes it.
  @Published var state: CardState?
  /// Bumped by `show`: the fields take the state's text even if the user
  /// typed something else (a fresh card, or the reset after adding).
  @Published var resetToken = 0
  @Published var description = ""
  @Published var picker: ChipId?
  @Published var buttonsVisible = false
  /// The + shows (views with an "Add task" row).
  @Published var addVisible = true
  /// In Search, the search button closes it.
  @Published var searching = false
  /// Glass stand-ins for the open drawer's header buttons.
  @Published var drawerButtons: [DrawerButton] = []
  /// The task sheet's glass X and ••• (y from the sheet's top) and where the
  /// sheet's top is on screen; they move with it.
  @Published var sheetButtons: [DrawerButton] = []
  @Published var sheetTop: CGFloat = 0
  /// While the sheet glides, its top follows the page's curve frame by frame.
  @Published var sheetGlide: SheetGlide?
  var sheetSeq = 0

  /// Window-space frames of the controls that take touches while the card is closed.
  var hitRects: [String: CGRect] = [:]

  /// Set by the plugin: delivers events to the web UI.
  var send: (CardEvent) -> Void = { _ in }
  var button: (String) -> Void = { _ in }

  var isOpen: Bool { state != nil }

  /// The last state shown: the card keeps drawing it while it slides away
  /// after `state` is cleared.
  var shown: CardState?

  func show(_ s: CardState) {
    Diag.log("CARD show")
    shown = s
    state = s
    description = s.description
    resetToken += 1
    if let only = s.onlyPicker { picker = only }
  }

  var pickerOnly: Bool { state?.onlyPicker != nil }

  /// Moves the sheet's buttons: at once, or gliding in step with the page's
  /// own animation (same start, duration and curve).
  func placeSheet(_ a: SheetArgs) {
    if let seq = a.seq {
      guard seq > sheetSeq else { return }
      sheetSeq = seq
    }
    if let buttons = a.buttons { sheetButtons = buttons }
    let now = Date()
    if a.duration > 0 {
      let from = a.from ?? sheetTopNow(now)
      let start = a.start.map { Date(timeIntervalSince1970: $0 / 1000) } ?? now
      let glide = SheetGlide(from: from, to: a.top, start: start, duration: a.duration)
      sheetGlide = glide
      DispatchQueue.main.asyncAfter(deadline: .now() + max(0, glide.end.timeIntervalSince(now)) + 0.05) {
        if self.sheetGlide == glide { self.sheetGlide = nil }
      }
    } else {
      sheetGlide = nil
    }
    sheetTop = a.top
  }

  /// The sheet's top on screen at `date` (mid-glide or at rest).
  func sheetTopNow(_ date: Date) -> CGFloat {
    sheetGlide?.value(at: date) ?? sheetTop
  }

  func update(_ s: CardState) {
    guard state != nil else { return }
    shown = s
    state = s
  }

  /// Slides the card down off screen together with the keyboard: the
  /// keyboard is dismissed first (it otherwise waits for the card to be
  /// gone), and the card leaves over the keyboard's own ~0.25 s.
  func hide() {
    Diag.log("CARD hide")
    guard state != nil else { return }
    UIApplication.shared.sendAction(#selector(UIResponder.resignFirstResponder), to: nil, from: nil, for: nil)
    withAnimation(.easeOut(duration: 0.25)) {
      state = nil
    }
    picker = nil
  }

  func cancel() {
    hide()
    send(.cancel)
  }
}

/// Keel's accent (the web `--accent`), lighter in dark mode for contrast.
let keelGreen = Color(UIColor {
  $0.userInterfaceStyle == .dark
    ? UIColor(red: 0x35 / 255, green: 0xad / 255, blue: 0x88 / 255, alpha: 1)
    : UIColor(red: 0x26 / 255, green: 0x83 / 255, blue: 0x66 / 255, alpha: 1)
})
