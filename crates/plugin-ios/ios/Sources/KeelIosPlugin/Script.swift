import WebKit

/// Runs page JavaScript without touches, for simulator checks
/// (`scripts/sim-check.sh`): `SIMCTL_CHILD_KEEL_SCRIPT="<js>" xcrun simctl
/// launch …` evaluates `<js>` in the page 4 s after launch. Does nothing
/// unless the variable is set.
enum Script {
  static func run(_ webview: WKWebView) {
    guard let code = ProcessInfo.processInfo.environment["KEEL_SCRIPT"], !code.isEmpty else { return }
    DispatchQueue.main.asyncAfter(deadline: .now() + 4) { [weak webview] in
      webview?.evaluateJavaScript(code)
    }
  }
}
