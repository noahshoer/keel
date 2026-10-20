import SwiftRs
import Tauri
import UIKit
import WebKit

/// Draws Keel's Liquid Glass composer, editor, pickers and floating buttons
/// over the web view. The web UI drives it; this side holds no task logic.
class GlassPlugin: Plugin {
  @objc public func show(_ invoke: Invoke) throws { invoke.resolve() }
  @objc public func update(_ invoke: Invoke) throws { invoke.resolve() }
  @objc public func hide(_ invoke: Invoke) throws { invoke.resolve() }
  @objc public func buttons(_ invoke: Invoke) throws { invoke.resolve() }
}

@_cdecl("init_plugin_glass")
public func initPlugin() -> Plugin {
  return GlassPlugin()
}
