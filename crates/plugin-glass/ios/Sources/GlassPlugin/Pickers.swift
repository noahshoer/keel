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
    .presentationDetents([.medium, .large])
  }

  func send(_ p: Pick) { model.send(.pick(p)) }
}

struct DateSheet: View {
  let picker: PickerData
  let pick: (Pick) -> Void

  var body: some View {
    let day = dayFormat.date(from: picker.date)
    let time = timeFormat.date(from: picker.time)
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
        DatePicker(
          "Date",
          selection: Binding(
            get: { day ?? Date() },
            set: { pick(.date(date: dayFormat.string(from: $0), time: picker.time)) }),
          displayedComponents: .date
        )
        .datePickerStyle(.graphical)
        .tint(keelGreen)
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
          Button("No time", role: .destructive) { pick(.date(date: picker.date, time: "")) }
        } else {
          Button("Add time") {
            pick(.date(date: picker.date.isEmpty ? dayFormat.string(from: Date()) : picker.date, time: "09:00"))
          }
        }
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
