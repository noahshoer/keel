import SwiftUI
import UIKit

/// Background tint behind each kind of highlighted phrase (the web `.hl-*` colors).
func highlightColor(_ kind: String) -> UIColor {
  switch kind {
  case "date", "time": return UIColor(red: 5 / 255, green: 133 / 255, blue: 39 / 255, alpha: 0.18)
  case "repeat", "deadline": return UIColor(red: 105 / 255, green: 47 / 255, blue: 194 / 255, alpha: 0.18)
  case "priority": return UIColor(red: 209 / 255, green: 69 / 255, blue: 59 / 255, alpha: 0.18)
  case "project", "section": return UIColor(red: 36 / 255, green: 111 / 255, blue: 224 / 255, alpha: 0.18)
  case "label": return UIColor(red: 235 / 255, green: 137 / 255, blue: 9 / 255, alpha: 0.2)
  default: return .clear
  }
}

/// The title, with quick-add phrases highlighted from the spans the web UI sends.
struct TitleField: UIViewRepresentable {
  @ObservedObject var model: CardModel

  /// Takes focus as soon as it's on screen, so the keyboard opens with the
  /// card (calling it from `makeUIView` was too early on the phone).
  final class AutoFocusField: UITextField {
    /// The field on screen, for `Demo`.
    static weak var current: AutoFocusField?
    override func didMoveToWindow() {
      super.didMoveToWindow()
      if window != nil { AutoFocusField.current = self }
      if window != nil { DispatchQueue.main.async { self.becomeFirstResponder() } }
    }
  }

  func makeUIView(context: Context) -> UITextField {
    let f = AutoFocusField()
    f.font = .systemFont(ofSize: 17, weight: .semibold)
    f.returnKeyType = .done
    f.delegate = context.coordinator
    f.setContentHuggingPriority(.required, for: .vertical)
    f.addTarget(context.coordinator, action: #selector(Coordinator.changed(_:)), for: .editingChanged)
    return f
  }

  func updateUIView(_ f: UITextField, context: Context) {
    guard let state = model.state else { return }
    f.placeholder = state.mode == .add ? "e.g. Call mom tomorrow 5pm p1 #Home" : "Task name"
    let c = context.coordinator
    if c.token != model.resetToken {
      c.token = model.resetToken
      f.text = state.title
    }
    // Typing got ahead of this state; the next one will match.
    guard f.text == state.title else { return }
    let text = NSMutableAttributedString(
      string: state.title, attributes: [.font: f.font!, .foregroundColor: UIColor.label])
    let length = (state.title as NSString).length
    for s in state.spans where s.start < s.end && s.end <= length {
      text.addAttribute(.backgroundColor, value: highlightColor(s.kind), range: NSRange(location: s.start, length: s.end - s.start))
    }
    let selection = f.selectedTextRange
    f.attributedText = text
    f.selectedTextRange = selection
  }

  func makeCoordinator() -> Coordinator { Coordinator(model: model) }

  final class Coordinator: NSObject, UITextFieldDelegate {
    let model: CardModel
    var token = -1
    init(model: CardModel) { self.model = model }

    @objc func changed(_ f: UITextField) {
      model.send(.title(f.text ?? ""))
    }

    func textFieldShouldReturn(_ f: UITextField) -> Bool {
      if model.state?.canSubmit == true { model.send(.submit) }
      return false
    }
  }
}

func chipIcon(_ id: ChipId) -> String {
  switch id {
  case .date: return "calendar"
  case .priority: return "flag"
  case .labels: return "tag"
  case .reminders: return "alarm"
  case .deadline: return "scope"
  case .repeat: return "repeat"
  case .location: return "number"
  }
}

struct ChipButton: View {
  let chip: ChipState
  let action: () -> Void

  var body: some View {
    Button(action: action) {
      Label(chip.label, systemImage: chipIcon(chip.id))
        .font(.subheadline.weight(chip.set ? .semibold : .regular))
        .foregroundStyle(chip.set ? keelGreen : Color.primary)
        .lineLimit(1)
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }
    .buttonStyle(.plain)
    .glassEffect(chip.set ? .regular.tint(keelGreen.opacity(0.18)).interactive() : .regular.interactive(), in: .capsule)
  }
}

/// The composer / editor: title, description, a sideways-scrolling chip row
/// and a send button pinned at its end, resting on the keyboard.
struct CardView: View {
  @ObservedObject var model: CardModel
  @State private var drag: CGFloat = 0

  var body: some View {
    if let state = model.state {
      ZStack(alignment: .bottom) {
        // Catches taps outside the card.
        Color.black.opacity(0.001)
          .ignoresSafeArea()
          .onTapGesture { model.cancel() }

        VStack(alignment: .leading, spacing: 10) {
          TitleField(model: model)
            .fixedSize(horizontal: false, vertical: true)
          TextField("Description", text: $model.description, axis: .vertical)
            .font(.subheadline)
            .foregroundStyle(.secondary)
            .lineLimit(1...4)
            .onChange(of: model.description) { _, text in
              if text != model.state?.description { model.send(.description(text)) }
            }
          HStack(spacing: 8) {
            ScrollView(.horizontal, showsIndicators: false) {
              GlassEffectContainer(spacing: 8) {
                HStack(spacing: 8) {
                  ForEach(state.chips, id: \.id) { chip in
                    ChipButton(chip: chip) { model.picker = chip.id }
                  }
                }
                .padding(.vertical, 2)
              }
            }
            .clipShape(.capsule)
            Button {
              if state.canSubmit { model.send(.submit) }
            } label: {
              Image(systemName: state.mode == .add ? "arrow.up" : "checkmark")
                .font(.body.weight(.bold))
                .foregroundStyle(.white)
                .frame(width: 40, height: 40)
            }
            .buttonStyle(.plain)
            .glassEffect(.regular.tint(keelGreen).interactive(), in: .circle)
            .opacity(state.canSubmit ? 1 : 0.4)
            .disabled(!state.canSubmit)
            .accessibilityLabel(state.mode == .add ? "Add task" : "Save")
          }
        }
        .padding(16)
        .glassEffect(in: .rect(cornerRadius: 26))
        .padding(.horizontal, 8)
        .padding(.bottom, 8)
        .offset(y: max(drag, 0))
        .gesture(
          DragGesture(minimumDistance: 12)
            .onChanged { drag = $0.translation.height }
            .onEnded { g in
              if g.translation.height > 80 { model.cancel() }
              drag = 0
            }
        )
        .animation(.spring(duration: 0.25), value: drag)
      }
    }
  }
}
