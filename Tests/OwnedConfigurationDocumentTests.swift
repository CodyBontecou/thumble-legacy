import Foundation
import XCTest

final class OwnedConfigurationDocumentTests: XCTestCase {
    private var root: URL { URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent() }
    private func fixture(_ name: String) throws -> Data {
        try Data(contentsOf: root.appendingPathComponent("Host/fixtures/owned-configuration/v1/\(name).json"))
    }
    func testFrozenNativeEditProjectionsAndMergedDestination() throws {
        let directory = root.appendingPathComponent("Host/fixtures/owned-projection-merge/v1")
        let output = root.appendingPathComponent("build/OwnedProjectionMerge")
        try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
        for (input, name) in [("input", "before"), ("destination", "after")] {
            let source = try Data(contentsOf: root.appendingPathComponent("Host/fixtures/owned-configuration-edits/v1/\(input).json"))
            let projection = try typedProjection(source)
            try projection.write(to: output.appendingPathComponent("\(name).json"), options: .atomic)
            let frozen = directory.appendingPathComponent("\(name).json")
            guard FileManager.default.fileExists(atPath: frozen.path) else {
                XCTFail("Freeze native projection from build/OwnedProjectionMerge/\(name).json")
                continue
            }
            XCTAssertEqual(try JSONDecoder().decode(ThumbleBridgeJSONValue.self, from: projection),
                           try JSONDecoder().decode(ThumbleBridgeJSONValue.self, from: Data(contentsOf: frozen)))
        }
        let merged = directory.appendingPathComponent("destination.json")
        guard FileManager.default.fileExists(atPath: merged.path) else {
            return XCTFail("Freeze the Rust merged destination before accepting the fixture")
        }
        let original = try Data(contentsOf: merged)
        let actual = try ThumbleOwnedConfigurationDocument(validating: original)
        let expected = try ThumbleOwnedConfigurationDocument(validating: Data(contentsOf:
            root.appendingPathComponent("Host/fixtures/owned-configuration-edits/v1/destination.json")))
        XCTAssertEqual(actual.originalData, original)
        XCTAssertEqual(actual.projectedProfiles, expected.projectedProfiles)
        XCTAssertEqual(actual.activeProfileID, expected.activeProfileID)
        XCTAssertEqual(actual.defaultProfileID, expected.defaultProfileID)
        XCTAssertTrue(String(decoding: original, as: UTF8.self).contains("18446744073709551617"))
        XCTAssertTrue(String(decoding: original, as: UTF8.self).contains("0.12345678901234567890123456789"))
    }
    private func typedProjection(_ source: Data) throws -> Data {
        let document = try ThumbleOwnedConfigurationDocument(validating: source)
        let original = try XCTUnwrap(JSONSerialization.jsonObject(with: source) as? [String: Any])
        let known = Set(["schema", "version", "exportedAt", "activeProfileID", "defaultProfileID",
                         "keyBindings", "outputBindings", "profileKeyBindings", "profileOutputBindings"])
        var result = original.filter { known.contains($0.key) }
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
        result["profiles"] = try document.projectedProfiles.map { profile in
            var projected = try XCTUnwrap(JSONSerialization.jsonObject(with: encoder.encode(profile)) as? [String: Any])
            // The temporary native view supplies landscape for the legacy model.
            // It is not a v4 export or a retained v5 orientation field.
            for key in ["skinReference", "skinBaselineCustomization", "landscapeSkinBaselineCustomization",
                        "portraitSkinBaselineCustomization", "landscapeCustomization", "portraitCustomization", "orientationPreference"] {
                projected.removeValue(forKey: key)
            }
            projected["profileFormatVersion"] = 5
            return projected
        }
        return try JSONSerialization.data(withJSONObject: result, options: [.sortedKeys, .withoutEscapingSlashes])
    }
    func testSharedVersionFiveContractAndRejectedMutations() throws {
        let original = try JSONSerialization.jsonObject(with: fixture("document"))
        let matrix = try XCTUnwrap(JSONSerialization.jsonObject(with: fixture("cases")) as? [String: Any])
        for row in try XCTUnwrap(matrix["cases"] as? [[String: Any]]) {
            let value = mutate(original, path: try XCTUnwrap(row["path"] as? [Any]), row: row)
            let bytes = try JSONSerialization.data(withJSONObject: value)
            let accepted = (try? ThumbleOwnedConfigurationDocument(validating: bytes)) != nil
            XCTAssertEqual(accepted, row["allowed"] as? Bool, row["name"] as? String ?? "")
        }
    }
    private func mutate(_ value: Any, path: [Any], row: [String: Any]) -> Any {
        guard let first = path.first else { return value }
        if let index = first as? Int {
            var array = value as! [Any]
            array[index] = mutate(array[index], path: Array(path.dropFirst()), row: row)
            return array
        }
        let key = first as! String
        var object = value as! [String: Any]
        if path.count == 1 {
            if row["action"] as? String == "remove" { object.removeValue(forKey: key) }
            else { object[key] = row["value"] }
        } else { object[key] = mutate(object[key]!, path: Array(path.dropFirst()), row: row) }
        return object
    }
    func testExactSourceAndAllStatesSurviveNativeProjectionWithoutPackageLookup() throws {
        let original = try fixture("document")
        let document = try ThumbleOwnedConfigurationDocument(validating: original)
        XCTAssertEqual(document.originalData, original)
        let profile = try XCTUnwrap(document.projectedProfiles.first)
        XCTAssertEqual(document.projectedProfiles.count, 2)
        XCTAssertEqual(document.activeProfileID, profile.id)
        XCTAssertEqual(document.defaultProfileID, document.projectedProfiles.last?.id)
        XCTAssertNil(profile.skinReference)
        XCTAssertNil(profile.portraitCustomization)
        XCTAssertNil(profile.landscapeCustomization)
        for scheme in ThumbleSkinColorScheme.allCases {
            for action in ThumbleActionSet.allCases {
                let layout = profile.resolvedCustomization(for: .landscape, colorScheme: scheme, skinPackage: nil, actionSet: action)
                let element = try XCTUnwrap(layout.elements.first)
                XCTAssertEqual(layout.elements.map(\.id),profile.customization.elements.map(\.id))
                XCTAssertEqual(layout.elements.map(\.output),profile.customization.elements.map(\.output))
                XCTAssertEqual(layout.resolvedPresentation(for: layout.buttonCustomization(for: element.inputID), scheme: scheme == .dark ? .dark : .light).icon,
                    .text("\(scheme.rawValue)/\(action.rawValue)"))
                XCTAssertEqual(layout.assetLibrary.assets.first?.data, Data("fixture-image".utf8))
            }
        }
        var copy = document.projectedProfiles
        copy[0].name = "Changed projection"
        copy[0].appearanceBank = nil
        XCTAssertNotEqual(copy[0],document.projectedProfiles[0])
        XCTAssertEqual(document.originalData,original)
        XCTAssertEqual(ProfileFormatPolicy.supportedFormats,[4])
        XCTAssertThrowsError(try ThumbleKeypadConfigurationExport.validateImportBoundary(original))
    }
    func testRustWrittenDestinationRetainsEditablePaintGeometryAndHaptics() throws {
        let data = try Data(contentsOf: root.appendingPathComponent("Host/fixtures/owned-configuration-edits/v1/destination.json"))
        let document = try ThumbleOwnedConfigurationDocument(validating: data)
        let profile = try XCTUnwrap(document.projectedProfiles.first)
        XCTAssertEqual(profile.name, "Edited keypad")
        for scheme in ThumbleSkinColorScheme.allCases {
            for action in ThumbleActionSet.allCases {
                let layout = profile.resolvedCustomization(for: .landscape, colorScheme: scheme, skinPackage: nil, actionSet: action)
                let element = try XCTUnwrap(layout.elements.first)
                let style = layout.buttonCustomization(for: element.inputID)
                XCTAssertEqual(style.centerX, 0.78)
                XCTAssertEqual(layout.resolvedPresentation(for: style, scheme: scheme == .dark ? .dark : .light).icon, .text("Mine"))
                let feedback = try XCTUnwrap(style.hapticFeedback)
                XCTAssertEqual(feedback.pattern, .double)
                XCTAssertEqual(feedback.intensity, 0.25)
                XCTAssertEqual(feedback.sharpness, 0.75)
                XCTAssertEqual(feedback.duration, 0.12)
                XCTAssertEqual(layout.elements.map(\.output), profile.customization.elements.map(\.output))
                XCTAssertEqual(layout.assetLibrary.assets.first?.data, Data("fixture-image".utf8))
            }
        }
        XCTAssertEqual(document.originalData, data)
        let text = try XCTUnwrap(String(data: document.originalData, encoding: .utf8))
        XCTAssertTrue(text.contains("18446744073709551617"))
        XCTAssertTrue(text.contains("0.123456789012345678901234567890"))
    }

    func testDuplicateKeysInvalidUTF8AndExactDocumentLimit() throws {
        let original = try fixture("document")
        let text = try XCTUnwrap(String(data: original, encoding: .utf8))
        let duplicate = text.replacingOccurrences(of: "\"version\": 5", with: "\"version\": 5, \"version\": 4")
        XCTAssertThrowsError(try ThumbleOwnedConfigurationDocument(validating: Data(duplicate.utf8)))
        XCTAssertThrowsError(try ThumbleOwnedConfigurationDocument(validating: Data([255])))
        var maximum = original
        maximum.append(Data(repeating: 32, count: ThumbleOwnedConfigurationDocument.maximumBytes-original.count))
        XCTAssertEqual(try ThumbleOwnedConfigurationDocument(validating: maximum).originalData, maximum)
        maximum.append(32)
        XCTAssertThrowsError(try ThumbleOwnedConfigurationDocument(validating: maximum))
    }
}
