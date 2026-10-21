import Foundation
import XCTest

final class VirtualGamepadReportTests: XCTestCase {
    func testNeutralReportHasNoPressedDirectionOrTrigger() {
        XCTAssertEqual(VirtualGamepadReportState().reportBytes, [0, 20] + Array(repeating: 0, count: 18))
    }

    func testCanonicalCrossLanguageFixtures() throws {
        struct Vector: Decodable {
            let name: String
            let hex: String
            let buttons: [VirtualGamepadButton]?
            let leftStickX: Double?, leftStickY: Double?, rightStickX: Double?, rightStickY: Double?
            let leftTrigger: Double?, rightTrigger: Double?
        }
        struct Fixtures: Decodable {
            let reportID: UInt8, reportLength: Int, descriptorHex: String
            let vectors: [Vector]
        }
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
        let data = try Data(contentsOf: root.appendingPathComponent("Host/fixtures/gamepad/v2.json"))
        let fixtures = try JSONDecoder().decode(Fixtures.self, from: data)
        XCTAssertEqual(VirtualGamepadReport.reportID, fixtures.reportID)
        XCTAssertEqual(VirtualGamepadReport.byteCount, fixtures.reportLength)
        XCTAssertEqual(VirtualGamepadReport.descriptor, try Self.hexBytes(fixtures.descriptorHex))
        for vector in fixtures.vectors {
            var state = VirtualGamepadReportState()
            state.buttons = Set(vector.buttons ?? [])
            state.leftStickX = vector.leftStickX ?? 0; state.leftStickY = vector.leftStickY ?? 0
            state.rightStickX = vector.rightStickX ?? 0; state.rightStickY = vector.rightStickY ?? 0
            state.leftTrigger = vector.leftTrigger ?? 0; state.rightTrigger = vector.rightTrigger ?? 0
            XCTAssertEqual(state.reportBytes, try Self.hexBytes(vector.hex), vector.name)
        }
    }

    func testDigitalTriggerReleaseRestoresIndependentAnalogHold() {
        var state = VirtualGamepadReportState()
        state.leftTrigger = 0.25
        state.buttons.insert(.leftTriggerButton)
        XCTAssertEqual(state.reportBytes[4], 255)
        state.buttons.remove(.leftTriggerButton)
        XCTAssertEqual(state.reportBytes[4], 64)
    }

    func testNonfiniteAxesFailNeutralRatherThanThrowingOrMovingFullScale() {
        var state = VirtualGamepadReportState()
        state.leftStickX = .nan; state.leftStickY = .infinity
        state.rightStickX = -.infinity; state.rightStickY = .nan
        state.leftTrigger = .nan; state.rightTrigger = .infinity
        XCTAssertEqual(state.reportBytes, [0, 20] + Array(repeating: 0, count: 18))
    }

    func testDescriptorMatchesTheRecognizedXboxInputLayout() {
        var globals: [Int: Int] = [:]
        var usagePage = 0
        var usages: [Int] = []
        var cursor = 0
        var inputBits = 0
        var axes: [Int] = []
        var outputReportID: Int?
        let bytes = VirtualGamepadReport.descriptor
        while cursor < bytes.count {
            let prefix = Int(bytes[cursor]); cursor += 1
            let size = (prefix & 3) == 3 ? 4 : prefix & 3
            XCTAssertLessThanOrEqual(cursor + size, bytes.count)
            let value = bytes[cursor..<cursor + size].enumerated().reduce(0) { $0 | (Int($1.element) << ($1.offset * 8)) }
            cursor += size
            let type = (prefix >> 2) & 3, tag = prefix >> 4
            if type == 1 {
                globals[tag] = value
                if tag == 0 { usagePage = value }
            } else if type == 2 && tag == 0 { usages.append(value) }
            else if type == 0 {
                if tag == 8 {
                    // No input Report ID: the first two bytes are fixed Xbox
                    // type/length, followed by buttons, triggers and 16-bit sticks.
                    XCTAssertNil(globals[8])
                    inputBits += (globals[7] ?? 0) * (globals[9] ?? 0)
                    if usagePage == 1 && value & 1 == 0 {
                        axes.append(contentsOf: usages)
                        if usages.contains(0x30) || usages.contains(0x33) {
                            XCTAssertEqual(globals[7], 16)
                            XCTAssertEqual(globals[1], 0x8000)
                            XCTAssertEqual(globals[2], 0x7FFF)
                        }
                    }
                } else if tag == 9 { outputReportID = globals[8] }
                usages.removeAll()
            }
        }
        XCTAssertEqual(inputBits, 160)
        XCTAssertEqual(axes, [0x32, 0x35, 0x30, 0x31, 0x33, 0x34])
        XCTAssertEqual(outputReportID, 0x31) // Enumerated, safely rejected by the input-only backend.
        XCTAssertEqual(VirtualGamepadReport.vendorID, 0x045E)
        XCTAssertEqual(VirtualGamepadReport.productID, 0x028E)
    }

    private static func hexBytes(_ text: String) throws -> [UInt8] {
        struct InvalidHex: Error {}
        guard text.count.isMultiple(of: 2) else { throw InvalidHex() }
        var bytes: [UInt8] = []
        var index = text.startIndex
        while index < text.endIndex {
            let next = text.index(index, offsetBy: 2)
            guard let byte = UInt8(text[index..<next], radix: 16) else { throw InvalidHex() }
            bytes.append(byte); index = next
        }
        return bytes
    }
}
