import SwiftUI

// The chip pickers. Each offers the same choices as its web counterpart in
// `crates/ui/src/pickers.rs` / `editor.rs`, and reports picks as `Pick`s in
// the editor's value formats.

private let dayFormat: DateFormatter = {
  let f = DateFormatter()
  f.locale = Locale(identifier: "en_US_POSIX")
  f.dateFormat = "yyyy-MM-dd"
  return f
}()

private let timeFormat: DateFormatter = {
  let f = DateFormatter()
  f.locale = Locale(identifier: "en_US_POSIX")
  f.dateFormat = "HH:mm"
  return f
}()

private let atFormat: DateFormatter = {
  let f = DateFormatter()
  f.locale = Locale(identifier: "en_US_POSIX")
  f.dateFormat = "yyyy-MM-dd'T'HH:mm"
  return f
}()

/// The time-of-day part of `time` ("HH:mm") on `day`.
private func combine(_ day: Date, _ time: Date) -> Date {
  let cal = Calendar.current
  let t = cal.dateComponents([.hour, .minute], from: time)
  return cal.date(bySettingHour: t.hour ?? 9, minute: t.minute ?? 0, second: 0, of: day) ?? day
}

func priorityColor(_ p: String) -> Color {
  ["1", "2", "3"].contains(p) ? priorityTone("p\(p)") : .secondary
}

/// Priority and Repeat are short lists: native menus straight off the chip.
struct ChipMenu: View {
  let chip: ChipState
  let picker: PickerData
  let pick: (Pick) -> Void

  var body: some View {
    Menu {
      switch chip.id {
      case .priority:
        ForEach(["1", "2", "3", "4"], id: \.self) { p in
          Button {
            pick(.priority(p))
          } label: {
            Label(p == "4" ? "Priority 4" : "Priority \(p)", systemImage: picker.priority == p ? "checkmark" : "flag.fill")
          }
          .tint(priorityColor(p))
        }
      default:
        ForEach(picker.repeatOptions, id: \.self) { opt in
          Button {
            pick(.repeat(opt[0]))
          } label: {
            if picker.repeat == opt[0] { Label(opt[1], systemImage: "checkmark") } else { Text(opt[1]) }
          }
        }
      }
    } label: {
      ChipLabel(chip: chip)
    }
    .glassEffect(chip.set ? .regular.tint(chip.color.opacity(0.18)).interactive() : .regular.interactive(), in: .capsule)
  }
}

/// The sheet for a chip that needs more than a menu.
struct PickerSheet: View {
  let id: ChipId
  @ObservedObject var model: CardModel

  var body: some View {
    NavigationStack {
      Group {
        if let p = model.state?.picker {
          switch id {
          case .date: DateSheet(picker: p, pick: send)
          case .deadline: DeadlineSheet(picker: p, pick: send)
          case .labels: LabelsSheet(picker: p, pick: send)
          case .reminders: RemindersSheet(picker: p, pick: send)
          case .location: LocationSheet(picker: p) { send($0); model.picker = nil }
          default: EmptyView()
          }
        }
      }
      .toolbar {
        ToolbarItem(placement: .confirmationAction) {
          Button("Done") { model.picker = nil }
        }
      }
    }
    .tint(keelGreen)
    // The date sheets open tall enough for their calendar.
    .presentationDetents(id == .date || id == .deadline ? [.large] : [.medium, .large])
  }

  func send(_ p: Pick) { model.send(.pick(p)) }
}

struct DateSheet: View {
  let picker: PickerData
  let pick: (Pick) -> Void

  var body: some View {
    let day = dayFormat.date(from: picker.date)
    let time = timeFormat.date(from: picker.time)
    ScrollViewReader { proxy in
    List {
      Section {
        ForEach(picker.quickDates, id: \.label) { q in
          Button {
            pick(.date(date: q.date, time: picker.time))
          } label: {
            HStack {
              Text(q.label).foregroundStyle(Color.primary)
              Spacer()
              Text(q.hint).foregroundStyle(Color.secondary)
              if picker.date == q.date { Image(systemName: "checkmark").foregroundStyle(keelGreen) }
            }
          }
        }
        if !picker.date.isEmpty || !picker.time.isEmpty {
          Button("No date", role: .destructive) { pick(.date(date: "", time: "")) }
        }
      }
      Section {
        MonthCalendar(selected: picker.date, repeatDays: Set(picker.repeatDays)) { day in
          pick(.date(date: day, time: picker.time))
        }
      }
      Section("Time") {
        if let time {
          DatePicker(
            "Time",
            selection: Binding(
              get: { time },
              set: { pick(.date(date: picker.date.isEmpty ? dayFormat.string(from: Date()) : picker.date, time: timeFormat.string(from: $0))) }),
            displayedComponents: .hourAndMinute
          )
          .datePickerStyle(.wheel)
          .labelsHidden()
          .id("time-wheel")
          Button("No time", role: .destructive) { pick(.date(date: picker.date, time: "")) }
        } else {
          Button("Add time") {
            pick(.date(date: picker.date.isEmpty ? dayFormat.string(from: Date()) : picker.date, time: "09:00"))
          }
        }
      }
    }
    // Adding a time puts the wheel below the fold; bring it into view.
    .onChange(of: picker.time.isEmpty) { _, empty in
      if !empty { withAnimation { proxy.scrollTo("time-wheel", anchor: .bottom) } }
    }
    }
    .navigationTitle("Date")
    .navigationBarTitleDisplayMode(.inline)
  }
}

