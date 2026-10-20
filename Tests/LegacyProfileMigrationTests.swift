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
