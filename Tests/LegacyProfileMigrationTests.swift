import CryptoKit
import SwiftUI
import XCTest

final class LegacyProfileMigrationTests: XCTestCase {
    private var root: URL { URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent() }
    private func migrationFixture() throws -> [String: Any] {
        try XCTUnwrap(JSONSerialization.jsonObject(with: Data(contentsOf: root.appendingPathComponent("Host/fixtures/feature-removal/v1/migration-cases.json"))) as? [String: Any])
    }
    private func package(_ raw: [String: Any]) throws -> ThumbleSkinPackage {
        var manifest = try XCTUnwrap(raw["manifest"] as? [String: Any])
        let payloads = try XCTUnwrap(raw["assetPayloads"] as? [String: String]).mapValues { try XCTUnwrap(Data(base64Encoded: $0)) }
        // Frozen resolver specimens predate archive-integrity requirements.
        // Build a valid in-memory package without altering any frozen source.
        var assets = try XCTUnwrap(manifest["assets"] as? [[String: Any]])
        for index in assets.indices {
            let data = try XCTUnwrap(payloads[XCTUnwrap(assets[index]["id"] as? String)])
            assets[index]["byteCount"] = data.count
            assets[index]["sha256"] = SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
        }
        manifest["assets"] = assets
        return ThumbleSkinPackage(manifest: try JSONDecoder().decode(ThumbleSkinManifest.self, from: JSONSerialization.data(withJSONObject: manifest)),
            skin: try JSONDecoder().decode(ThumbleSkin.self, from: JSONSerialization.data(withJSONObject: XCTUnwrap(raw["skin"]))), assets: payloads)
    }
    func testNativePreparationTransportAndFrozenMigrationInputs() throws {
        let fixture = try migrationFixture()
        let packages = try XCTUnwrap(fixture["packages"] as? [String: [String: Any]])
        var rows = try XCTUnwrap(fixture["cases"] as? [[String: Any]])
        var alternate = try XCTUnwrap(rows.first { $0["name"] as? String == "plain" })
        alternate["name"] = "explicit-landscape"
        var alternateProfile = try XCTUnwrap(alternate["profile"] as? [String: Any])
        var primary = try XCTUnwrap(alternateProfile["customization"] as? [String: Any])
        var landscape = primary
        primary["colorSchemePreference"] = "light"
        landscape["colorSchemePreference"] = "dark"
        landscape["futureLayoutMetadata"] = ["scope": "landscape", "keep": true]
        alternateProfile["customization"] = primary
        alternateProfile["landscapeCustomization"] = landscape
        alternate["profile"] = alternateProfile
        rows.append(alternate)
        for name in ["legacy-canvas-alias", "missing-canvas"] {
            var row = try XCTUnwrap(rows.first { $0["name"] as? String == "plain" })
            row["name"] = name
            var profile = try XCTUnwrap(row["profile"] as? [String: Any])
            var layout = try XCTUnwrap(profile["customization"] as? [String: Any])
            if name == "legacy-canvas-alias" {
                layout["deviceCanvas"] = ["frameID": "landscape", "futureCanvasMetadata": "keep"]
            } else { layout.removeValue(forKey: "deviceCanvas") }
            profile["customization"] = layout
            row["profile"] = profile
            rows.append(row)
        }
        let output = root.appendingPathComponent("build/LegacyConfigurationMigration")
        try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
        for (index, row) in rows.enumerated() {
            let name = try XCTUnwrap(row["name"] as? String)
            var profile = try XCTUnwrap(row["profile"] as? [String: Any])
            var layout = try XCTUnwrap(profile["customization"] as? [String: Any])
            layout["futureLayoutMetadata"] = ["scope": "selected", "keep": true]
            profile["customization"] = layout
            var sourceText = String(decoding: try JSONSerialization.data(withJSONObject: profile, options: [.sortedKeys, .withoutEscapingSlashes]), as: UTF8.self)
            sourceText.removeLast()
            sourceText += ",\"futureNativeMigrationMetadata\":{\"large\":18446744073709551617,\"decimal\":0.12345678901234567890123456789}}"
            let source = Data(sourceText.utf8)
            let invocation = try XCTUnwrap(UUID(uuidString: "ba000000-0000-4000-8000-00000000000\(index)"))
            var packet: [String: Any] = ["schemaVersion": 1, "invocationID": invocation.uuidString,
                "legacyProfileCommand": "prepare", "profileData": source.base64EncodedString()]
            if let key = row["availablePackage"] as? String {
                packet["packageData"] = try ThumbleSkinPackageCodec.encode(package(XCTUnwrap(packages[key]))).base64EncodedString()
            }
            let requestData = try JSONSerialization.data(withJSONObject: packet, options: [.sortedKeys, .withoutEscapingSlashes])
            let request = try JSONDecoder().decodeUnique(ThumbleLegacyProfileBridgeRequest.self, from: requestData)
            var generated = ["\(name)-profile.json": source, "\(name)-request.json": requestData]
            if row["expectedDisposition"] as? String == "recovery-required" {
                XCTAssertThrowsError(try ThumbleLegacyProfileBridge.transform(request)) {
                    XCTAssertEqual($0 as? ThumbleLegacyProfileBridgeError, .missingPackage)
                }
            } else {
                let result = try ThumbleLegacyProfileBridge.transform(request)
                XCTAssertEqual(result.invocationID, invocation)
                XCTAssertEqual(result.sourceSHA256, SHA256.hash(data: source).map { String(format: "%02x", $0) }.joined())
                XCTAssertEqual(result.sourceLayout, name == "explicit-landscape" ? "landscapeCustomization" : "customization")
                XCTAssertEqual(result.sourceOrientation, name == "portrait-only" ? "portrait" : "landscape")
                let encoder = JSONEncoder(); encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
                generated["\(name)-response.json"] = try encoder.encode(result)
                XCTAssertEqual(try JSONDecoder().decode(GamepadConfigurationProfile.self, from: result.afterProfileData).id, result.profileID)
            }
            for (filename, bytes) in generated {
                try bytes.write(to: output.appendingPathComponent(filename), options: .atomic)
                let frozen = root.appendingPathComponent("Host/fixtures/legacy-configuration-migration/v1/\(filename)")
                guard FileManager.default.fileExists(atPath: frozen.path) else {
                    XCTFail("Freeze generated input from build/LegacyConfigurationMigration/\(filename)")
                    continue
                }
                XCTAssertEqual(bytes, try Data(contentsOf: frozen), filename)
            }
        }
    }
    func testPreparationTransportRejectsUnexpectedFieldsAndInvalidBase64() throws {
        var packet: [String: Any] = ["schemaVersion": 1, "invocationID": UUID().uuidString,
            "legacyProfileCommand": "prepare", "profileData": "e30="]
        for key in ["path", "operation", "skinCommand"] {
            var invalid = packet; invalid[key] = "unexpected"
            XCTAssertThrowsError(try JSONDecoder().decodeUnique(ThumbleLegacyProfileBridgeRequest.self,
                from: JSONSerialization.data(withJSONObject: invalid)))
        }
        packet["profileData"] = "e30"
        XCTAssertThrowsError(try JSONDecoder().decodeUnique(ThumbleLegacyProfileBridgeRequest.self,
            from: JSONSerialization.data(withJSONObject: packet)))
    }
    func testUnknownStoredCanvasRequiresRecoveryWithoutChoosingADefault() throws {
        let rows = try XCTUnwrap(try migrationFixture()["cases"] as? [[String: Any]])
        var profile = try XCTUnwrap(rows.first { $0["name"] as? String == "plain" }?["profile"] as? [String: Any])
        var layout = try XCTUnwrap(profile["customization"] as? [String: Any])
        layout["deviceCanvas"] = ["frameID": "unknown-future-device"]
        profile["customization"] = layout
        let data = try JSONSerialization.data(withJSONObject: profile)
        XCTAssertThrowsError(try ThumbleLegacyProfileMigration.prepare(profileData: data, package: nil)) {
            XCTAssertEqual($0 as? ThumbleLegacyProfileMigration.RecoveryRequired, .invalidSource)
        }
    }
    func testActualLegacyConversionDocumentsRetainNativeProfileFeatures() throws {
        let directory = root.appendingPathComponent("Host/fixtures/legacy-configuration-migration/v1")
        for name in ["plain", "portrait-only", "explicit-landscape", "legacy-canvas-alias", "missing-canvas"] {
            let bytes = try Data(contentsOf: directory.appendingPathComponent("\(name)-destination.json"))
            let document = try ThumbleOwnedConfigurationDocument(validating: bytes)
            let actual = try XCTUnwrap(document.projectedProfiles.first)
            let packet = try XCTUnwrap(JSONSerialization.jsonObject(with: Data(contentsOf:
                directory.appendingPathComponent("\(name)-response.json"))) as? [String: Any])
            let encoded = try XCTUnwrap(packet["afterProfileData"] as? String)
            let staged = try JSONDecoder().decode(GamepadConfigurationProfile.self, from: XCTUnwrap(Data(base64Encoded: encoded)))
            XCTAssertEqual(document.originalData, bytes)
            XCTAssertNil(document.defaultProfileID)
            XCTAssertEqual(actual, staged)
            for scheme in ThumbleSkinColorScheme.allCases { for action in ThumbleActionSet.allCases {
                let layout = actual.resolvedCustomization(for: .landscape, colorScheme: scheme, skinPackage: nil, actionSet: action)
                let expected = staged.resolvedCustomization(for: .landscape, colorScheme: scheme, skinPackage: nil, actionSet: action)
                XCTAssertEqual(GamepadAppearanceSnapshot(layout), GamepadAppearanceSnapshot(expected))
                XCTAssertEqual(layout.elements.map(\.output), expected.elements.map(\.output))
            }}
            let text = String(decoding: bytes, as: UTF8.self)
            XCTAssertTrue(text.contains("18446744073709551617"))
            XCTAssertTrue(text.contains("0.12345678901234567890123456789"))
            XCTAssertTrue(text.contains("futureLayoutMetadata"))
            XCTAssertThrowsError(try ThumbleKeypadConfigurationExport.validateImportBoundary(bytes))
        }
    }
    func testFrozenMigrationDispositionsProducePreparationOrPreservedRecovery() throws {
        let fixture = try migrationFixture()
        let packages = try XCTUnwrap(fixture["packages"] as? [String: [String: Any]])
        for row in try XCTUnwrap(fixture["cases"] as? [[String: Any]]) {
            let data = try JSONSerialization.data(withJSONObject: XCTUnwrap(row["profile"]), options: [.sortedKeys])
            let skin = try (row["availablePackage"] as? String).map { try package(XCTUnwrap(packages[$0])) }
            if row["expectedDisposition"] as? String == "recovery-required" {
                XCTAssertThrowsError(try ThumbleLegacyProfileMigration.prepare(profileData: data, package: skin)) {
                    XCTAssertEqual($0 as? ThumbleLegacyAppearanceMaterializer.RecoveryRequired, .missingPackage)
                }
                continue
            }
            let result = try ThumbleLegacyProfileMigration.prepare(profileData: data, package: skin)
            let staging = result.stagingProfile
            XCTAssertEqual(result.originalData, data)
            XCTAssertEqual(staging.id, result.originalProfile.id)
            XCTAssertEqual(staging.updatedAt, result.originalProfile.updatedAt)
            XCTAssertEqual(staging.outputMode, result.originalProfile.outputMode)
            XCTAssertEqual(staging.launchTarget, result.originalProfile.launchTarget)
            XCTAssertNil(staging.skinReference)
            XCTAssertNil(staging.skinBaselineCustomization)
            XCTAssertNil(staging.landscapeSkinBaselineCustomization)
            XCTAssertNil(staging.portraitSkinBaselineCustomization)
            XCTAssertNil(staging.landscapeCustomization)
            XCTAssertNil(staging.portraitCustomization)
            XCTAssertTrue(staging.customization.deviceCanvas.editorDeviceFrame.isLandscape)
            XCTAssertEqual(staging.customization.elements.map(\.id), result.sourceCustomization.elements.map(\.id))
            XCTAssertEqual(staging.customization.elements.map(\.output), result.sourceCustomization.elements.map(\.output))
            if row["expectedDisposition"] as? String == "convert-and-review" {
                XCTAssertTrue(result.reviewReasons.contains(.portraitArranged))
                XCTAssertEqual(result.sourceOrientation, .portrait)
            } else { XCTAssertTrue(result.reviewReasons.isEmpty) }
            let again = try ThumbleLegacyProfileMigration.prepare(profileData: data, package: skin)
            XCTAssertEqual(again.stagingProfile,staging)
            XCTAssertEqual(again.reviewReasons,result.reviewReasons)
        }
    }
    func testPortraitArrangementKeepsGroupsGuidesBindingsAndHaptics() throws {
        var local = GamepadCustomization.defaultValue
        local.deviceCanvas = GamepadDeviceCanvas(frameID: "iphone-17-pro-portrait")
        let first = try XCTUnwrap(local.elements.first).inputID
        var style = local.buttonCustomization(for: first)
        style.centerX = 0.2; style.centerY = 0.3; style.isHidden = true
        style.hapticFeedback = .init(style: .light, pattern: .double, intensity: 0.25, sharpness: 0.75, duration: 0.12)
        local.setButtonCustomization(style,for:first)
        let groupID = UUID(uuidString: "dc000000-0000-0000-0000-000000000001")!
        local.designMetadata = .init(groups: [.init(id: groupID, name: "Retain", children: [.builtin(first)])])
        let profile = GamepadConfigurationProfile(name: "Portrait", primaryCustomization: local, orientationPreference: .portrait, updatedAt: 100)
        let result = try ThumbleLegacyProfileMigration.prepare(profileData: JSONEncoder().encode(profile),package:nil)
        let destination = result.stagingProfile.customization
        XCTAssertEqual(try XCTUnwrap(destination.buttonCustomization(for:first).centerX),0.7,accuracy:0.000001)
        XCTAssertEqual(try XCTUnwrap(destination.buttonCustomization(for:first).centerY),0.2,accuracy:0.000001)
        XCTAssertEqual(destination.buttonCustomization(for:first).hapticFeedback,style.hapticFeedback)
        XCTAssertTrue(destination.buttonCustomization(for:first).isHidden)
        XCTAssertEqual(destination.designMetadata?.groups,profile.customization.designMetadata?.groups)
        XCTAssertEqual(destination.elements.map(\.output),profile.customization.elements.map(\.output))
        XCTAssertEqual(result.stagingProfile.updatedAt,100)
        XCTAssertEqual(try ThumbleLegacyProfileMigration.prepare(profileData: JSONEncoder().encode(profile),package:nil).stagingProfile,result.stagingProfile)
    }
    func testVariantOnlyControlsRequireRecoveryInsteadOfDisappearing() throws {
        let profile = GamepadConfigurationProfile(name: "Different controls", customization: .defaultValue,
            landscapeCustomization: .blankCanvas, updatedAt: 100)
        let data = try JSONEncoder().encode(profile)
        XCTAssertThrowsError(try ThumbleLegacyProfileMigration.prepare(profileData:data,package:nil)) {
            guard case .variantOnlyControls(let ids) = $0 as? ThumbleLegacyProfileMigration.RecoveryRequired else { return XCTFail("Expected control recovery: \($0)") }
            XCTAssertEqual(Set(ids),Set(profile.customization.elements.map(\.id)))
        }
        XCTAssertEqual(profile.customization.elements.count,10)
    }
    func testRealArchiveBecomesPackageIndependentInAllEightStates() throws {
        let package = try ThumbleSkinPackageCodec.decode(Data(contentsOf: root.appendingPathComponent("Host/fixtures/legacy-skin-packages/solar-sumi-1.0.0-edbecde5b708.pocketpad")))
        var profile = GamepadControllerTemplate.arcadeStick.makeProfile()
        profile.applySkin(package)
        let data = try JSONEncoder().encode(profile)
        let result = try ThumbleLegacyProfileMigration.prepare(profileData:data,package:package)
        XCTAssertEqual(result.stagingProfile.appearanceBank?.variants.count,8)
        for scheme in ThumbleSkinColorScheme.allCases { for action in ThumbleActionSet.allCases {
            let before = profile.resolvedCustomization(for:.landscape,colorScheme:scheme,skinPackage:package,actionSet:action)
            let after = result.stagingProfile.resolvedCustomization(for:.landscape,colorScheme:scheme,skinPackage:nil,actionSet:action)
            XCTAssertEqual(GamepadAppearanceSnapshot(before),GamepadAppearanceSnapshot(after))
            XCTAssertEqual(before.elements.map(\.output),after.elements.map(\.output))
        }}
        XCTAssertEqual(result.originalData,data)
    }
}