struct DeadlineSheet: View {
  let picker: PickerData
  let pick: (Pick) -> Void

  var body: some View {
    List {
      Section {
        DatePicker(
          "Deadline",
          selection: Binding(
            get: { dayFormat.date(from: picker.deadline) ?? Date() },
            set: { pick(.deadline(dayFormat.string(from: $0))) }),
          displayedComponents: .date
        )
        .datePickerStyle(.graphical)
        .tint(keelGreen)
      }
      if !picker.deadline.isEmpty {
        Button("No deadline", role: .destructive) { pick(.deadline("")) }
      }
    }
    .navigationTitle("Deadline")
    .navigationBarTitleDisplayMode(.inline)
  }
}

struct LabelsSheet: View {
  let picker: PickerData
  let pick: (Pick) -> Void

  var body: some View {
    List {
      if picker.allLabels.isEmpty && picker.labels.isEmpty {
        Text("No labels yet. Type @name in the title to add one.").foregroundStyle(Color.secondary)
      }
      // Labels typed in the title but not saved yet show too.
      let names = picker.allLabels + picker.labels.filter { l in !picker.allLabels.contains { $0.caseInsensitiveCompare(l) == .orderedSame } }
      ForEach(names, id: \.self) { name in
        let on = picker.labels.contains { $0.caseInsensitiveCompare(name) == .orderedSame }
        Button {
          let next = on ? picker.labels.filter { $0.caseInsensitiveCompare(name) != .orderedSame } : picker.labels + [name]
          pick(.labels(next))
        } label: {
          HStack {
            Label(name, systemImage: "tag").foregroundStyle(Color.primary)
            Spacer()
            if on { Image(systemName: "checkmark").foregroundStyle(keelGreen) }
          }
        }
      }
    }
    .navigationTitle("Labels")
    .navigationBarTitleDisplayMode(.inline)
  }
}

struct RemindersSheet: View {
  let picker: PickerData
  let pick: (Pick) -> Void
  @State private var at = Calendar.current.date(byAdding: .hour, value: 1, to: Date()) ?? Date()

  func add(_ r: ReminderDraft) {
    guard !picker.reminders.contains(where: { $0.minuteOffset == r.minuteOffset && $0.at == r.at }) else { return }
    pick(.reminders(picker.reminders + [r]))
  }

  var body: some View {
    List {
      Section {
        if picker.reminders.isEmpty {
          Text("No reminders yet").foregroundStyle(Color.secondary)
        }
        ForEach(Array(picker.reminders.enumerated()), id: \.offset) { i, _ in
          HStack {
            Label(i < picker.reminderLabels.count ? picker.reminderLabels[i] : "Reminder", systemImage: "alarm")
            Spacer()
            Button(role: .destructive) {
              var next = picker.reminders
              next.remove(at: i)
              pick(.reminders(next))
            } label: {
              Image(systemName: "minus.circle.fill")
            }
            .buttonStyle(.plain)
            .foregroundStyle(.red)
            .accessibilityLabel("Remove reminder")
          }
        }
      }
      Section {
        if !picker.hasDue {
          Text("Give the task a date to use these:").foregroundStyle(Color.secondary)
        }
        ForEach(picker.reminderPresets, id: \.minutes) { preset in
          Button(preset.label) { add(ReminderDraft(id: nil, minuteOffset: preset.minutes, at: nil)) }
            .disabled(!picker.hasDue)
        }
      }
      Section("At a time") {
        DatePicker("When", selection: $at)
        Button("Add reminder") { add(ReminderDraft(id: nil, minuteOffset: nil, at: atFormat.string(from: at))) }
      }
    }
    .navigationTitle("Reminders")
    .navigationBarTitleDisplayMode(.inline)
  }
}

