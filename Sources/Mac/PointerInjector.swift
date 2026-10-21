import ApplicationServices
import AppKit
import CoreGraphics
import Foundation

final class PointerInjector {
    private let source: CGEventSource?
    private var cachedAccessibilityTrusted = AXIsProcessTrusted()
    private var heldButtons = Set<ControllerPointerButton>()

    init() {
        source = CGEventSource(stateID: .hidSystemState)
        source?.localEventsSuppressionInterval = 0
    }

    var isAccessibilityTrusted: Bool {
        cachedAccessibilityTrusted
    }

    @discardableResult
    func refreshAccessibilityStatus() -> Bool {
        cachedAccessibilityTrusted = AXIsProcessTrusted()
        return cachedAccessibilityTrusted
    }

    func moveBy(deltaX: Double, deltaY: Double) {
        guard cachedAccessibilityTrusted, deltaX.isFinite, deltaY.isFinite,
              abs(deltaX) >= 0.01 || abs(deltaY) >= 0.01
        else { return }

        let current = currentMouseLocation()
        let target = clampedMouseLocation(
            CGPoint(
                x: current.x + CGFloat(deltaX),
                y: current.y + CGFloat(deltaY)
            )
        )

        let button: ControllerPointerButton? = heldButtons.contains(.left) ? .left : heldButtons.contains(.right) ? .right : heldButtons.contains(.middle) ? .middle : nil
        let type = Self.movementEventType(heldButtons: heldButtons)
        let event = CGEvent(
            mouseEventSource: source,
            mouseType: type,
            mouseCursorPosition: target,
            mouseButton: button.map(cgMouseButton(for:)) ?? .left
        )
        event?.setIntegerValueField(.mouseEventDeltaX, value: Int64(deltaX.rounded()))
        event?.setIntegerValueField(.mouseEventDeltaY, value: Int64(deltaY.rounded()))
        event?.post(tap: .cghidEventTap)
    }

    func scrollBy(deltaX: Double, deltaY: Double) {
        guard cachedAccessibilityTrusted,
              abs(deltaX) >= 0.01 || abs(deltaY) >= 0.01
        else { return }

        let vertical = Self.wheelDelta(from: -deltaY)
        let horizontal = Self.wheelDelta(from: -deltaX)
        guard vertical != 0 || horizontal != 0 else { return }

        CGEvent(
            scrollWheelEvent2Source: source,
            units: .pixel,
            wheelCount: 2,
            wheel1: vertical,
            wheel2: horizontal,
            wheel3: 0
        )?.post(tap: .cghidEventTap)
    }

    func setButton(_ button: ControllerPointerButton, pressed: Bool) {
        if pressed { heldButtons.insert(button) } else { heldButtons.remove(button) }
        guard cachedAccessibilityTrusted else { return }
        let mouseButton = cgMouseButton(for: button)
        guard let mouseType = mouseEventType(for: button, pressed: pressed) else { return }
        let location = currentMouseLocation()

        let event = CGEvent(
            mouseEventSource: source,
            mouseType: mouseType,
            mouseCursorPosition: location,
            mouseButton: mouseButton
        )
        if button == .middle {
            event?.setIntegerValueField(.mouseEventButtonNumber, value: 2)
        }
        event?.post(tap: .cghidEventTap)
    }

    func click(_ button: ControllerPointerButton) {
        setButton(button, pressed: true)
        setButton(button, pressed: false)
    }

    static func movementEventType(heldButtons: Set<ControllerPointerButton>) -> CGEventType {
        if heldButtons.contains(.left) { return .leftMouseDragged }
        if heldButtons.contains(.right) { return .rightMouseDragged }
        if heldButtons.contains(.middle) { return .otherMouseDragged }
        return .mouseMoved
    }

    private func currentMouseLocation() -> CGPoint {
        CGEvent(source: source)?.location ?? .zero
    }

    private func clampedMouseLocation(_ point: CGPoint) -> CGPoint {
        let bounds = activeDisplayBounds()
        guard !bounds.isNull, !bounds.isEmpty else { return point }
        return CGPoint(
            x: min(max(point.x, bounds.minX), bounds.maxX - 1),
            y: min(max(point.y, bounds.minY), bounds.maxY - 1)
        )
    }

    private func activeDisplayBounds() -> CGRect {
        var displayCount: UInt32 = 0
        guard CGGetActiveDisplayList(0, nil, &displayCount) == .success,
              displayCount > 0
        else { return .null }

        var displays = [CGDirectDisplayID](repeating: 0, count: Int(displayCount))
        guard CGGetActiveDisplayList(displayCount, &displays, &displayCount) == .success else {
            return .null
        }

        return displays.prefix(Int(displayCount)).reduce(CGRect.null) { partial, display in
            let bounds = CGDisplayBounds(display)
            return partial.isNull ? bounds : partial.union(bounds)
        }
    }

    private func cgMouseButton(for button: ControllerPointerButton) -> CGMouseButton {
        switch button {
        case .left: return .left
        case .right: return .right
        case .middle: return .center
        }
    }

    private func mouseEventType(for button: ControllerPointerButton, pressed: Bool) -> CGEventType? {
        switch (button, pressed) {
        case (.left, true): return .leftMouseDown
        case (.left, false): return .leftMouseUp
        case (.right, true): return .rightMouseDown
        case (.right, false): return .rightMouseUp
        case (.middle, true): return .otherMouseDown
        case (.middle, false): return .otherMouseUp
        }
    }

    private static func wheelDelta(from value: Double) -> Int32 {
        guard value.isFinite else { return 0 }
        let clamped = min(max(value, Double(Int32.min)), Double(Int32.max))
        if abs(clamped) < 1 {
            return abs(clamped) >= 0.05 ? (clamped > 0 ? 1 : -1) : 0
        }
        return Int32(clamped.rounded())
    }
}
