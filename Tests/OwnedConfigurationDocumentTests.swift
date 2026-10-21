import Foundation
import SwiftUI
import XCTest

final class OwnedConfigurationDocumentTests: XCTestCase {
    func testOwnedCreationPreservesAllSourceAppearanceAndSelectionOptions() throws {
        let source = try fixture("document")
        let before = try ThumbleOwnedConfigurationDocument(validating: source)
        let original = before.projectedProfiles[0]
        let newID = UUID(uuidString: "EE000000-0000-4000-8000-000000000001")!
        for select in [false,true] { for makeDefault in [false,true] {
            let operation: [String:Any] = ["type":"profile.create", "sourceProfileID":original.id.uuidString,"newProfileID":newID.uuidString,"name":"Owned copy","select":select,"makeDefault":makeDefault]
            let response = try ThumbleOwnedOperationBridge.transform(JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self, from: operationRequest(source,operation)))
            let result = try ThumbleOwnedConfigurationDocument(validating: XCTUnwrap(Data(base64Encoded: response.afterDocumentData)))
            let copy = try XCTUnwrap(result.projectedProfiles.last)
            XCTAssertEqual(Array(result.projectedProfiles.dropLast()),before.projectedProfiles)
            XCTAssertEqual(copy.id,newID); XCTAssertEqual(copy.customization,original.customization)
            XCTAssertEqual(copy.appearanceBank,original.appearanceBank); XCTAssertEqual(copy.appearanceBank?.variants.count,8)
            XCTAssertEqual(copy.outputMode,original.outputMode); XCTAssertNil(copy.launchTarget)
            XCTAssertEqual(result.activeProfileID,select ? newID:before.activeProfileID)
            XCTAssertEqual(result.defaultProfileID,makeDefault ? newID:before.defaultProfileID)
        }}
    }

    func testOwnedDefaultCreationPreservesLowercaseBindingMapKeys() throws {
        var source = try fixture("document")
        let id = "ee000000-0000-4000-8000-000000000002"
        let operations: [[String:Any]] = [
            ["type":"profile.create", "newProfileID":id, "name":"Default", "select":false, "makeDefault":false],
            ["type":"customization.reset", "profileID":id, "variant":"primary"],
            ["type":"binding.reset-all", "profileID":id]
        ]
        for operation in operations {
            let response = try ThumbleOwnedOperationBridge.transform(JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self, from: operationRequest(source,operation)))
            source = try XCTUnwrap(Data(base64Encoded: response.afterDocumentData))
        }
        let root = try XCTUnwrap(JSONSerialization.jsonObject(with: source) as? [String:Any])
        for field in ["profileKeyBindings", "profileOutputBindings"] {
            let maps = try XCTUnwrap(root[field] as? [String:Any])
            XCTAssertNotNil(maps[id]); XCTAssertNil(maps[id.uppercased()])
            XCTAssertFalse(try XCTUnwrap(maps[id] as? [String:Any]).isEmpty)
        }
    }

    func testOwnedApplicationAttachmentPreservesPaintAndLaterClockNoOps() throws {
        var source = try fixture("document")
        let before = try ThumbleOwnedConfigurationDocument(validating: source)
        let profile = before.projectedProfiles[0]
        let attachment: [String:Any] = ["displayName":"World of Warcraft", "filePath":"/Applications/World of Warcraft.app", "bundleIdentifier":"com.example.wow"]
        for value in [attachment as Any, NSNull()] {
            let operation: [String:Any] = ["type":"profile.application-set","profileID":profile.id.uuidString,"attachment":value]
            let request = try JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self,from:operationRequest(source,operation))
            let response = try ThumbleOwnedOperationBridge.transform(request)
            source = try XCTUnwrap(Data(base64Encoded:response.afterDocumentData))
            let after = try ThumbleOwnedConfigurationDocument(validating:source)
            XCTAssertEqual(after.projectedProfiles[0].customization,profile.customization)
            XCTAssertEqual(after.projectedProfiles[0].appearanceBank,profile.appearanceBank)
            XCTAssertEqual(after.projectedProfiles[1],before.projectedProfiles[1])
            XCTAssertEqual(after.activeProfileID,before.activeProfileID)
            let repeatRequest = try JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self,from:operationRequest(source,operation,nowMillis:9000))
            XCTAssertFalse(try ThumbleOwnedOperationBridge.transform(repeatRequest).changed)
        }
        var invalid = attachment; invalid["filePath"] = "relative.app"
        let operation: [String:Any] = ["type":"profile.application-set","profileID":profile.id.uuidString,"attachment":invalid]
        XCTAssertThrowsError(try ThumbleOwnedOperationBridge.transform(JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self,from:operationRequest(source,operation))))
    }

    func testIssueSpecificRepairKeepsPaintAndRespectsLocks() throws {
        for locked in [false,true] {
            var root = try XCTUnwrap(JSONSerialization.jsonObject(with: fixture("document")) as? [String:Any])
            var profiles = try XCTUnwrap(root["profiles"] as? [[String:Any]])
            var layout = try XCTUnwrap(profiles[0]["customization"] as? [String:Any])
            var elements = try XCTUnwrap(layout["elements"] as? [[String:Any]])
            var paint = try XCTUnwrap(elements[0]["layout"] as? [String:Any]); paint["widthScale"]=0.1; paint["heightScale"]=0.1; paint["isLocationLocked"]=locked
            elements[0]["layout"]=paint; layout["elements"]=elements
            var mirrors = try XCTUnwrap(layout["buttonCustomizations"] as? [Any])
            for index in stride(from:0,to:mirrors.count,by:2) where mirrors[index] as? String == elements[0]["id"] as? String { mirrors[index+1]=paint }
            layout["buttonCustomizations"]=mirrors
            var buttons = try XCTUnwrap(layout["customButtons"] as? [[String:Any]])
            for i in buttons.indices where buttons[i]["id"] as? String == elements[0]["id"] as? String { buttons[i]["layout"]=paint }
            layout["customButtons"]=buttons; profiles[0]["customization"]=layout; root["profiles"]=profiles
            let source = try JSONSerialization.data(withJSONObject:root)
            let before = try ThumbleOwnedConfigurationDocument(validating:source); let profile=before.projectedProfiles[0]
            XCTAssertTrue(profile.customization.layoutQualityReport().issues.contains { $0.code == "small-control" })
            var operation: [String:Any] = ["type":"customization.fix","profileID":profile.id.uuidString,"variant":"primary","target":["kind":"issue","code":"small-control"],"canvas":["source":"stored"],"includeLocked":false]
            let response = try ThumbleOwnedOperationBridge.transform(JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self,from:operationRequest(source,operation)))
            let after = try ThumbleOwnedConfigurationDocument(validating:XCTUnwrap(Data(base64Encoded:response.afterDocumentData)))
            XCTAssertEqual(after.projectedProfiles[0].appearanceBank,profile.appearanceBank)
            XCTAssertEqual(after.projectedProfiles[0].customization.elements.map(\.output),profile.customization.elements.map(\.output))
            XCTAssertEqual(after.projectedProfiles[1],before.projectedProfiles[1])
            if locked {
                let a=try XCTUnwrap(JSONSerialization.jsonObject(with:JSONEncoder().encode(after.projectedProfiles[0].customization)) as? [String:Any])
                let b=try XCTUnwrap(JSONSerialization.jsonObject(with:JSONEncoder().encode(profile.customization)) as? [String:Any])
                let changed=Set(a.keys).union(b.keys).filter { !NSDictionary(dictionary:["value":a[$0] ?? NSNull()]).isEqual(to:["value":b[$0] ?? NSNull()]) }.sorted()
                XCTAssertTrue(changed.isEmpty,"Locked repair changed fields: \(changed)")
            }
            else { XCTAssertFalse(after.projectedProfiles[0].customization.layoutQualityReport().issues.contains { $0.code == "small-control" }) }
            operation["target"]=["kind":"issue","code":"unknown-issue"]
            XCTAssertThrowsError(try ThumbleOwnedOperationBridge.transform(JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self,from:operationRequest(source,operation))))
        }
    }

    func testRichBackgroundFillsReplaceRequestedScopeAcrossAllOwnedStates() throws {
        let source = try fixture("document")
        let before = try ThumbleOwnedConfigurationDocument(validating: source)
        let profile = before.projectedProfiles[0]
        let pixel = Data([1, 2, 3, 4])
        let color: [String: Any] = ["red":0.2,"green":0.4,"blue":0.6,"alpha":1]
        let fills: [[String: Any]] = [
            ["kind":"gradient","type":"linear","angleDegrees":45,"stops":[["offset":0,"color":color],["offset":1,"color":color]]],
            ["kind":"tile","pattern":"dots","foregroundColor":color,"backgroundColor":color,"scale":1,"spacingX":0,"spacingY":0,"alignment":"center","opacity":1],
            ["kind":"image","dataBase64":pixel.base64EncodedString(),"fileName":"test.png","contentMode":"fit"]]
        let backgroundFields = Set(["backgroundFillStyle","backgroundLightFillStyle","backgroundDarkFillStyle","backgroundLightColor","backgroundDarkColor"])
        for scope in ["all", "light", "dark"] {
            for fill in fills {
                let edit: [String: Any] = ["action":"fill","scope":scope,"fill":fill]
                let operation: [String: Any] = ["type":"customization.set","profileID":profile.id.uuidString,"variant":"primary","changes":["backgroundEdit":edit]]
                let request = try JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self, from: operationRequest(source, operation))
                let response = try ThumbleOwnedOperationBridge.transform(request)
                let after = try ThumbleOwnedConfigurationDocument(validating: XCTUnwrap(Data(base64Encoded: response.afterDocumentData)))
                XCTAssertEqual(after.projectedProfiles[1], before.projectedProfiles[1])
                let original = try XCTUnwrap(profile.appearanceBank)
                let bank = try XCTUnwrap(after.projectedProfiles[0].appearanceBank)
                XCTAssertEqual(bank.variants.count, 8)
                for (old, row) in zip(original.variants, bank.variants) {
                    let oldPaint = old.appearance.applying(to: profile.customization)
                    let paint = row.appearance.applying(to: after.projectedProfiles[0].customization)
                    for scheme in [ThumbleSkinColorScheme.light, .dark] where scope == "all" || scope == scheme.rawValue {
                        let actual = paint.keypadBackgroundFillStyle(scheme: scheme == .dark ? .dark : .light)
                        switch fill["kind"] as! String {
                        case "image":
                            guard case .image(let image) = actual else { return XCTFail("Missing image in saved state") }
                            XCTAssertEqual(image.data, pixel); XCTAssertEqual(image.contentMode, .fit)
                        case "gradient": guard case .gradient = actual else { return XCTFail("Missing gradient in saved state") }
                        default: guard case .tile = actual else { return XCTFail("Missing tile in saved state") }
                        }
                    }
                    if scope != "all" {
                        let scheme: SwiftUI.ColorScheme = scope == "light" ? .dark : .light
                        XCTAssertEqual(paint.keypadBackgroundFillStyle(scheme: scheme), oldPaint.keypadBackgroundFillStyle(scheme: scheme))
                    }
                    let a = try XCTUnwrap(JSONSerialization.jsonObject(with: JSONEncoder().encode(old.appearance)) as? [String: Any])
                    let b = try XCTUnwrap(JSONSerialization.jsonObject(with: JSONEncoder().encode(row.appearance)) as? [String: Any])
                    XCTAssertEqual(NSDictionary(dictionary: a.filter { !backgroundFields.contains($0.key) }), NSDictionary(dictionary: b.filter { !backgroundFields.contains($0.key) }))
                }
                let replay = try JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self, from: operationRequest(try XCTUnwrap(Data(base64Encoded: response.afterDocumentData)), operation, nowMillis: 9000))
                XCTAssertFalse(try ThumbleOwnedOperationBridge.transform(replay).changed)
                if scope == "all", fill["kind"] as? String == "image" {
                    var root = try XCTUnwrap(JSONSerialization.jsonObject(with: XCTUnwrap(Data(base64Encoded: response.afterDocumentData))) as? [String: Any])
                    var profiles = try XCTUnwrap(root["profiles"] as? [[String: Any]])
                    var bank = try XCTUnwrap(profiles[0]["appearanceBank"] as? [String: Any])
                    var rows = try XCTUnwrap(bank["variants"] as? [[String: Any]])
                    var paint = try XCTUnwrap(rows[3]["appearance"] as? [String: Any])
                    paint["backgroundDarkFillStyle"] = ["kind":"solid","color":color]
                    rows[3]["appearance"] = paint; bank["variants"] = rows; profiles[0]["appearanceBank"] = bank; root["profiles"] = profiles
                    let stateOnly = try JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self, from: operationRequest(JSONSerialization.data(withJSONObject: root), operation, nowMillis: 12000))
                    let corrected = try ThumbleOwnedOperationBridge.transform(stateOnly)
                    XCTAssertTrue(corrected.changed)
                    let correctedDocument = try ThumbleOwnedConfigurationDocument(validating: XCTUnwrap(Data(base64Encoded: corrected.afterDocumentData)))
                    let correctedProfile = correctedDocument.projectedProfiles[0]
                    XCTAssertNil(correctedProfile.appearanceBank?.variants[3].appearance.applying(to: correctedProfile.customization).backgroundDarkFillStyle)
                    var lightOperation = operation
                    lightOperation["changes"] = ["backgroundEdit":["action":"set","scope":"light","color":color]]
                    let lightRequest = try JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self, from: operationRequest(try XCTUnwrap(Data(base64Encoded: response.afterDocumentData)), lightOperation, nowMillis: 13000))
                    let lightResult = try ThumbleOwnedOperationBridge.transform(lightRequest)
                    let lightDocument = try ThumbleOwnedConfigurationDocument(validating: XCTUnwrap(Data(base64Encoded: lightResult.afterDocumentData)))
                    let lightProfile = lightDocument.projectedProfiles[0]
                    for row in try XCTUnwrap(lightProfile.appearanceBank).variants {
                        let layout = row.appearance.applying(to: lightProfile.customization)
                        guard case .solid = layout.keypadBackgroundFillStyle(scheme: .light), case .image(let dark) = layout.keypadBackgroundFillStyle(scheme: .dark) else { return XCTFail("Scheme color must preserve the opposite image") }
                        XCTAssertEqual(dark.data, pixel)
                    }
                    var clearOperation = operation; clearOperation["changes"] = ["backgroundEdit":["action":"clear"]]
                    let clearRequest = try JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self, from: operationRequest(try XCTUnwrap(Data(base64Encoded: lightResult.afterDocumentData)), clearOperation, nowMillis: 14000))
                    let cleared = try ThumbleOwnedOperationBridge.transform(clearRequest)
                    let clearDocument = try ThumbleOwnedConfigurationDocument(validating: XCTUnwrap(Data(base64Encoded: cleared.afterDocumentData)))
                    let clearProfile = clearDocument.projectedProfiles[0]
                    for row in try XCTUnwrap(clearProfile.appearanceBank).variants {
                        let layout = row.appearance.applying(to: clearProfile.customization)
                        XCTAssertNil(layout.backgroundFillStyle); XCTAssertNil(layout.backgroundLightFillStyle); XCTAssertNil(layout.backgroundDarkFillStyle)
                        XCTAssertNil(layout.backgroundLightColor); XCTAssertNil(layout.backgroundDarkColor)
                    }
                }
            }
        }
    }

    func testCustomCanvasTransactionPreservesOwnedControlsAndAppearance() throws {
        let source = try fixture("document")
        let before = try ThumbleOwnedConfigurationDocument(validating: source)
        let selected = try XCTUnwrap(before.projectedProfiles.first)
        let operation: [String: Any] = ["type":"device.set", "profileID":selected.id.uuidString, "variant":"primary", "frameID":"custom-390x844-landscape"]
        let request = try JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self, from: operationRequest(source, operation))
        let result = try ThumbleOwnedOperationBridge.transform(request)
        let after = try ThumbleOwnedConfigurationDocument(validating: XCTUnwrap(Data(base64Encoded: result.afterDocumentData)))
        var expected = selected.customization
        expected.deviceCanvas = GamepadDeviceCanvas(frameID: "custom-390x844-landscape")
        XCTAssertEqual(after.projectedProfiles[0].customization, expected)
        XCTAssertEqual(after.projectedProfiles[0].appearanceBank, selected.appearanceBank)
        XCTAssertEqual(after.projectedProfiles[0].launchTarget, selected.launchTarget)
        XCTAssertEqual(after.projectedProfiles[1], before.projectedProfiles[1])
        let repeated = try JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self, from: operationRequest(try XCTUnwrap(Data(base64Encoded: result.afterDocumentData)), operation, nowMillis: 9000))
        let noOp = try ThumbleOwnedOperationBridge.transform(repeated)
        XCTAssertFalse(noOp.changed)
        XCTAssertTrue(noOp.beforeDocumentData == noOp.afterDocumentData)
        for invalid in ["custom-844x390-landscape", "custom-239x844-landscape", "custom-0390x844-landscape", "custom-390x844-portrait"] {
            var operation = operation; operation["frameID"] = invalid
            let bad = try JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self, from: operationRequest(source, operation))
            XCTAssertThrowsError(try ThumbleOwnedOperationBridge.transform(bad), invalid)
        }
    }

    private func legacyImportFixture() throws -> Data {
        try Data(contentsOf: root.appendingPathComponent("Host/fixtures/customization-interchange/v1/legacy-request.json"))
    }
    func testLegacyCustomizationImportMatchesCapturedResponse() throws {
        let request = try JSONDecoder().decodeUnique(ThumbleConfigurationBridgeRequest.self, from: legacyImportFixture())
        let actual = try ThumbleConfigurationBridge.transform(request)
        let expected = try JSONDecoder().decodeUnique(ThumbleConfigurationBridgeResponse.self, from: Data(contentsOf: root.appendingPathComponent("Host/fixtures/customization-interchange/v1/legacy-response.json")))
        XCTAssertEqual(actual, expected)
    }
    func testLegacyCustomizationImportRetainsBindingExtensionsAndRefusesTheirDeletion() throws {
        var request = try XCTUnwrap(JSONSerialization.jsonObject(with: legacyImportFixture()) as? [String: Any])
        var document = try XCTUnwrap(request["document"] as? [String: Any])
        let profiles = try XCTUnwrap(document["profiles"] as? [[String: Any]])
        let profile = try JSONDecoder().decodeUnique(GamepadConfigurationProfile.self, from: JSONSerialization.data(withJSONObject: profiles[0]))
        let outputs = profile.initialMacOutputBindings
        var raw = try XCTUnwrap(JSONSerialization.jsonObject(with: JSONEncoder().encode(MacConfigurationBindings.rawOutputs(outputs))) as? [String: [String: Any]])
        let id = try XCTUnwrap(raw.keys.first(where: { !$0.hasPrefix("00000000") }))
        var value = try XCTUnwrap(raw.removeValue(forKey: id)); value["futureRetained"] = "keep binding metadata"
        raw[id.lowercased()] = value
        let keys = try JSONSerialization.jsonObject(with: JSONEncoder().encode(MacConfigurationBindings.rawKeyBindings(outputs.keyboardBindings)))
        document["profileOutputBindings"] = [profile.id.uuidString:raw]; document["outputBindings"] = raw
        document["profileKeyBindings"] = [profile.id.uuidString:keys]; document["keyBindings"] = keys
        request["document"] = document
        let decoded = try JSONDecoder().decodeUnique(ThumbleConfigurationBridgeRequest.self, from: JSONSerialization.data(withJSONObject: request))
        let result = try ThumbleConfigurationBridge.transform(decoded)
        guard case .object(let map)? = result.document.profileOutputBindings[profile.id.uuidString],
              case .object(let row)? = map[id.lowercased()] else { return XCTFail("binding key/metadata disappeared") }
        XCTAssertEqual(row["futureRetained"], .string("keep binding metadata"))
        var operation = try XCTUnwrap(request["operation"] as? [String: Any])
        var layout = try XCTUnwrap(JSONSerialization.jsonObject(with: Data((operation["customizationJSON"] as! String).utf8)) as? [String: Any])
        var elements = try XCTUnwrap(layout["elements"] as? [[String: Any]])
        elements.removeAll { ($0["id"] as? String)?.lowercased() == id.lowercased() }; layout["elements"] = elements
        if var customs = layout["customButtons"] as? [[String: Any]] { customs.removeAll { ($0["id"] as? String)?.lowercased() == id.lowercased() }; layout["customButtons"] = customs }
        operation["customizationJSON"] = String(decoding: try JSONSerialization.data(withJSONObject: layout), as: UTF8.self); request["operation"] = operation
        let deletion = try JSONDecoder().decodeUnique(ThumbleConfigurationBridgeRequest.self, from: JSONSerialization.data(withJSONObject: request))
        XCTAssertThrowsError(try ThumbleConfigurationBridge.transform(deletion))
    }
    func testResetLeavesUnrelatedOwnedBankUntouched() throws {
        let source = try fixture("document")
        let before = try ThumbleOwnedConfigurationDocument(validating: source)
        let id = try XCTUnwrap(before.projectedProfiles.last).id.uuidString
        for type in ["customization.reset", "profile.reset"] {
            var operation: [String: Any] = ["type":type, "profileID":id]
            if type == "customization.reset" { operation["variant"] = "primary" }
            let request = try JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self, from: operationRequest(source, operation))
            let result = try ThumbleOwnedOperationBridge.transform(request)
            let after = try ThumbleOwnedConfigurationDocument(validating: XCTUnwrap(Data(base64Encoded: result.afterDocumentData)))
            XCTAssertEqual(after.projectedProfiles.first, before.projectedProfiles.first)
        }
    }
    func testCustomizationResetClearsStateOnlyAppearance() throws {
        let original = try fixture("document")
        var root = try XCTUnwrap(JSONSerialization.jsonObject(with: original) as? [String: Any])
        var profiles = try XCTUnwrap(root["profiles"] as? [[String: Any]])
        var bank = try XCTUnwrap(profiles[0]["appearanceBank"] as? [String: Any])
        var variants = try XCTUnwrap(bank["variants"] as? [[String: Any]])
        var appearance = try XCTUnwrap(variants[1]["appearance"] as? [String: Any])
        var controls = try XCTUnwrap(appearance["controls"] as? [String: [String: Any]])
        controls["00000000-0000-0000-0000-000000000109"]?["cornerRadius"] = 44
        appearance["controls"] = controls; variants[1]["appearance"] = appearance
        bank["variants"] = variants; profiles[0]["appearanceBank"] = bank; root["profiles"] = profiles
        let source = try JSONSerialization.data(withJSONObject: root)
        for type in ["customization.reset", "profile.reset"] {
            var operation: [String: Any] = ["type":type, "profileID":profiles[0]["id"]!]
            if type == "customization.reset" { operation["variant"] = "primary" }
            let bytes = try operationRequest(source, operation)
            let request = try JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self, from: bytes)
            let result = try ThumbleOwnedOperationBridge.transform(request)
            let after = try ThumbleOwnedConfigurationDocument(validating: XCTUnwrap(Data(base64Encoded: result.afterDocumentData)))
            let profile = try XCTUnwrap(after.projectedProfiles.first)
            let repeatedBytes = try operationRequest(XCTUnwrap(Data(base64Encoded: result.afterDocumentData)), operation, nowMillis: 3000)
            let repeated = try ThumbleOwnedOperationBridge.transform(JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self, from: repeatedBytes))
            XCTAssertFalse(repeated.changed)
            XCTAssertEqual(repeated.afterDocumentData, result.afterDocumentData)
            let id = UUID(uuidString: "00000000-0000-0000-0000-000000000109")!
            let expected = try XCTUnwrap(GamepadCustomization.defaultValue.elements.first(where: { $0.id == id }))
            for scheme in ThumbleSkinColorScheme.allCases { for action in ThumbleActionSet.allCases {
                let layout = profile.resolvedCustomization(for: .landscape, colorScheme: scheme, skinPackage: nil, actionSet: action)
                let control = try XCTUnwrap(layout.elements.first(where: { $0.id == id }))
                XCTAssertEqual(control.layout.cornerRadius, expected.layout.cornerRadius, "\(scheme)/\(action)")
                XCTAssertEqual(layout.artworkLayers, GamepadCustomization.defaultValue.artworkLayers)
                XCTAssertEqual(layout.styleLibrary, GamepadCustomization.defaultValue.styleLibrary)
            } }
        }
    }

    func testOwnedResetMatchesFrozenNativeResponses() throws {
        let directory = root.appendingPathComponent("Host/fixtures/customization-reset/v1")
        for name in ["plain", "banked", "plain-profile", "banked-profile"] {
            let request = try JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self, from: Data(contentsOf: directory.appendingPathComponent(name + "-request.json")))
            let result = try ThumbleOwnedOperationBridge.transform(request)
            let expected = try XCTUnwrap(JSONSerialization.jsonObject(with: Data(contentsOf: directory.appendingPathComponent(name + "-response.json"))) as? [String: Any])
            XCTAssertEqual(result.beforeDocumentData, expected["beforeDocumentData"] as? String)
            XCTAssertEqual(result.afterDocumentData, expected["afterDocumentData"] as? String)
        }
    }
    func testStyleImportMatchesFrozenNativeResponse() throws {
        let directory = root.appendingPathComponent("Host/fixtures/style-library/v1")
        for name in ["plain", "banked"] {
            let data = try Data(contentsOf: directory.appendingPathComponent(name + "-request.json"))
            let request = try JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self, from: data)
            let result = try ThumbleOwnedOperationBridge.transform(request)
            let expected = try XCTUnwrap(JSONSerialization.jsonObject(with: Data(contentsOf: directory.appendingPathComponent(name + "-response.json"))) as? [String: Any])
            XCTAssertEqual(result.beforeDocumentData, expected["beforeDocumentData"] as? String)
            XCTAssertEqual(result.afterDocumentData, expected["afterDocumentData"] as? String)
        }
    }
    func testStyleFilePoolsRepeatedMaximumImagesAndRetainsRichStates() throws {
        let data = Data(repeating: 42, count: GamepadImageFill.maximumStoredBytes)
        let style = GamepadControlVisualStyle(normal: .init(fillStyle: .image(.init(data: data))),
            pressed: .init(scale: 0.94), hapticFeedback: .init(style: .heavy, pattern: .double, intensity: 0.73, sharpness: 0.42, duration: 0.17))
        let library = GamepadStyleLibrary(styles: (0..<12).map { .init(id: "style-\($0)", name: "Style \($0)", visualStyle: style) })
        let bytes = try GamepadStyleLibraryFile.write(library)
        let record = try XCTUnwrap(JSONSerialization.jsonObject(with: bytes) as? [String: Any])
        XCTAssertEqual((record["appearanceResources"] as? [String: String])?.count, 1)
        XCTAssertLessThan(bytes.count, 4 * 1024 * 1024)
        XCTAssertEqual(try GamepadStyleLibraryFile.read(bytes), library.normalized)
        XCTAssertThrowsError(try JSONDecoder().decodeUnique(GamepadConfigurationProfile.self, from: bytes))
        var corrupt = record
        corrupt["appearanceResources"] = [:]
        XCTAssertThrowsError(try GamepadStyleLibraryFile.read(JSONSerialization.data(withJSONObject: corrupt)))
        let classic = GamepadStyleLibrary(styles: [library.styles[0]])
        XCTAssertEqual(try GamepadStyleLibraryFile.read(JSONEncoder().encode(classic)), classic.normalized)
        var unsupported = try XCTUnwrap(JSONSerialization.jsonObject(with: JSONEncoder().encode(classic)) as? [String: Any])
        var styles = try XCTUnwrap(unsupported["styles"] as? [[String: Any]])
        styles[0]["futureUnsupported"] = "keep original source"
        unsupported["styles"] = styles
        XCTAssertThrowsError(try GamepadStyleLibraryFile.read(JSONSerialization.data(withJSONObject: unsupported)))
    }
    private var root: URL { URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent() }
    private func fixture(_ name: String) throws -> Data {
        try Data(contentsOf: root.appendingPathComponent("Host/fixtures/owned-configuration/v1/\(name).json"))
    }
    private func operationRequest(_ source: Data, _ operation: [String: Any], nowMillis: Int = 2000) throws -> Data {
        try JSONSerialization.data(withJSONObject: ["schemaVersion":1, "invocationID":"AC000000-0000-0000-0000-000000000001", "ownedConfigurationCommand":"apply", "documentData":source.base64EncodedString(), "operation":operation, "nowMillis":nowMillis], options: [.sortedKeys, .withoutEscapingSlashes])
    }
    func testReviewedOwnedDesignCreatesEightStatesWithoutChangingFeedbackOrOutputs() throws {
        let directory = root.appendingPathComponent("Host/fixtures/owned-design/v1")
        let bytes = try Data(contentsOf: directory.appendingPathComponent("apply-request.json"))
        let request = try JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self, from: bytes)
        let response = try ThumbleOwnedOperationBridge.transform(request)
        let expected = try XCTUnwrap(JSONSerialization.jsonObject(with: Data(contentsOf: directory.appendingPathComponent("apply-response.json"))) as? [String: Any])
        XCTAssertEqual(response.beforeDocumentData, expected["beforeDocumentData"] as? String)
        XCTAssertEqual(response.afterDocumentData, expected["afterDocumentData"] as? String)
        let before = try ThumbleOwnedConfigurationDocument(validating: XCTUnwrap(Data(base64Encoded: response.beforeDocumentData)))
        let after = try ThumbleOwnedConfigurationDocument(validating: XCTUnwrap(Data(base64Encoded: response.afterDocumentData)))
        let original = try XCTUnwrap(before.projectedProfiles.last)
        let edited = try XCTUnwrap(after.projectedProfiles.last)
        XCTAssertEqual(before.projectedProfiles.first, after.projectedProfiles.first)
        XCTAssertNil(edited.skinReference)
        XCTAssertNil(edited.landscapeCustomization)
        XCTAssertNil(edited.portraitCustomization)
        XCTAssertEqual(edited.customization.elements.map(\.output), original.customization.elements.map(\.output))
        XCTAssertEqual(edited.customization.elements.map(\.id), original.customization.elements.map(\.id))
        XCTAssertEqual(edited.appearanceBank?.variants.count, 8)
        let id = UUID(uuidString: "00000000-0000-0000-0000-000000000105")!
        let originalFeedback = try XCTUnwrap(original.customization.elements.first(where: { $0.id == id })).layout.hapticFeedback
        for scheme in ThumbleSkinColorScheme.allCases { for action in ThumbleActionSet.allCases {
            let resolved = edited.resolvedCustomization(for: .landscape, colorScheme: scheme, skinPackage: nil, actionSet: action)
            let control = try XCTUnwrap(resolved.elements.first(where: { $0.id == id }))
            XCTAssertEqual(try XCTUnwrap(control.layout.centerX), CGFloat(0.61), accuracy: 0.000001)
            XCTAssertEqual(control.layout.hapticFeedback, originalFeedback)
        } }
        for field in ["baseProfileSHA256", "profileSHA256", "packageSHA256"] {
            var invalid = try XCTUnwrap(JSONSerialization.jsonObject(with: bytes) as? [String: Any])
            var operation = try XCTUnwrap(invalid["operation"] as? [String: Any])
            operation[field] = String(repeating: "0", count: 64); invalid["operation"] = operation
            let decoded = try JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self,
                from: JSONSerialization.data(withJSONObject: invalid))
            XCTAssertThrowsError(try ThumbleOwnedOperationBridge.transform(decoded), field)
        }
    }
    func testOwnedOperationBridgeEditsGeometryHapticsAndControlIdentity() throws {
        let source = try fixture("document")
        let document = try ThumbleOwnedConfigurationDocument(validating: source)
        let profile = try XCTUnwrap(document.projectedProfiles.first)
        let element = try XCTUnwrap(profile.customization.elements.first)
        let id = profile.id.uuidString
        let newID = "AC000000-0000-0000-0000-000000000002"
        let cases: [(String, [String: Any])] = [
            ("nullable-select", ["type":"profile.select", "profileID":try XCTUnwrap(document.projectedProfiles.last).id.uuidString]),
            ("geometry", ["type":"element.nudge", "profileID":id, "variant":"landscape", "elementIDs":[element.id.uuidString], "deltaX":0.02, "deltaY":0.03]),
            ("haptic", ["type":"element.set", "profileID":id, "variant":"primary", "elementID":element.id.uuidString, "changes":["haptic":["style":"light", "intensity":0.42]]]),
            ("add", ["type":"element.add", "profileID":id, "variant":"landscape", "elementID":newID, "kind":"button", "changes":["label":"New action"]]),
            ("delete", ["type":"element.delete", "profileID":id, "variant":"primary", "elementID":newID]),
            ("scheme", ["type":"customization.set", "profileID":id, "variant":"landscape", "changes":["colorScheme":"dark", "backgroundEdit":["action":"keep"]]])
        ]
        let output = root.appendingPathComponent("build/OwnedOperationBridge")
        try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
        var addedSource: Data?
        for (name, operation) in cases {
            var input = name == "delete" ? try XCTUnwrap(addedSource) : source
            if name == "nullable-select" {
                input = Data(String(decoding:source,as:UTF8.self).replacingOccurrences(of: "\"defaultProfileID\"\\s*:\\s*\"[^\"]+\"", with: "\"defaultProfileID\":null", options:.regularExpression).utf8)
            }
            let expectedBefore = try ThumbleOwnedConfigurationDocument(validating: input)
            let baseProfile = try XCTUnwrap(expectedBefore.projectedProfiles.first)
            let requestBytes = try operationRequest(input, operation)
            let request = try JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self, from: requestBytes)
            let response = try ThumbleOwnedOperationBridge.transform(request)
            XCTAssertTrue(response.changed, name)
            XCTAssertEqual(response.sourceSHA256, ProfileArtifactAdoptionMetadata.sha256(input))
            let before = try ThumbleOwnedConfigurationDocument(validating: XCTUnwrap(Data(base64Encoded: response.beforeDocumentData)))
            let after = try ThumbleOwnedConfigurationDocument(validating: XCTUnwrap(Data(base64Encoded: response.afterDocumentData)))
            XCTAssertEqual(before.projectedProfiles, expectedBefore.projectedProfiles, name)
            XCTAssertEqual(after.projectedProfiles.count, before.projectedProfiles.count)
            if name == "nullable-select" {
                XCTAssertNil(before.defaultProfileID); XCTAssertNil(after.defaultProfileID)
                XCTAssertEqual(after.activeProfileID,document.projectedProfiles.last?.id)
            }
            let updated = try XCTUnwrap(after.projectedProfiles.first)
            let expected = name == "add" ? Set(profile.customization.elements.map(\.id)).union([UUID(uuidString:newID)!]) : (name == "delete" ? Set(baseProfile.customization.elements.map(\.id)).subtracting([UUID(uuidString:newID)!]) : Set(profile.customization.elements.map(\.id)))
            XCTAssertEqual(Set(updated.customization.elements.map(\.id)), expected, name)
            for scheme in ThumbleSkinColorScheme.allCases { for action in ThumbleActionSet.allCases {
                let resolved = updated.resolvedCustomization(for: .landscape, colorScheme: scheme, skinPackage: nil, actionSet: action)
                XCTAssertEqual(Set(resolved.elements.map(\.id)), expected, name)
                if name == "geometry" { XCTAssertEqual(resolved.elements.map(\.output), profile.customization.elements.map(\.output)) }
                if name == "haptic" { XCTAssertEqual(resolved.elements.first?.layout.hapticFeedback?.intensity, 0.42) }
            }}
            if name == "add" { addedSource = Data(base64Encoded: response.afterDocumentData) }
            try requestBytes.write(to: output.appendingPathComponent("\(name)-request.json"), options:.atomic)
            let encoder = JSONEncoder(); encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
            let responseBytes = try encoder.encode(response)
            try responseBytes.write(to:output.appendingPathComponent("\(name)-response.json"), options:.atomic)
            let frozen = root.appendingPathComponent("Host/fixtures/owned-native-operations/v1")
            XCTAssertEqual(requestBytes, try Data(contentsOf:frozen.appendingPathComponent("\(name)-request.json")), name)
            XCTAssertEqual(responseBytes, try Data(contentsOf:frozen.appendingPathComponent("\(name)-response.json")), name)
        }
    }
    func testOwnedOperationBridgeRejectsPortraitAndUnexpectedFields() throws {
        let source = try fixture("document")
        let id = try ThumbleOwnedConfigurationDocument(validating: source).activeProfileID.uuidString
        for op in [["type":"orientation.set", "profileID":id, "preference":"portrait"], ["type":"customization.reset", "profileID":id, "variant":"portrait"]] {
            let request = try JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self, from: operationRequest(source,op))
            XCTAssertThrowsError(try ThumbleOwnedOperationBridge.transform(request))
        }
        var root = try XCTUnwrap(JSONSerialization.jsonObject(with: operationRequest(source,["type":"profile.select", "profileID":id])) as? [String:Any])
        root["path"] = "/tmp/unused"
        XCTAssertThrowsError(try JSONDecoder().decodeUnique(ThumbleOwnedOperationBridgeRequest.self, from: JSONSerialization.data(withJSONObject: root)))
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
