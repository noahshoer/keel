import SwiftUI
import UIKit

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
          case .date: DateSheet(picker: p, pick: send, send: { model.send($0) })
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
  /// For the calendar's month navigation (handled in Rust).
  let send: (CardEvent) -> Void

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
              Text(q.label).foregroundStyle(priorityTone(q.tone))
              Spacer()
              Text(q.hint).foregroundStyle(Color.secondary)
              if picker.date == q.date { Image(systemName: "checkmark").foregroundStyle(priorityTone(q.tone)) }
            }
          }
        }
        if !picker.date.isEmpty || !picker.time.isEmpty {
          Button("No date", role: .destructive) { pick(.date(date: "", time: "")) }
        }
      }
      Section {
        MonthCalendar(picker: picker, send: send) { day in
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

/// A month grid like Todoist's, drawn from Rust's `calendar::month_view`
/// (no date math here): the chosen day filled in its due color, today in
/// green, repeat days ringed. The title opens a month / year chooser; the
/// arrows and the chooser ask Rust for the month.
struct MonthCalendar: View {
  let picker: PickerData
  let send: (CardEvent) -> Void
  let pick: (String) -> Void
  // KEEL_GLASS_WHEEL (simulator demo only): open with the wheels showing.
  @State private var choosing = ProcessInfo.processInfo.environment["KEEL_GLASS_WHEEL"] != nil

  var body: some View {
    let cal = picker.calendar
    VStack(spacing: 10) {
      HStack {
        Button { withAnimation(.snappy) { choosing.toggle() } } label: {
          HStack(spacing: 4) {
            Text(cal.title).font(.headline).foregroundStyle(choosing ? keelGreen : Color.primary)
            Image(systemName: "chevron.right").font(.caption.weight(.bold))
              .rotationEffect(.degrees(choosing ? 90 : 0))
          }
        }
        .buttonStyle(.borderless)
        .accessibilityLabel("Choose month and year")
        Spacer()
        Button { send(.calendarShift(-1)) } label: { Image(systemName: "chevron.left") }.buttonStyle(.borderless)
          .accessibilityLabel("Previous month")
        Button { send(.calendarShift(1)) } label: { Image(systemName: "chevron.right") }.buttonStyle(.borderless)
          .padding(.leading, 18).accessibilityLabel("Next month")
      }
      .tint(keelGreen)
      if choosing {
        wheels(cal)
      } else {
        // An eager Grid, not a LazyVGrid: a lazy grid inside a List row
        // re-checks which cells are visible on every scroll offset, which
        // dropped most frames while the sheet scrolled.
        Grid(horizontalSpacing: 0, verticalSpacing: 6) {
          GridRow {
            ForEach(cal.weekdays.indices, id: \.self) { i in
              Text(cal.weekdays[i]).font(.caption.weight(.semibold)).foregroundStyle(Color.secondary)
                .frame(maxWidth: .infinity)
            }
          }
          // Rows with none of the month's days (a sixth week) are left out.
          ForEach(cal.weeks.filter { $0.contains(where: \.inMonth) }, id: \.first!.date) { week in
            GridRow {
              ForEach(week, id: \.date) { cell in
                if cell.inMonth { dayCell(cell) } else { Color.clear.frame(height: 38) }
              }
            }
          }
        }
      }
    }
    .padding(.vertical, 4)
  }

  /// Month and year wheels, like iOS Calendar's (they stand in for the
  /// grid while open). Each turn asks Rust for that month; the choices come
  /// from Rust too.
  private func wheels(_ cal: MonthViewData) -> some View {
    HStack(spacing: 0) {
      WheelColumn(titles: picker.months, selected: cal.month - 1, loops: true) { i in
        send(.calendarMonth(year: cal.year, month: i + 1))
      }
      .frame(maxWidth: .infinity)
      // String(_:) so a year doesn't get a thousands separator.
      WheelColumn(titles: picker.years.map { String($0) }, selected: picker.years.firstIndex(of: cal.year) ?? 0) { i in
        send(.calendarMonth(year: picker.years[i], month: cal.month))
      }
      .frame(maxWidth: .infinity)
    }
    .transition(.opacity)
  }

  private func dayCell(_ cell: MonthViewData.Day) -> some View {
    let fill = priorityTone(picker.selectedTone ?? "due-today")
    return Button { pick(cell.date) } label: {
      Text("\(cell.day)")
        .font(.body.weight(cell.selected || cell.today ? .semibold : .regular))
        .foregroundStyle(cell.selected ? Color.white : cell.today ? keelGreen : cell.past ? Color.secondary : Color.primary)
        .frame(width: 38, height: 38)
        .background {
          if cell.selected {
            Circle().fill(fill)
          } else if cell.repeat {
            Circle().strokeBorder(Color.secondary, style: StrokeStyle(lineWidth: 1.5, dash: [3, 3]))
          }
        }
    }
    .buttonStyle(.plain)
    .accessibilityLabel(cell.date)
  }
}

/// One wheel column, drawn by UIPickerView (the month and year wheels both
/// use it, so they look the same). With `loops`, the wheel loops like
/// Calendar's month wheel: `titles` is the only list, and the rows just point
/// into it (`row % count`). UIPickerView asks for a row's title only as it
/// scrolls into view and reuses the few on screen, so the extra rows cost
/// nothing. After every stop it jumps back to the same title in the middle
/// (identical rows, so the jump can't be seen), so no fling reaches an end.
struct WheelColumn: UIViewRepresentable {
  let titles: [String]
  /// Index into `titles`.
  let selected: Int
  var loops = false
  let onChange: (Int) -> Void

  /// How many times a looping wheel's rows go around.
  static let laps = 1000

  func makeUIView(context: Context) -> UIPickerView {
    let wheel = UIPickerView()
    wheel.dataSource = context.coordinator
    wheel.delegate = context.coordinator
    wheel.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
    wheel.selectRow(context.coordinator.middleRow(selected), inComponent: 0, animated: false)
    return wheel
  }

  func updateUIView(_ wheel: UIPickerView, context: Context) {
    let c = context.coordinator
    let changedTitles = c.parent.titles != titles
    c.parent = self
    if changedTitles { wheel.reloadAllComponents() }
    // Moved elsewhere (the arrows), or the list changed: show the value.
    if changedTitles || c.index(wheel.selectedRow(inComponent: 0)) != selected {
      wheel.selectRow(c.middleRow(selected), inComponent: 0, animated: false)
    }
  }

  func makeCoordinator() -> Coordinator { Coordinator(self) }

  final class Coordinator: NSObject, UIPickerViewDataSource, UIPickerViewDelegate {
    var parent: WheelColumn
    init(_ parent: WheelColumn) { self.parent = parent }

    private var count: Int { max(parent.titles.count, 1) }
    func index(_ row: Int) -> Int { row % count }
    /// The row showing `index`: in the middle lap when looping.
    func middleRow(_ index: Int) -> Int { parent.loops ? count * (WheelColumn.laps / 2) + index : index }

    func numberOfComponents(in _: UIPickerView) -> Int { 1 }
    func pickerView(_: UIPickerView, numberOfRowsInComponent _: Int) -> Int {
      parent.loops ? count * WheelColumn.laps : parent.titles.count
    }
    func pickerView(_: UIPickerView, titleForRow row: Int, forComponent _: Int) -> String? {
      parent.titles.isEmpty ? nil : parent.titles[index(row)]
    }
    func pickerView(_ wheel: UIPickerView, didSelectRow row: Int, inComponent _: Int) {
      if parent.loops { wheel.selectRow(middleRow(index(row)), inComponent: 0, animated: false) }
      parent.onChange(index(row))
    }
  }
}
