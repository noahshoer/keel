import SwiftRs
import SwiftUI
import Tauri
import UIKit
import WebKit

class StateArgs: Decodable {
  let state: CardState
}

class ActionsArgs: Decodable {
  let target: String
  let title: String
  let items: [ActionMenu.Item]
}

class PromptArgs: Decodable {
  let prompt: PromptState
}

class ButtonsArgs: Decodable {
  let visible: Bool
  let add: Bool
  let searching: Bool
  let drawer: [DrawerButton]
  let theme: String
}

/// Draws Keel's Liquid Glass composer, editor, pickers and floating buttons
/// over the web view. The web UI drives it (`show` / `update` / `hide` /
/// `buttons`) and hears back through two events: `card` (a `GlassEvent` as
/// JSON) and `button` (which floating button was tapped). No task logic here.
class GlassPlugin: Plugin {
  let model = CardModel()
  var hosting: UIViewController?
  weak var webview: WKWebView?

  public override func load(webview: WKWebView) {
    self.webview = webview
    model.send = { [weak self] event in
      guard let data = try? JSONEncoder().encode(event), let json = String(data: data, encoding: .utf8) else { return }
      self?.trigger("card", data: ["json": json])
    }
    model.chooseAction = { [weak self] target, action in
      guard let data = try? JSONSerialization.data(withJSONObject: ["target": target, "action": action]),
        let json = String(data: data, encoding: .utf8)
      else { return }
      self?.trigger("action", data: ["json": json])
    }
    model.answerPrompt = { [weak self] id, values in
      let body: [String: Any] = ["id": id, "values": values.map { $0 as Any } ?? NSNull()]
      guard let data = try? JSONSerialization.data(withJSONObject: body),
        let json = String(data: data, encoding: .utf8)
      else { return }
      self?.trigger("prompt", data: ["json": json])
    }
    model.button = { [weak self] name in
      self?.trigger("button", data: ["name": name])
    }
    DispatchQueue.main.async {
      self.hosting = attachOverlay(model: self.model, to: webview)
      Demo.run(webview: webview, model: self.model)
    }
  }

  @objc public func show(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(StateArgs.self)
    DispatchQueue.main.async { self.model.show(args.state) }
    invoke.resolve()
  }

  @objc public func update(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(StateArgs.self)
    DispatchQueue.main.async { self.model.update(args.state) }
    invoke.resolve()
  }

  @objc public func hide(_ invoke: Invoke) throws {
    DispatchQueue.main.async { self.model.hide() }
    invoke.resolve()
  }

  @objc public func actions(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(ActionsArgs.self)
    DispatchQueue.main.async {
      self.model.actionMenu = ActionMenu(target: args.target, title: args.title, items: args.items)
    }
    invoke.resolve()
  }

  @objc public func prompt(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(PromptArgs.self)
    DispatchQueue.main.async {
      let wait = max(0, 0.45 - Date().timeIntervalSince(self.model.promptClosedAt))
      DispatchQueue.main.asyncAfter(deadline: .now() + wait) {
        self.model.promptValues = args.prompt.fields.map(\.value)
        self.model.prompt = args.prompt
      }
    }
    invoke.resolve()
  }

  @objc public func buttons(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(ButtonsArgs.self)
    DispatchQueue.main.async {
      self.applyTheme(args.theme)
      self.model.buttonsVisible = args.visible
      self.model.addVisible = args.add
      self.model.searching = args.searching
      withAnimation(.easeOut(duration: 0.18)) { self.model.drawerButtons = args.drawer }
    }
    invoke.resolve()
  }
}

extension GlassPlugin {
  /// Native views (glass, the card, pickers, menus, the keyboard) follow
  /// Keel's theme instead of the phone's: the window gets an override, or
  /// none for "system".
  func applyTheme(_ theme: String) {
    let style: UIUserInterfaceStyle =
      switch theme {
      case "light": .light
      case "dark": .dark
      default: .unspecified
      }
    guard let window = webview?.window ?? hosting?.view.window else { return }
    if window.overrideUserInterfaceStyle != style { window.overrideUserInterfaceStyle = style }
  }
}

@_cdecl("init_plugin_glass")
public func initPlugin() -> Plugin {
  return GlassPlugin()
}
