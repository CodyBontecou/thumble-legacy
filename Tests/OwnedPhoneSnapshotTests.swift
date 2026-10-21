import Foundation
import XCTest

final class OwnedPhoneSnapshotTests: XCTestCase {
    private var root: URL { URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent() }
    private func fixture() throws -> Data { try Data(contentsOf: root.appendingPathComponent("Host/fixtures/owned-configuration/v1/document.json")) }
    private func snapshot() throws -> OwnedPhoneSnapshot {
        try .init(serverID: "mac-a", configurationRevision: 41, documentData: fixture())
    }
    private func wire() throws -> Data {
        let fields = try JSONDecoder.rawObjectFields(in: fixture(), named: ["profiles", "activeProfileID", "defaultProfileID"])
        var bytes = Data(#"{"type":"pairing_accepted","timestamp":0,"profileFormatVersion":5,"configurationRevision":18446744073709551615,"serverID":"mac-a","authToken":"must-not-enter-cache","gamepadProfiles":"#.utf8)
        bytes.append(try XCTUnwrap(fields["profiles"]))
        bytes.append(Data(#", "gamepadProfileID":"#.utf8)); bytes.append(try XCTUnwrap(fields["activeProfileID"]))
        bytes.append(Data(#", "defaultGamepadProfileID":"#.utf8)); bytes.append(try XCTUnwrap(fields["defaultProfileID"]))
        bytes.append(125)
        return bytes
    }
    func testRootTokenExtractionKeepsOpaqueNumbersEscapesAndWhitespace() throws {
        let original = #"{"ignore":"secret","gamepad\u0050rofiles": [ {"opaque":1.00000000000000000001,"nested":["}\\\"",true]} ],"active":18446744073709551615}"#
        let fields = try JSONDecoder.rawObjectFields(in: Data(original.utf8), named: ["gamepadProfiles", "active"])
        XCTAssertEqual(String(decoding: try XCTUnwrap(fields["gamepadProfiles"]), as: UTF8.self), #"[ {"opaque":1.00000000000000000001,"nested":["}\\\"",true]} ]"#)
        XCTAssertEqual(String(decoding: try XCTUnwrap(fields["active"]), as: UTF8.self), "18446744073709551615")
        XCTAssertNil(fields["ignore"])
        XCTAssertThrowsError(try JSONDecoder.rawObjectFields(in: Data(#"{"a":1,"\u0061":2}"#.utf8), named: ["a"]))
        XCTAssertThrowsError(try JSONDecoder.rawObjectFields(in: Data(#"{"a":[1,]}"#.utf8), named: ["a"]))
    }
    func testSnapshotRetainsCanonicalRecordsAndNeverCachesAuthentication() throws {
        let value = try OwnedPhoneSnapshot(wireData: wire(), fromAuthenticatedServerID: "mac-a")
        XCTAssertEqual(value.configurationRevision, UInt64.max)
        XCTAssertEqual(value.profiles.count, 2)
        XCTAssertEqual(value.profiles.first?.appearanceBank?.variants.count, 8)
        let doc = try XCTUnwrap(JSONSerialization.jsonObject(with: value.documentData) as? [String: Any])
        XCTAssertNil(doc["authToken"]); XCTAssertNil(doc["serverID"])
        let profile = try XCTUnwrap((doc["profiles"] as? [[String: Any]])?.first)
        XCTAssertEqual(profile["profileFormatVersion"] as? Int, 5)
        XCTAssertNil(profile["orientationPreference"]); XCTAssertNil(profile["landscapeCustomization"])
        let sourceFields = try JSONDecoder.rawObjectFields(in: fixture(), named: ["profiles"])
        let cachedFields = try JSONDecoder.rawObjectFields(in: value.documentData, named: ["profiles"])
        XCTAssertEqual(sourceFields["profiles"], cachedFields["profiles"])
        XCTAssertThrowsError(try OwnedPhoneSnapshot(wireData: wire(), fromAuthenticatedServerID: "mac-b"))
    }
    func testSnapshotRejectsLegacyScalarAndFloatingRevision() throws {
        let bytes = try wire()
        let text = String(decoding: bytes, as: UTF8.self)
        for bad in [text.replacingOccurrences(of: "18446744073709551615", with: "41.0"),
                    text.replacingOccurrences(of: "\"profileFormatVersion\":5", with: "\"profileFormatVersion\":4")] {
            XCTAssertThrowsError(try OwnedPhoneSnapshot(wireData: Data(bad.utf8), fromAuthenticatedServerID: "mac-a"))
        }
        XCTAssertThrowsError(try OwnedPhoneSnapshot(wireData: Data(#"{"type":"gamepad_customization","profileFormatVersion":5,"configurationRevision":41,"gamepadProfileID":"00000000-0000-0000-0000-000000000001","gamepadCustomization":{}}"#.utf8), fromAuthenticatedServerID: "mac-a"))
    }
    func testAtomicCacheRestoresSourceAndEditsAndRefusesToOverwriteCorruption() throws {
        let value = try snapshot()
        let layout = try XCTUnwrap(value.profiles.first).customization
        var after = layout; let element = try XCTUnwrap(after.elements.first)
        var paint = element.layout; paint.centerX = 0.62
        if element.kind == .button, let builtin = element.defaultControlID { after.setButtonCustomization(paint, for: builtin) }
        else { after.elements[0].layout = paint; if let i = after.customButtons.firstIndex(where: { $0.id == element.id }) { after.customButtons[i].layout = paint } }
        let intent = try XCTUnwrap(OwnedPhoneLayoutAdapter.intent(profileID: value.profiles[0].id, before: layout, after: after, baseConfigurationRevision: 41))
        let journal = try OwnedPhoneEditJournal().recording(serverID: "mac-a", edit: intent, updatedAt: 100, recoveryDocument: value.documentData)
        let cache = OwnedPhoneCache(snapshot: value, journal: journal)
        let restored = try OwnedPhoneCache(validating: cache.encoded())
        XCTAssertEqual(restored.snapshot?.documentData, value.documentData)
        XCTAssertEqual(restored.journal.pending.first?.edit.originalData, intent.originalData)
        XCTAssertEqual(restored.snapshot?.configurationRevision, 41)
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let file = directory.appendingPathComponent("owned-cache.json")
        try cache.save(to: file)
        XCTAssertEqual(try OwnedPhoneCache.load(from: file).snapshot?.documentData, value.documentData)
        try Data("corrupt".utf8).write(to: file)
        XCTAssertThrowsError(try OwnedPhoneCache(snapshot: nil, journal: .init()).save(to: file))
        XCTAssertEqual(try Data(contentsOf: file), Data("corrupt".utf8))
    }
    func testAdapterOnlyAcceptsScalarGeometryAndKeepsBankOutputsAndPaint() throws {
        let value = try snapshot(); let profile = try XCTUnwrap(value.profiles.first)
        let before = profile.customization
        var after = before
        let element = try XCTUnwrap(after.elements.first)
        var paint = element.layout; paint.centerX = 0.64; paint.widthScale = 1.3; paint.isHidden = true
        if element.kind == .button, let builtin = element.defaultControlID { after.setButtonCustomization(paint, for: builtin) }
        else { after.elements[0].layout = paint; if let i = after.customButtons.firstIndex(where: { $0.id == element.id }) { after.customButtons[i].layout = paint } }
        let intent = try XCTUnwrap(OwnedPhoneLayoutAdapter.intent(profileID: profile.id, before: before, after: after, baseConfigurationRevision: UInt64.max))
        XCTAssertEqual(intent.baseConfigurationRevision, UInt64.max)
        XCTAssertEqual(intent.controlEdits.count, 1)
        let applied = try OwnedPhoneLayoutAdapter.applying(intent, to: before)
        XCTAssertEqual(applied, after.normalized)
        XCTAssertEqual(applied.assetLibrary, before.assetLibrary)
        XCTAssertEqual(applied.elements.first?.output, before.elements.first?.output)
        var repaint = after; repaint.showsButtonLabels.toggle()
        XCTAssertThrowsError(try OwnedPhoneLayoutAdapter.intent(profileID: profile.id, before: before, after: repaint, baseConfigurationRevision: 41))
        var renamed = after; renamed.setLabel("Cannot smuggle a label", for: element.inputID)
        XCTAssertThrowsError(try OwnedPhoneLayoutAdapter.intent(profileID: profile.id, before: before, after: renamed, baseConfigurationRevision: 41))
        XCTAssertNil(try OwnedPhoneLayoutAdapter.intent(profileID: profile.id, before: before, after: before.stampedForLocalUpdate, baseConfigurationRevision: 41))
    }
    private func edited(_ layout: GamepadCustomization, x: CGFloat? = nil, width: CGFloat? = nil) throws -> GamepadCustomization {
        var result = layout
        let element = try XCTUnwrap(layout.elements.first)
        var paint = element.layout
        if let x { paint.centerX = x }; if let width { paint.widthScale = width }
        if element.kind == .button, let builtin = element.defaultControlID { result.setButtonCustomization(paint, for: builtin) }
        else { result.elements[0].layout = paint; if let i = result.customButtons.firstIndex(where: { $0.id == element.id }) { result.customButtons[i].layout = paint } }
        return result
    }
    private func receipt(_ edit: OwnedPhoneEdit, status: String, current: UInt64, committed: UInt64? = nil) throws -> ControllerMessage {
        var result: [String: Any] = ["schemaVersion":1, "editID":edit.editID.uuidString, "profileID":edit.profileID.uuidString,
            "baseConfigurationRevision":NSNumber(value:edit.baseConfigurationRevision), "configurationRevision":NSNumber(value:current),
            "status":status, "changed": committed.map { $0 > edit.baseConfigurationRevision } ?? false]
        if let committed { result["committedRevision"] = NSNumber(value:committed) }
        else { result["errorCode"] = "owned_phone_revision_conflict" }
        return try ControllerWireCodec.decode(JSONSerialization.data(withJSONObject:["type":"owned_phone_edit_result", "timestamp":0,
            "profileFormatVersion":5,"configurationRevision":NSNumber(value:current),"ownedPhoneEditResult":result]), using: JSONDecoder())
    }
    private func updatedSnapshot(_ layout: GamepadCustomization, revision: UInt64) throws -> OwnedPhoneSnapshot {
        var root = try XCTUnwrap(JSONSerialization.jsonObject(with: fixture()) as? [String:Any])
        var profiles = try XCTUnwrap(root["profiles"] as? [[String:Any]])
        var native = try XCTUnwrap(ThumbleOwnedConfigurationDocument(validating:fixture()).projectedProfiles.first)
        native.customization = layout
        if let bank = native.appearanceBank {
            let paint = GamepadAppearanceSnapshot(layout)
            native.appearanceBank = try GamepadAppearanceBank(variants:bank.variants.map {
                .init(colorScheme:$0.colorScheme,actionSet:$0.actionSet,appearance:$0.appearance.reconcilingControls(with:paint,duplicates:[:]))
            })
        }
        var record = try XCTUnwrap(JSONSerialization.jsonObject(with:JSONEncoder().encode(native)) as? [String:Any])
        for key in ["skinReference","skinBaselineCustomization","landscapeSkinBaselineCustomization","portraitSkinBaselineCustomization","landscapeCustomization","portraitCustomization","orientationPreference"] { record.removeValue(forKey:key) }
        record["profileFormatVersion"] = 5; record["name"] = "Fresh character metadata"
        profiles[0] = record
        let declared = Set(layout.elements.map(\.id))
        for field in ["profileKeyBindings","profileOutputBindings"] {
            if var maps = root[field] as? [String:Any] {
                for key in maps.keys where UUID(uuidString:key) == native.id {
                    if let bindings = maps[key] as? [String:Any] { maps[key] = bindings.filter { UUID(uuidString:$0.key).map(declared.contains) ?? false } }
                }
                root[field] = maps
            }
        }
        if (root["activeProfileID"] as? String).flatMap(UUID.init(uuidString:)) == native.id {
            for field in ["keyBindings","outputBindings"] {
                if let bindings = root[field] as? [String:Any] { root[field] = bindings.filter { UUID(uuidString:$0.key).map(declared.contains) ?? false } }
            }
        }
        root["profiles"] = profiles
        return try .init(serverID:"mac-a",configurationRevision:revision,documentData:JSONSerialization.data(withJSONObject:root))
    }
    func testSessionKeepsSnapshotsSeparateFromAcknowledgmentsAndRetriesOnlyAfterConflict() throws {
        let original = try snapshot(); let profile = try XCTUnwrap(original.profiles.first)
        let initial = OwnedPhoneSession(cache:.init(snapshot:original,journal:.init()))
        let moved = try edited(profile.customization,x:0.66)
        let one = try initial.recording(profileID:profile.id,customization:moved,nowMillis:100)
        let resized = try edited(moved,width:1.7)
        let two = try one.recording(profileID:profile.id,customization:resized,nowMillis:101)
        XCTAssertEqual(two.cache.journal.pending.count,2)
        let first = two.cache.journal.pending[0].edit
        let second = two.cache.journal.pending[1].edit
        let packet = try OwnedPhoneSession.wireData(for:two.cache.journal.pending[0])
        let raw = try JSONDecoder.rawObjectFields(in:packet,named:["ownedPhoneEdit"])
        XCTAssertEqual(raw["ownedPhoneEdit"],first.originalData)
        let success = try receipt(first,status:"applied",current:42,committed:42)
        XCTAssertThrowsError(try two.acknowledging(success,fromAuthenticatedServerID:"mac-a"))
        XCTAssertEqual(two.cache.journal.pending.count,2)
        let fresh = try updatedSnapshot(moved,revision:42)
        let received = two.receiving(fresh)
        XCTAssertEqual(received.cache.journal.pending.count,2,"A matching snapshot is not an acknowledgment")
        let acked = try received.acknowledging(success,fromAuthenticatedServerID:"mac-a")
        XCTAssertEqual(acked.cache.journal.pending.count,1)
        XCTAssertEqual(acked.profiles[0].customization.elements[0].layout.widthScale,1.7)
        XCTAssertEqual(acked.profiles[0].name,"Fresh character metadata")
        let conflict = try receipt(second,status:"conflict",current:42)
        let retried = try acked.retryingConflict(conflict,fromAuthenticatedServerID:"mac-a")
        XCTAssertNotEqual(retried.cache.journal.pending[0].edit.editID,second.editID)
        XCTAssertEqual(retried.cache.journal.pending[0].edit.baseConfigurationRevision,42)
        XCTAssertEqual(acked.cache.journal.pending[0].edit.originalData,second.originalData)
        XCTAssertEqual(retried.cache.journal.pending[0].recoveryDocument,fresh.documentData)
        XCTAssertThrowsError(try two.retryingConflict(try receipt(first,status:"conflict",current:42),fromAuthenticatedServerID:"mac-a"))
        let roundtrip = OwnedPhoneSession(cache:try OwnedPhoneCache(validating:retried.cache.encoded()))
        XCTAssertEqual(roundtrip.cache.journal.pending[0].edit.originalData,retried.cache.journal.pending[0].edit.originalData)
        XCTAssertEqual(roundtrip.profiles[0].customization.elements[0].layout.widthScale,1.7)
    }
    func testMissingControlKeepsRecoveryAndCannotBeRecreatedByRetry() throws {
        let original = try snapshot(); let profile = try XCTUnwrap(original.profiles.first)
        let initial = OwnedPhoneSession(cache:.init(snapshot:original,journal:.init()))
        let pending = try initial.recording(profileID:profile.id,customization:edited(profile.customization,x:0.67),nowMillis:100)
        let edit = try XCTUnwrap(pending.cache.journal.pending.first?.edit)
        var removed = profile.customization; removed.removeCustomButton(id:removed.elements[0].id)
        removed.buttonCustomizations.removeValue(forKey:profile.customization.elements[0].inputID)
        removed.labelOverrides.removeValue(forKey:profile.customization.elements[0].inputID)
        let fresh = try updatedSnapshot(removed.normalized,revision:42)
        let received = pending.receiving(fresh)
        XCTAssertTrue(received.profilesNeedingReview.contains(profile.id))
        XCTAssertEqual(received.cache.journal.pending.first?.recoveryDocument,original.documentData)
        XCTAssertThrowsError(try received.retryingConflict(receipt(edit,status:"conflict",current:42),fromAuthenticatedServerID:"mac-a"))
        XCTAssertEqual(received.cache.journal.pending.first?.edit.originalData,edit.originalData)
    }

    func testReceivedFrameSeparatesPairingMetadataFromCanonicalCache() throws {
        let packet = try wire()
        XCTAssertThrowsError(try ProfileFormatPolicy.validateIncoming(packet))
        try ProfileFormatPolicy.validateIncoming(packet,offered:[4,5])
        let frame = try OwnedPhoneReceivedFrame(wireData:packet,expectedServerID:"mac-a")
        XCTAssertEqual(frame.message.authToken,"must-not-enter-cache")
        XCTAssertEqual(frame.message.configurationRevision,UInt64.max)
        XCTAssertEqual(frame.message.gamepadProfiles?.count,2)
        XCTAssertEqual(frame.message.gamepadProfileID,frame.snapshot.activeProfileID)
        let cache = try OwnedPhoneCache(snapshot:frame.snapshot,journal:.init()).encoded()
        let restored = try OwnedPhoneCache(validating:cache)
        XCTAssertFalse(String(decoding:restored.snapshot!.documentData,as:UTF8.self).contains("must-not-enter-cache"))
        XCTAssertThrowsError(try OwnedPhoneReceivedFrame(wireData:packet,expectedServerID:"mac-b"))
    }
    func testOfflineSelectionAndDefaultStayLocalAndResetToServerOnRefresh() throws {
        let original = try snapshot(); let other = original.profiles[1].id
        let session = OwnedPhoneSession(cache:.init(snapshot:original,journal:.init()))
        let selected = try session.selecting(other).selecting(other,makeDefault:true)
        XCTAssertEqual(selected.cache.localSelection,other)
        XCTAssertEqual(selected.cache.localDefault,other)
        XCTAssertEqual(selected.cache.snapshot?.documentData,original.documentData)
        let restored = try OwnedPhoneCache(validating:selected.cache.encoded())
        XCTAssertEqual(restored.localSelection,other);XCTAssertEqual(restored.localDefault,other)
        let refresh = selected.receiving(original)
        XCTAssertNil(refresh.cache.localSelection);XCTAssertNil(refresh.cache.localDefault)
        XCTAssertThrowsError(try session.selecting(UUID()))
    }
    func testCacheUnknownFieldsAndPartialSnapshotAreNotSilentlyRewritten() throws {
        let cache = OwnedPhoneCache(snapshot:try snapshot(),journal:.init())
        var raw = try XCTUnwrap(JSONSerialization.jsonObject(with:cache.encoded()) as? [String:Any])
        raw["unknownCacheField"] = true
        XCTAssertThrowsError(try OwnedPhoneCache(validating:JSONSerialization.data(withJSONObject:raw)))
        raw.removeValue(forKey:"unknownCacheField"); raw.removeValue(forKey:"documentData")
        XCTAssertThrowsError(try OwnedPhoneCache(validating:JSONSerialization.data(withJSONObject:raw)))
    }

    func testRemovedPresetUsesNativeHideWhileCustomRemovalDeletes() throws {
        let snapshot = try snapshot(); let profile = snapshot.profiles[0]
        let builtin = try XCTUnwrap(profile.customization.elements.first)
        func removal(_ id: UUID, profileID: UUID) throws -> OwnedPhoneEdit {
            try OwnedPhoneEdit(validating:JSONSerialization.data(withJSONObject:["schemaVersion":1,"editID":UUID().uuidString,
                "profileID":profileID.uuidString,"baseConfigurationRevision":41,"removedControlIDs":[id.uuidString]]))
        }
        let hidden = try OwnedPhoneLayoutAdapter.applying(removal(builtin.id,profileID:profile.id),to:profile.customization)
        XCTAssertTrue(hidden.elements.contains(where: { $0.id == builtin.id }))
        XCTAssertTrue(hidden.buttonCustomization(for:builtin.inputID).isHidden)
        let second = snapshot.profiles[1]
        let custom = try XCTUnwrap(second.customization.elements.first(where: { $0.defaultControlID == nil }))
        let deleted = try OwnedPhoneLayoutAdapter.applying(removal(custom.id,profileID:second.id),to:second.customization)
        XCTAssertFalse(deleted.elements.contains(where: { $0.id == custom.id }))
    }

}
