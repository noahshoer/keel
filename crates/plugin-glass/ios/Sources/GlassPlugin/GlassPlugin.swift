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

/// `enabled` switches the diagnostics log; `line` (with `source`) adds to it.
class DiagArgs: Decodable {
  let enabled: Bool?
  let source: String?
  let line: String?
  let fresh: Bool?
}

class ButtonsArgs: Decodable {
  let visible: Bool
  let add: Bool
  let searching: Bool
  let drawer: [DrawerButton]
  let floating: [DrawerButton]?
  let theme: String
}

/// The task sheet's glass X and ••• (`plugin:glass|sheet`): the web buttons
/// to draw as glass (`SheetGlass`; empty removes them). Absent = no change.
class SheetArgs: Decodable {
  let buttons: [DrawerButton]?
}

/// Draws Keel's Liquid Glass composer, editor, pickers and floating buttons
/// over the web view. The web UI drives it (`show` / `update` / `hide` /
/// `buttons`) and hears back through two events: `card` (a `GlassEvent` as
/// JSON) and `button` (which floating button was tapped). No task logic here.
class GlassPlugin: Plugin {
  let model = CardModel()
  var hosting: UIViewController?
  weak var webview: WKWebView?
  lazy var sheetGlass = PageGlass(
    webview: webview, name: "sheet", font: .system(size: 18, weight: .medium),
    fallback: "document.querySelector('.tv-sheet')?.classList.remove('glass-head')")
  lazy var drawerGlass = PageGlass(
    webview: webview, name: "drawer", font: .system(size: 18, weight: .medium),
    fallback: "document.body.classList.add('drawer-glass-off')")
  lazy var floatGlass = PageGlass(
    webview: webview, name: "float", font: .body.weight(.semibold),
    fallback: "document.querySelector('.float-btns')?.classList.remove('float-glass')")

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
      Diag.attach(webview, hosting: self.hosting?.view)
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

  /// The diagnostics log (see `Diag`): Rust turns it on or off and forwards
  /// the page's and its own lines here.
  @objc public func diag(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(DiagArgs.self)
    if args.fresh == true { Diag.startFresh() }
    if let on = args.enabled { Diag.setEnabled(on) }
    if let line = args.line { Diag.log(line, source: args.source ?? "js") }
    invoke.resolve()
  }

  @objc public func buttons(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(ButtonsArgs.self)
    DispatchQueue.main.async {
      self.applyTheme(args.theme)
      self.model.buttonsVisible = args.visible
      self.model.addVisible = args.add
      self.model.searching = args.searching
      // The drawer's theme and settings buttons ride its slide (the page
      // sends none once it has slid away).
      if args.drawer.isEmpty { self.drawerGlass.hide() } else { self.drawerGlass.show(args.drawer) }
      if let floating = args.floating {
        if floating.isEmpty { self.floatGlass.hide() } else { self.floatGlass.show(floating) }
      }
    }
    invoke.resolve()
  }

  @objc public func sheet(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(SheetArgs.self)
    DispatchQueue.main.async {
      // Only the buttons matter: the glass rides the web buttons' own layers.
      guard let buttons = args.buttons else { return }
      if buttons.isEmpty { self.sheetGlass.hide() } else { self.sheetGlass.show(buttons) }
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
