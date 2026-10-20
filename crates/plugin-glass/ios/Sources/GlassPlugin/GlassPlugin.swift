import SwiftRs
import Tauri
import UIKit
import WebKit

class StateArgs: Decodable {
  let state: CardState
}

class ButtonsArgs: Decodable {
  let visible: Bool
}

/// Draws Keel's Liquid Glass composer, editor, pickers and floating buttons
/// over the web view. The web UI drives it (`show` / `update` / `hide` /
/// `buttons`) and hears back through two events: `card` (a `GlassEvent` as
/// JSON) and `button` (which floating button was tapped). No task logic here.
class GlassPlugin: Plugin {
  let model = CardModel()
  var hosting: UIViewController?

  public override func load(webview: WKWebView) {
    model.send = { [weak self] event in
      guard let data = try? JSONEncoder().encode(event), let json = String(data: data, encoding: .utf8) else { return }
      self?.trigger("card", data: ["json": json])
    }
    model.button = { [weak self] name in
      self?.trigger("button", data: ["name": name])
    }
    DispatchQueue.main.async {
      self.hosting = attachOverlay(model: self.model, to: webview)
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

  @objc public func buttons(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(ButtonsArgs.self)
    DispatchQueue.main.async { self.model.buttonsVisible = args.visible }
    invoke.resolve()
  }
}

@_cdecl("init_plugin_glass")
public func initPlugin() -> Plugin {
  return GlassPlugin()
}