struct LocationSheet: View {
  let picker: PickerData
  let pick: (Pick) -> Void

  var body: some View {
    List(picker.projects, id: \.id) { p in
      row(p.name, icon: p.isInbox ? "tray" : "number", on: picker.projectId == p.id && picker.sectionId == nil) {
        pick(.location(projectId: p.id, sectionId: nil))
      }
      ForEach(p.sections, id: \.self) { s in
        row(s[1], icon: "rectangle.split.1x2", on: picker.sectionId == s[0]) {
          pick(.location(projectId: p.id, sectionId: s[0]))
        }
        .padding(.leading, 24)
      }
    }
    .navigationTitle("Project")
    .navigationBarTitleDisplayMode(.inline)
  }

  func row(_ name: String, icon: String, on: Bool, action: @escaping () -> Void) -> some View {
    Button(action: action) {
      HStack {
        Label(name, systemImage: icon).foregroundStyle(Color.primary)
        Spacer()
        if on { Image(systemName: "checkmark").foregroundStyle(keelGreen) }
      }
    }
  }
}

/// A month grid like Todoist's: the chosen day filled, today in green, and
/// every day the task repeats on ringed with a dashed circle. (The system's
/// graphical date picker can't mark days.)
struct MonthCalendar: View {
  let selected: String
  let repeatDays: Set<String>
  let pick: (String) -> Void
  @State private var month: Date

  private static let cal = Calendar.current

  init(selected: String, repeatDays: Set<String>, pick: @escaping (String) -> Void) {
    self.selected = selected
    self.repeatDays = repeatDays
    self.pick = pick
    let start = dayFormat.date(from: selected) ?? Date()
    _month = State(initialValue: Self.cal.date(from: Self.cal.dateComponents([.year, .month], from: start)) ?? start)
  }

  /// The month's days, with leading blanks so the first lands on its weekday.
  private var cells: [Date?] {
    let cal = Self.cal
    let days = cal.range(of: .day, in: .month, for: month)?.count ?? 30
    let lead = (cal.component(.weekday, from: month) - cal.firstWeekday + 7) % 7
    return Array(repeating: nil, count: lead)
      + (0..<days).compactMap { cal.date(byAdding: .day, value: $0, to: month) }
  }

  private var weekdays: [String] {
    let s = Self.cal.veryShortStandaloneWeekdaySymbols
    let f = Self.cal.firstWeekday - 1
    return Array(s[f...] + s[..<f])
  }

  var body: some View {
    let today = dayFormat.string(from: Date())
    VStack(spacing: 10) {
      HStack {
        Text(month, format: .dateTime.month(.wide).year()).font(.headline)
        Spacer()
        Button { shift(-1) } label: { Image(systemName: "chevron.left") }.buttonStyle(.borderless)
          .accessibilityLabel("Previous month")
        Button { shift(1) } label: { Image(systemName: "chevron.right") }.buttonStyle(.borderless)
          .padding(.leading, 18).accessibilityLabel("Next month")
      }
      .tint(keelGreen)
      let columns = Array(repeating: GridItem(.flexible(), spacing: 0), count: 7)
      LazyVGrid(columns: columns, spacing: 6) {
        ForEach(weekdays.indices, id: \.self) { i in
          Text(weekdays[i]).font(.caption.weight(.semibold)).foregroundStyle(Color.secondary)
        }
        ForEach(cells.indices, id: \.self) { i in
          if let date = cells[i] {
            let key = dayFormat.string(from: date)
            dayCell(date: date, key: key, today: today)
          } else {
            Color.clear.frame(height: 38)
          }
        }
      }
    }
    .padding(.vertical, 4)
  }

  private func dayCell(date: Date, key: String, today: String) -> some View {
    let isSelected = key == selected
    let isToday = key == today
    let isPast = key < today
    return Button { pick(key) } label: {
      Text("\(Self.cal.component(.day, from: date))")
        .font(.body.weight(isSelected || isToday ? .semibold : .regular))
        .foregroundStyle(isSelected ? Color.white : isToday ? keelGreen : isPast ? Color.secondary : Color.primary)
        .frame(width: 38, height: 38)
        .background {
          if isSelected {
            Circle().fill(keelGreen)
          } else if repeatDays.contains(key) {
            Circle().strokeBorder(Color.secondary, style: StrokeStyle(lineWidth: 1.5, dash: [3, 3]))
          }
        }
    }
    .buttonStyle(.plain)
    .accessibilityLabel(Text(date, format: .dateTime.weekday(.wide).month().day()))
  }

  private func shift(_ months: Int) {
    if let m = Self.cal.date(byAdding: .month, value: months, to: month) { month = m }
  }
}
