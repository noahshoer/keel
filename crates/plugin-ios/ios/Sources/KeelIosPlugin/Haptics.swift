import CoreHaptics
import UIKit

/// Keel's vibrations, played with Core Haptics, where an event's sharpness
/// is its pitch: `complete` is two quick taps, lower then higher; `press` is
/// one deep thump. The engine is made on first use and started again after
/// iOS stops or resets it. Without Core Haptics, UIKit's impact taps stand in.
enum Haptics {
  private static var engine: CHHapticEngine?
  private static var running = false

  static func play(_ kind: String) {
    guard CHHapticEngine.capabilitiesForHardware().supportsHaptics else {
      fallback(kind)
      return
    }
    do {
      let player = try ready().makePlayer(with: pattern(kind))
      try player.start(atTime: CHHapticTimeImmediate)
    } catch {
      Diag.log("haptic \(kind) failed: \(error)")
      running = false
    }
  }

  private static func ready() throws -> CHHapticEngine {
    let e: CHHapticEngine
    if let have = engine {
      e = have
    } else {
      e = try CHHapticEngine()
      e.playsHapticsOnly = true
      e.stoppedHandler = { _ in running = false }
      e.resetHandler = { running = false }
      engine = e
    }
    if !running {
      try e.start()
      running = true
    }
    return e
  }

  private static func tap(at time: TimeInterval, intensity: Float, sharpness: Float) -> CHHapticEvent {
    CHHapticEvent(
      eventType: .hapticTransient,
      parameters: [
        CHHapticEventParameter(parameterID: .hapticIntensity, value: intensity),
        CHHapticEventParameter(parameterID: .hapticSharpness, value: sharpness),
      ],
      relativeTime: time)
  }

  private static func pattern(_ kind: String) throws -> CHHapticPattern {
    let events: [CHHapticEvent]
    switch kind {
    case "press":
      events = [
        tap(at: 0, intensity: 1, sharpness: 0.1),
        CHHapticEvent(
          eventType: .hapticContinuous,
          parameters: [
            CHHapticEventParameter(parameterID: .hapticIntensity, value: 0.9),
            CHHapticEventParameter(parameterID: .hapticSharpness, value: 0.05),
          ],
          relativeTime: 0, duration: 0.09),
      ]
    default:
      events = [
        tap(at: 0, intensity: 0.75, sharpness: 0.15),
        tap(at: 0.07, intensity: 0.85, sharpness: 0.85),
      ]
    }
    return try CHHapticPattern(events: events, parameters: [])
  }

  private static func fallback(_ kind: String) {
    if kind == "press" {
      UIImpactFeedbackGenerator(style: .heavy).impactOccurred()
      return
    }
    UIImpactFeedbackGenerator(style: .light).impactOccurred()
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.07) {
      UIImpactFeedbackGenerator(style: .rigid).impactOccurred()
    }
  }
}
