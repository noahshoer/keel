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
  /// "p1".."p3" for the priority flag colors; nil means Keel green.
  let tone: String?

  var color: Color { tone.map(priorityTone) ?? keelGreen }
}

/// The web's `--p1`..`--p3` flag colors, light and dark.
func priorityTone(_ tone: String) -> Color {
  let (light, dark): (UInt32, UInt32) =
    switch tone {
    case "p1": (0xd1453b, 0xff7066)
    case "p2": (0xeb8909, 0xff9a14)
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
}

struct QuickDate: Codable, Equatable {
  let label: String
  let hint: String
  let date: String
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
  let spans: [GlassSpan]
  let chips: [ChipState]
  let canSubmit: Bool
  let picker: PickerData
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
  case title(String)
  case description(String)
  case pick(Pick)
  case submit
  case cancel

  private enum K: String, CodingKey { case type, text, pick }

  func encode(to encoder: Encoder) throws {
    var c = encoder.container(keyedBy: K.self)
    switch self {
    case .title(let t): try c.encode("title", forKey: .type); try c.encode(t, forKey: .text)
    case .description(let t): try c.encode("description", forKey: .type); try c.encode(t, forKey: .text)
    case .pick(let p): try c.encode("pick", forKey: .type); try c.encode(p, forKey: .pick)
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

/// One of the drawer's header buttons, at the spot the page drew it.
struct DrawerButton: Codable, Equatable {
  let id: String
  let symbol: String
  let x: CGFloat
  let y: CGFloat
  let w: CGFloat
  let h: CGFloat
}

final class CardModel: ObservableObject {
  @Published var actionMenu: ActionMenu?
  /// Set by the plugin: reports the chosen action.
  var chooseAction: (String, String) -> Void = { _, _ in }

  /// nil while the card is closed.
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

  /// Window-space frames of the controls that take touches while the card is closed.
  var hitRects: [String: CGRect] = [:]

  /// Set by the plugin: delivers events to the web UI.
  var send: (CardEvent) -> Void = { _ in }
  var button: (String) -> Void = { _ in }

  var isOpen: Bool { state != nil }

  func show(_ s: CardState) {
    state = s
    description = s.description
    resetToken += 1
  }

  func update(_ s: CardState) {
    guard state != nil else { return }
    state = s
  }

  func hide() {
    state = nil
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
