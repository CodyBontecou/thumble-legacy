import Foundation

/// Canonical v2 input report shared by both receiver implementations. Semantic
/// coordinates grow downwards; the Xbox HID encoder converts Y to upwards.
public struct VirtualGamepadReportState: Equatable, Sendable {
    public var buttons: Set<VirtualGamepadButton> = []
    public var leftStickX: Double = 0
    public var leftStickY: Double = 0
    public var rightStickX: Double = 0
    public var rightStickY: Double = 0
    public var leftTrigger: Double = 0
    public var rightTrigger: Double = 0

    public init() {}

    // A digital trigger always actuates its axis. Releasing it restores the
    // independently held analog value instead of releasing another owner.
    public var effectiveLeftTrigger: Double { buttons.contains(.leftTriggerButton) ? 1 : leftTrigger }
    public var effectiveRightTrigger: Double { buttons.contains(.rightTriggerButton) ? 1 : rightTrigger }

    public var reportBytes: [UInt8] {
        var mask: UInt16 = 0
        for button in buttons {
            if let bit = Self.bitIndex(button) { mask |= UInt16(1) << bit }
        }
        // Opposing directions cancel independently on each D-pad axis.
        let opposites: [(VirtualGamepadButton, VirtualGamepadButton, Int, Int)] = [(.dpadUp, .dpadDown, 0, 1), (.dpadLeft, .dpadRight, 2, 3)]
        for (first, second, firstBit, secondBit) in opposites {
            if buttons.contains(first) && buttons.contains(second) { mask &= ~((1 << firstBit) | (1 << secondBit)) }
        }
        var bytes = [UInt8](repeating: 0, count: VirtualGamepadReport.byteCount)
        bytes[1] = UInt8(VirtualGamepadReport.byteCount)
        bytes[2] = UInt8(truncatingIfNeeded: mask); bytes[3] = UInt8(truncatingIfNeeded: mask >> 8)
        bytes[4] = Self.triggerAxis(effectiveLeftTrigger); bytes[5] = Self.triggerAxis(effectiveRightTrigger)
        // Phone/API coordinates grow downwards; Xbox HID Y grows upwards.
        for (offset, value) in [(6, leftStickX), (8, -leftStickY), (10, rightStickX), (12, -rightStickY)] {
            let axis = UInt16(bitPattern: Self.signedAxis(value))
            bytes[offset] = UInt8(truncatingIfNeeded: axis); bytes[offset + 1] = UInt8(truncatingIfNeeded: axis >> 8)
        }
        return bytes
    }

    private static func bitIndex(_ button: VirtualGamepadButton) -> UInt16? {
        switch button {
        case .dpadUp: 0
        case .dpadDown: 1
        case .dpadLeft: 2
        case .dpadRight: 3
        case .start: 4
        case .select: 5
        case .leftStickPress: 6
        case .rightStickPress: 7
        case .leftShoulder: 8
        case .rightShoulder: 9
        case .home: 10
        case .south: 12
        case .east: 13
        case .west: 14
        case .north: 15
        case .leftTriggerButton, .rightTriggerButton: nil
        }
    }

    private static func signedAxis(_ value: Double) -> Int16 {
        let finite = value.isFinite ? min(1, max(-1, value)) : 0
        return Int16((finite * (finite < 0 ? 32768 : 32767)).rounded())
    }

    private static func triggerAxis(_ value: Double) -> UInt8 {
        let finite = value.isFinite ? value : 0
        return UInt8((min(1, max(0, finite)) * 255).rounded())
    }
}

public enum VirtualGamepadReport {
    // Xbox 360 compatibility identity/USB layout. Product/manufacturer remain
    // Thumble. Identity alone enumerates but does not make generic reports work.
    public static let reportID: UInt8 = 0
    public static let byteCount = 20
    public static let vendorID = 0x045E
    public static let productID = 0x028E

    /// Apple Xbox userspace-driver descriptor, pinned from steam-controller-bridge.
    /// Copyright (c) 2026 Lynxware; MIT, see ThirdParty/SteamControllerBridge-LICENSE.txt.
    public static let descriptor: [UInt8] = [
        0x05, 0x01, 0x09, 0x05, 0xA1, 0x01, 0x05, 0x01, 0x09, 0x3A, 0xA1, 0x02, 0x75, 0x08, 0x95, 0x02,
        0x05, 0x01, 0x09, 0x3F, 0x09, 0x3B, 0x81, 0x01, 0x75, 0x01, 0x15, 0x00, 0x25, 0x01, 0x35, 0x00,
        0x45, 0x01, 0x95, 0x04, 0x05, 0x09, 0x19, 0x0C, 0x29, 0x0F, 0x81, 0x02, 0x75, 0x01, 0x15, 0x00,
        0x25, 0x01, 0x35, 0x00, 0x45, 0x01, 0x95, 0x04, 0x05, 0x09, 0x09, 0x09, 0x09, 0x0A, 0x09, 0x07,
        0x09, 0x08, 0x81, 0x02, 0x75, 0x01, 0x15, 0x00, 0x25, 0x01, 0x35, 0x00, 0x45, 0x01, 0x95, 0x03,
        0x05, 0x09, 0x09, 0x05, 0x09, 0x06, 0x09, 0x0B, 0x81, 0x02, 0x75, 0x01, 0x95, 0x01, 0x81, 0x01,
        0x75, 0x01, 0x15, 0x00, 0x25, 0x01, 0x35, 0x00, 0x45, 0x01, 0x95, 0x04, 0x05, 0x09, 0x19, 0x01,
        0x29, 0x04, 0x81, 0x02, 0x75, 0x08, 0x15, 0x00, 0x26, 0xFF, 0x00, 0x35, 0x00, 0x46, 0xFF, 0x00,
        0x95, 0x02, 0x05, 0x01, 0x09, 0x32, 0x09, 0x35, 0x81, 0x02, 0x75, 0x10, 0x16, 0x00, 0x80, 0x26,
        0xFF, 0x7F, 0x36, 0x00, 0x80, 0x46, 0xFF, 0x7F, 0x05, 0x01, 0x09, 0x01, 0xA1, 0x00, 0x95, 0x02,
        0x05, 0x01, 0x09, 0x30, 0x09, 0x31, 0x81, 0x02, 0xC0, 0x05, 0x01, 0x09, 0x01, 0xA1, 0x00, 0x95,
        0x02, 0x05, 0x01, 0x09, 0x33, 0x09, 0x34, 0x81, 0x02, 0xC0, 0x75, 0x08, 0x95, 0x06, 0x81, 0x01,
        0x06, 0x00, 0xFF, 0x15, 0x00, 0x26, 0xFF, 0x00, 0x85, 0x31, 0x09, 0x01, 0x75, 0x08, 0x95, 0x4D,
        0x91, 0x02, 0xC0, 0xC0,
    ]
}
