import Foundation
import XCTest

final class ThumbleLinkTests: XCTestCase {
    private func fixture() throws -> String {
        try String(contentsOf: URL(fileURLWithPath: #filePath).deletingLastPathComponent()
            .appendingPathComponent("Fixtures/ThumbleLink/warrior.txt"), encoding: .utf8)
    }

    private func emptyConfiguration() throws -> Data {
        let profile = GamepadConfigurationProfile(name: "Existing Setup", customization: GamepadCustomization())
        return try JSONEncoder().encode(ThumbleKeypadConfigurationExport(profiles: [profile], activeProfileID: profile.id, defaultProfileID: profile.id))
    }

    private func profile(_ plan: ThumbleLinkImport.Plan) throws -> GamepadConfigurationProfile {
        try XCTUnwrap(MacConfigurationBindings.decodeKeypadImport(data: plan.configurationData, sourceName: "Test").profiles.first)
    }

    private func editedSnapshot(_ change: (inout [String: Any]) -> Void) throws -> String {
        let json = try fixture().replacingOccurrences(of: ThumbleLinkSnapshot.prefix, with: "")
        var value = try XCTUnwrap(JSONSerialization.jsonObject(with: Data(json.utf8)) as? [String: Any])
        change(&value)
        return String(decoding: try JSONSerialization.data(withJSONObject: value, options: .sortedKeys), as: UTF8.self)
    }

    func testSnapshotBoundsVersionsAndDuplicateKeys() throws {
        let snapshot = try ThumbleLinkSnapshot.parse(fixture())
        XCTAssertEqual(snapshot.character.name, "Test Warrior")
        XCTAssertEqual(snapshot.actions.count, 5)
        XCTAssertEqual(snapshot.profileID, try ThumbleLinkSnapshot.parse(fixture()).profileID)
        XCTAssertThrowsError(try ThumbleLinkSnapshot.parse(String(repeating: "x", count: ThumbleLinkSnapshot.maximumBytes + 1)))
        XCTAssertThrowsError(try ThumbleLinkSnapshot.parse("{\"schema\":\"a\",\"schema\":\"b\"}"))
        for invalid in [try editedSnapshot { $0["version"] = 2 },
                        try editedSnapshot { $0["actions"] = [] },
                        try editedSnapshot { $0["client"] = ["flavor": "retail", "version": "12", "build": "1", "interface": 120000] },
                        try editedSnapshot { value in
                            let rows = value["actions"] as! [[String: Any]]
                            value["actions"] = [rows[0], rows[0]]
                        }] {
            XCTAssertThrowsError(try ThumbleLinkSnapshot.parse(invalid))
        }
    }

    func testClassicEraImportRefreshAndClientIsolation() throws {
        let eraText = try String(contentsOf: URL(fileURLWithPath: #filePath).deletingLastPathComponent()
            .appendingPathComponent("Fixtures/ThumbleLink/warrior-classic-era.txt"), encoding: .utf8)
        let forever = try ThumbleLinkSnapshot.parse(fixture())
        let era = try ThumbleLinkSnapshot.parse(eraText)
        XCTAssertEqual(era.client.displayName, "Classic Era")
        XCTAssertNotEqual(era.profileID, forever.profileID)
        XCTAssertNotEqual(era.elementID(for: "ACTIONBUTTON1"), forever.elementID(for: "ACTIONBUTTON1"))
        let first = try ThumbleLinkImport.prepare(text: eraText, currentConfiguration: emptyConfiguration())
        XCTAssertEqual(try profile(first).customization.elements.count, 3)
        let refreshed = try ThumbleLinkImport.prepare(text: eraText, currentConfiguration: first.configurationData)
        XCTAssertTrue(refreshed.isRefresh)
        let otherClient = try ThumbleLinkImport.prepare(text: fixture(), currentConfiguration: first.configurationData)
        XCTAssertFalse(otherClient.isRefresh)
        let both = try MacConfigurationBindings.decodeKeypadImport(data: otherClient.configurationData, sourceName: "Test")
        // Plans contain only the profile to merge; the authority preserves other setups.
        XCTAssertEqual(both.profiles.map(\.id), [forever.profileID])
        XCTAssertEqual(try profile(first).id, era.profileID)
        for (flavor, interface) in [("classic-era", 16001), ("forever", 11509), ("classic-era", 11508)] {
            XCTAssertThrowsError(try ThumbleLinkSnapshot.parse(editedSnapshot {
                $0["client"] = ["flavor": flavor, "interface": interface, "version": "test", "build": "1"]
            }))
        }
    }

    private func hunterFixture() throws -> String {
        try String(contentsOf: URL(fileURLWithPath: #filePath).deletingLastPathComponent()
            .appendingPathComponent("Fixtures/ThumbleLink/hunter-classic-era.txt"), encoding: .utf8)
    }

    func testControllerImportGroupsActualBindingsAndKeepsFullLegends() throws {
        let plan = try ThumbleLinkImport.prepare(text: hunterFixture(), currentConfiguration: emptyConfiguration())
        XCTAssertEqual(plan.layout, .controller)
        let controller = try profile(plan)
        XCTAssertEqual(controller.outputMode, .keyboard)
        XCTAssertEqual(controller.customization.elements.count, 16)
        let actions = Dictionary(uniqueKeysWithValues: plan.snapshot.actions.map { ($0.command, $0) })
        for orientation in [GamepadEditorDeviceOrientation.portrait, .landscape] {
            let customization = controller.customization(for: orientation)
            let controls = customization.resolvedControls(in: customization.deviceCanvas.editorDeviceFrame.screenRect.size)
                .filter { $0.elementID != nil }
            XCTAssertEqual(controls.count, 16)
            for row in plan.rows where row.key != nil {
                let id = plan.snapshot.elementID(for: row.command)
                let element = try XCTUnwrap(customization.elements.first { $0.id == id })
                XCTAssertEqual(element.output?.keyboard, row.key.flatMap(ThumbleLinkKeyBinding.resolve))
                XCTAssertTrue(element.output?.gamepadButtons.isEmpty ?? true)
                XCTAssertEqual(element.presentation?.accessibilityName, actions[row.command]?.label)
            }
            let forward = try XCTUnwrap(customization.elements.first { $0.id == plan.snapshot.elementID(for: "MOVEFORWARD") })
            let attack = try XCTUnwrap(customization.elements.first { $0.id == plan.snapshot.elementID(for: "ACTIONBUTTON1") })
            XCTAssertEqual(forward.visualRole, .movement)
            XCTAssertEqual(forward.presentation?.legend, "Up")
            let target = try XCTUnwrap(customization.elements.first { $0.id == plan.snapshot.elementID(for: "TARGETNEARESTENEMY") })
            let previous = try XCTUnwrap(customization.elements.first { $0.id == plan.snapshot.elementID(for: "TARGETPREVIOUSENEMY") })
            XCTAssertEqual(target.presentation?.legend, "Target")
            XCTAssertEqual(previous.presentation?.legend, "Previous")
            XCTAssertLessThan(try XCTUnwrap(forward.layout.centerX), try XCTUnwrap(attack.layout.centerX))
            for control in controls {
                XCTAssertGreaterThanOrEqual(control.size.width, 44)
                XCTAssertGreaterThanOrEqual(control.size.height, 44)
                for other in controls where other.id != control.id {
                    XCTAssertFalse(control.hitFrame.insetBy(dx: 0.01, dy: 0.01).intersects(other.hitFrame),
                        "\(orientation.rawValue): \(control.visualLegend) overlaps \(other.visualLegend)")
                }
            }
        }
        let raptor = try XCTUnwrap(controller.customization.elements.first { $0.id == plan.snapshot.elementID(for: "ACTIONBUTTON2") })
        XCTAssertEqual(raptor.presentation?.legend, "Raptor Strike")
    }

    func testControllerOverflowKeepsEveryBindingAndRemainsNonOverlapping() throws {
        let original = try hunterFixture().replacingOccurrences(of: ThumbleLinkSnapshot.prefix, with: "")
        var snapshot = try XCTUnwrap(JSONSerialization.jsonObject(with: Data(original.utf8)) as? [String: Any])
        var actions = try XCTUnwrap(snapshot["actions"] as? [[String: Any]])
        for index in 1...8 {
            actions.append(["command": "EXTRABUTTON\(index)", "label": "Extra \(index)", "keys": ["F\(index + 2)"], "kind": "spell"])
        }
        snapshot["actions"] = actions
        let text = String(decoding: try JSONSerialization.data(withJSONObject: snapshot), as: UTF8.self)
        let plan = try ThumbleLinkImport.prepare(text: text, currentConfiguration: emptyConfiguration())
        XCTAssertEqual(plan.layout, .controller)
        let controller = try profile(plan)
        for orientation in [GamepadEditorDeviceOrientation.portrait, .landscape] {
            let layout = controller.customization(for: orientation)
            let controls = layout.resolvedControls(in: layout.deviceCanvas.editorDeviceFrame.screenRect.size)
                .filter { $0.elementID != nil }
            XCTAssertEqual(controls.count, 24)
            for first in controls {
                XCTAssertGreaterThanOrEqual(first.size.width, 44)
                XCTAssertGreaterThanOrEqual(first.size.height, 44)
                for second in controls where second.id != first.id {
                    XCTAssertFalse(first.hitFrame.insetBy(dx: 0.01, dy: 0.01).intersects(second.hitFrame))
                }
            }
        }
        actions.append(["command": "EXTRABUTTON9", "label": "Extra 9", "keys": ["F11"], "kind": "spell"])
        snapshot["actions"] = actions
        let tooMany = String(decoding: try JSONSerialization.data(withJSONObject: snapshot), as: UTF8.self)
        XCTAssertThrowsError(try ThumbleLinkImport.prepare(text: tooMany, currentConfiguration: emptyConfiguration(), layout: .controller))
        let grid = try ThumbleLinkImport.prepare(text: tooMany, currentConfiguration: emptyConfiguration())
        XCTAssertEqual(grid.layout, .grid)
        XCTAssertEqual(try profile(grid).customization.elements.count, 25)
    }

    func testExplicitControllerConversionPreservesBindingsAndOrdinaryRefreshPreservesLayout() throws {
        let initial = try ThumbleLinkImport.prepare(text: hunterFixture(), currentConfiguration: emptyConfiguration(), layout: .grid)
        let grid = try profile(initial)
        let plainRefresh = try ThumbleLinkImport.prepare(text: hunterFixture(), currentConfiguration: initial.configurationData)
        XCTAssertEqual(plainRefresh.layout, .grid)
        XCTAssertEqual(try profile(plainRefresh).customization.elements.map(\.layout), grid.customization.elements.map(\.layout))
        let converted = try ThumbleLinkImport.prepare(text: hunterFixture(), currentConfiguration: initial.configurationData, layout: .controller)
        let controller = try profile(converted)
        XCTAssertEqual(controller.customization.elements.map(\.id), grid.customization.elements.map(\.id))
        XCTAssertEqual(controller.customization.elements.map(\.output), grid.customization.elements.map(\.output))
        XCTAssertNotEqual(controller.customization.elements.map(\.layout), grid.customization.elements.map(\.layout))
        let refreshed = try ThumbleLinkImport.prepare(text: hunterFixture(), currentConfiguration: converted.configurationData)
        XCTAssertEqual(refreshed.layout, .controller)
        XCTAssertEqual(try profile(refreshed).customization.elements.map(\.layout), controller.customization.elements.map(\.layout))
    }

    func testWoWKeyVocabularyUsesPhysicalKeysAndModifierChords() throws {
        XCTAssertEqual(ThumbleLinkKeyBinding.resolve("CTRL-SHIFT-Q"), KeypadKeyboardBinding(keyCode: 12, modifiersRawValue: 10))
        XCTAssertEqual(ThumbleLinkKeyBinding.resolve("SHIFT--"), KeypadKeyboardBinding(keyCode: 27, modifiersRawValue: 2))
        XCTAssertEqual(ThumbleLinkKeyBinding.resolve("DELETE")?.keyCode, 117)
        XCTAssertEqual(ThumbleLinkKeyBinding.resolve("BACKSPACE")?.keyCode, 51)
        XCTAssertEqual(ThumbleLinkKeyBinding.resolve("NUMPAD1")?.keyCode, 83)
        XCTAssertEqual(ThumbleLinkKeyBinding.resolve("ALT-META-F12")?.modifiersRawValue, 5)
        for unsupported in ["BUTTON1", "MOUSEWHEELUP", "PAD1", "SHIFT-BUTTON4", "18", "123", "CTRL-CTRL-Q", "F01", "F21", "", "CMD-Q"] {
            XCTAssertNil(ThumbleLinkKeyBinding.resolve(unsupported), unsupported)
        }
    }

    func testCreateProfileSelectsSupportedAlternateAndSkipsUnboundOrOverridden() throws {
        let plan = try ThumbleLinkImport.prepare(text: fixture(), currentConfiguration: emptyConfiguration())
        let profile = try profile(plan)
        XCTAssertFalse(plan.isRefresh)
        XCTAssertEqual(profile.outputMode, .keyboard)
        XCTAssertEqual(profile.customization.elements.count, 3)
        XCTAssertEqual(profile.portraitCustomization?.elements.map(\.id), profile.customization.elements.map(\.id))
        let q = try XCTUnwrap(profile.customization.elements.first { $0.label == "Pummel" })
        XCTAssertEqual(q.output?.keyboard?.keyCode, 12)
        XCTAssertEqual(plan.rows.first { $0.label == "Pummel" }?.key, "Q")
        XCTAssertEqual(plan.rows.filter { $0.key == nil }.count, 2)
        XCTAssertEqual(q.defaultOutput, q.output)
        XCTAssertTrue(try ThumbleKeypadConfigurationExport.validateImportBoundary(plan.configurationData))
    }

    func testRefreshUpdatesBindingsAndNamesPreservingGeometryAndMetadata() throws {
        let first = try ThumbleLinkImport.prepare(text: fixture(), currentConfiguration: emptyConfiguration())
        var envelope = try MacConfigurationBindings.decodeKeypadImport(data: first.configurationData, sourceName: "Test")
        let id = first.snapshot.elementID(for: "ACTIONBUTTON1")
        let index = try XCTUnwrap(envelope.profiles[0].customization.elements.firstIndex { $0.id == id })
        envelope.profiles[0].customization.setPosition(CGPoint(x: 0.24, y: 0.54), for: .custom(id))
        envelope.profiles[0].name = "My Warrior"
        let newText = try editedSnapshot { value in
            var actions = value["actions"] as! [[String: Any]]
            actions[0]["keys"] = ["E"]
            actions[0]["label"] = "New Ability"
            value["actions"] = actions
        }
        let plan = try ThumbleLinkImport.prepare(text: newText, currentConfiguration: JSONEncoder().encode(envelope))
        let refreshed = try profile(plan)
        XCTAssertTrue(plan.isRefresh)
        XCTAssertEqual(plan.profileName, "My Warrior")
        XCTAssertEqual(try XCTUnwrap(refreshed.customization.elements[index].layout.centerX), 0.24, accuracy: 0.001)
        XCTAssertEqual(refreshed.customization.elements[index].label, "New Ability")
        XCTAssertEqual(refreshed.customization.elements[index].output?.keyboard?.keyCode, 14)
        XCTAssertEqual(refreshed.portraitCustomization?.elements.first { $0.id == id }?.output?.keyboard?.keyCode, 14)
        let raw = try MacConfigurationBindings.decodeKeypadImport(data: plan.configurationData, sourceName: "Test")
        guard case .object(let source) = raw.profileSources[refreshed.id] else { return XCTFail("missing raw metadata") }
        XCTAssertNotNil(source["thumbleLink"])
    }

    func testRefreshPreservesUserLabelsOutputsAndDeletedControls() throws {
        let first = try ThumbleLinkImport.prepare(text: fixture(), currentConfiguration: emptyConfiguration())
        var envelope = try MacConfigurationBindings.decodeKeypadImport(data: first.configurationData, sourceName: "Test")
        let id = first.snapshot.elementID(for: "ACTIONBUTTON1")
        let index = try XCTUnwrap(envelope.profiles[0].customization.elements.firstIndex { $0.id == id })
        envelope.profiles[0].customization.elements[index].label = "My button"
        let mirrorIndex = try XCTUnwrap(envelope.profiles[0].customization.customButtons.firstIndex { $0.id == id })
        envelope.profiles[0].customization.customButtons[mirrorIndex].label = "My button"
        let custom = KeypadElementOutputBinding(keyboard: KeypadKeyboardBinding(keyCode: 7))
        envelope.profiles[0].customization.elements[index].output = custom
        envelope.profileOutputBindings[envelope.profiles[0].id.uuidString]?[id.uuidString] = MacControlOutputBinding(shared: custom)
        envelope.profileKeyBindings[envelope.profiles[0].id.uuidString]?[id.uuidString] = MacControlOutputBinding(shared: custom).keyboard
        let deletedID = first.snapshot.elementID(for: "ACTIONBUTTON2")
        envelope.profiles[0].customization.removeCustomButton(id: deletedID)
        envelope.profiles[0].portraitCustomization?.removeCustomButton(id: deletedID)
        envelope.profileOutputBindings[envelope.profiles[0].id.uuidString]?.removeValue(forKey: deletedID.uuidString)
        envelope.profileKeyBindings[envelope.profiles[0].id.uuidString]?.removeValue(forKey: deletedID.uuidString)
        let refreshed = try profile(ThumbleLinkImport.prepare(text: fixture(), currentConfiguration: JSONEncoder().encode(envelope)))
        XCTAssertEqual(refreshed.customization.elements.first { $0.id == id }?.label, "My button")
        XCTAssertEqual(refreshed.customization.elements.first { $0.id == id }?.output, custom)
        XCTAssertNil(refreshed.customization.elements.first { $0.id == deletedID })
    }

    func testMissingBindingClearsPreviouslyImportedOutput() throws {
        let first = try ThumbleLinkImport.prepare(text: fixture(), currentConfiguration: emptyConfiguration())
        let newText = try editedSnapshot { value in
            var actions = value["actions"] as! [[String: Any]]
            actions[0]["keys"] = []
            value["actions"] = actions
        }
        let refreshed = try profile(ThumbleLinkImport.prepare(text: newText, currentConfiguration: first.configurationData))
        let element = try XCTUnwrap(refreshed.customization.elements.first { $0.id == first.snapshot.elementID(for: "ACTIONBUTTON1") })
        XCTAssertTrue(element.output?.isEmpty == true)
        XCTAssertTrue(element.defaultOutput?.isEmpty == true)
    }

    func testPreviouslyUnboundActionIsAddedWhenItGetsAKeyboardBinding() throws {
        let first = try ThumbleLinkImport.prepare(text: fixture(), currentConfiguration: emptyConfiguration())
        let text = try editedSnapshot { value in
            var actions = value["actions"] as! [[String: Any]]
            actions[3]["keys"] = ["R"]
            value["actions"] = actions
        }
        let refreshed = try profile(ThumbleLinkImport.prepare(text: text, currentConfiguration: first.configurationData))
        XCTAssertEqual(refreshed.customization.elements.count, 4)
        XCTAssertEqual(refreshed.customization.elements.first { $0.id == first.snapshot.elementID(for: "ACTIONBUTTON4") }?.output?.keyboard?.keyCode, 15)
    }

    func testReviewDetectsConcurrentEditsAndNameCollisions() throws {
        let base = try emptyConfiguration()
        let plan = try ThumbleLinkImport.prepare(text: fixture(), currentConfiguration: base)
        try plan.requireUnchangedConfiguration(base)
        var envelope = try MacConfigurationBindings.decodeKeypadImport(data: base, sourceName: "Test")
        envelope.profiles[0].name = "Changed"
        XCTAssertThrowsError(try plan.requireUnchangedConfiguration(JSONEncoder().encode(envelope)))
        XCTAssertThrowsError(try ThumbleLinkImport.prepare(text: fixture(), currentConfiguration: base, name: "Existing Setup"))
    }

    func testNoUsableBindingsDoesNotProduceAnImport() throws {
        let text = try editedSnapshot { value in
            var actions = value["actions"] as! [[String: Any]]
            for index in actions.indices { actions[index]["keys"] = ["BUTTON1"] }
            value["actions"] = actions
        }
        XCTAssertThrowsError(try ThumbleLinkImport.prepare(text: text, currentConfiguration: emptyConfiguration()))
    }
}
