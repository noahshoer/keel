import UIKit
import WebKit

/// Drives the card without touches, for simulator checks:
/// `SIMCTL_CHILD_KEEL_GLASS_DEMO="<selector>|<text>|<submit 0/1>|<chip>" xcrun simctl launch …`
/// clicks `selector` in the page, types `text` into the title, submits, or
/// opens `chip`'s picker sheet. A `js:` selector runs page JavaScript first;
/// the other steps follow it.
/// Does nothing unless the variable is set.
enum Demo {
  static func run(webview: WKWebView, model: CardModel) {
    // KEEL_GLASS_ANSWER="a|b": answers the first prompt that opens (fields in order).
    if let answer = ProcessInfo.processInfo.environment["KEEL_GLASS_ANSWER"] {
      func poll(_ n: Int) {
        if let p = model.prompt {
          model.prompt = nil
          model.promptClosedAt = Date()
          model.answerPrompt(p.id, answer.components(separatedBy: "|"))
        } else if n < 60 {
          DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { poll(n + 1) }
        }
      }
      poll(0)
    }
    // KEEL_GLASS_CHOOSE="streak": chooses that item in the first action menu that opens.
    if let choice = ProcessInfo.processInfo.environment["KEEL_GLASS_CHOOSE"] {
      func poll(_ n: Int) {
        if let menu = model.actionMenu {
          model.actionMenu = nil
          model.chooseAction(menu.target, choice)
        } else if n < 60 {
          DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { poll(n + 1) }
        }
      }
      poll(0)
    }
    guard let spec = ProcessInfo.processInfo.environment["KEEL_GLASS_DEMO"] else { return }
    let parts = spec.components(separatedBy: "|")
    let selector = parts.first ?? ".add-task-row"
    let text = parts.count > 1 ? parts[1] : ""
    let submit = parts.count > 2 && parts[2] == "1"
    let chip = parts.count > 3 ? ChipId(rawValue: parts[3]) : nil
    // "priority=1": picks a priority (then submits, if asked).
    let pickPriority = parts.count > 4 && parts[4].hasPrefix("priority=") ? String(parts[4].dropFirst(9)) : nil
    DispatchQueue.main.asyncAfter(deadline: .now() + 4) {
      // "js:<code>" runs page JavaScript; "@fab", "@menu", "@search" tap a
      // floating button instead of clicking.
      if selector.hasPrefix("js:") {
        webview.evaluateJavaScript(String(selector.dropFirst(3)))
      } else if selector.hasPrefix("@") {
        model.button(String(selector.dropFirst()))
      } else {
        // "last:<selector>" clicks the last match.
        let js = selector.hasPrefix("last:")
          ? "[...document.querySelectorAll('\(selector.dropFirst(5))')].pop()?.click()"
          : "document.querySelector('\(selector)')?.click()"
        webview.evaluateJavaScript(js)
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
        if let chip { model.picker = chip }
        // KEEL_GLASS_CAL="1;2027-3": then press "next month", then choose March 2027.
        if let cal = ProcessInfo.processInfo.environment["KEEL_GLASS_CAL"] {
          let steps = cal.components(separatedBy: ";")
          DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
            if let d = Int(steps[0]) { model.send(.calendarShift(d)) }
            if steps.count > 1 {
              let ym = steps[1].components(separatedBy: "-").compactMap { Int($0) }
              DispatchQueue.main.asyncAfter(deadline: .now() + 1) { model.send(.calendarMonth(year: ym[0], month: ym[1])) }
            }
          }
        }
        if !text.isEmpty, let field = TitleField.AutoFocusField.current {
          field.text = text
          field.delegate?.textViewDidChange?(field)
          // KEEL_GLASS_BACKSPACE=1: then press backspace once (as the keyboard would).
          if ProcessInfo.processInfo.environment["KEEL_GLASS_BACKSPACE"] == "1" {
            DispatchQueue.main.asyncAfter(deadline: .now() + 1) {
              let n = (field.text as NSString).length
              let range = NSRange(location: n - 1, length: 1)
              if field.delegate?.textView?(field, shouldChangeTextIn: range, replacementText: "") ?? true {
                field.textStorage.replaceCharacters(in: range, with: "")
                field.delegate?.textViewDidChange?(field)
              }
            }
          }
        }
        // KEEL_GLASS_LABELS="a;b": toggles each label on in the open Labels
        // sheet (as its rows do), KEEL_GLASS_GAP ms apart (default 1000),
        // then closes it like Done.
        let env = ProcessInfo.processInfo.environment
        let gap = Double(env["KEEL_GLASS_GAP"].flatMap(Int.init) ?? 1000) / 1000
        if let names = env["KEEL_GLASS_LABELS"]?.components(separatedBy: ";") {
          for (i, name) in names.enumerated() {
            DispatchQueue.main.asyncAfter(deadline: .now() + 1 + Double(i) * gap) {
              model.send(.pick(.labels((model.state?.picker.labels ?? []) + [name])))
            }
          }
          DispatchQueue.main.asyncAfter(deadline: .now() + 5 + Double(names.count) * gap) { model.picker = nil }
        }
        // KEEL_GLASS_REPEAT="weekly;none": picks each repeat option, 3 s
        // apart, then closes the sheet like Done.
        if let values = env["KEEL_GLASS_REPEAT"]?.components(separatedBy: ";") {
          for (i, v) in values.enumerated() {
            DispatchQueue.main.asyncAfter(deadline: .now() + 1 + Double(i) * 3) { model.send(.pick(.repeat(v))) }
          }
          DispatchQueue.main.asyncAfter(deadline: .now() + 1 + Double(values.count) * 3) { model.picker = nil }
        }
        if let p = pickPriority {
          DispatchQueue.main.asyncAfter(deadline: .now() + 1) {
            model.send(.pick(.priority(p)))
            if submit { DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { model.send(.submit) } }
          }
          return
        }
        if submit {
          DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { model.send(.submit) }
        }
      }
    }
  }
}
