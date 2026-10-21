import Foundation
import XCTest

final class OwnedPhoneEditTests: XCTestCase {
    private var root: URL { URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent() }
    private func data(_ path: String) throws -> Data { try Data(contentsOf: root.appendingPathComponent("Host/fixtures/" + path)) }
    private func intent() throws -> OwnedPhoneEdit { try OwnedPhoneEdit(validating: data("owned-phone-edits/v1/intent.json")) }
    private func frames() throws -> [ControllerMessage] {
        try JSONDecoder().decodeUnique([ControllerMessage].self, from: data("owned-phone-wire/v1/frames.json"))
    }
    private func journal(_ server: String = "mac-a") throws -> OwnedPhoneEditJournal {
        try OwnedPhoneEditJournal().recording(serverID: server, edit: intent(), updatedAt: 100, recoveryDocument: data("owned-element-sequences/v1/after-reset-and-hide.json"))
    }
    func testSharedIntentMatrixMatchesRustWithoutActivatingPhoneOffers() throws {
        let matrix = try XCTUnwrap(JSONSerialization.jsonObject(with: data("owned-phone-edits/v1/cases.json")) as? [String: Any])
        for row in try XCTUnwrap(matrix["cases"] as? [[String: Any]]) {
            let bytes: Data
            if let text = row["raw"] as? String { bytes = Data(text.utf8) }
            else { bytes = try JSONSerialization.data(withJSONObject: XCTUnwrap(row["request"])) }
            let allowed = (try? OwnedPhoneEdit(validating: bytes)) != nil
            XCTAssertEqual(allowed, row["allowed"] as? Bool, "\(row["name"] ?? "case")")
        }
        XCTAssertEqual(ProfileFormatPolicy.supportedFormats, [4])
    }
    func testUnsignedIntegerTokenKindsAndUnknownFieldsFailClosed() throws {
        let text = try XCTUnwrap(String(data: intent().originalData, encoding: .utf8))
        let regex = try NSRegularExpression(pattern: "\"baseConfigurationRevision\"\\s*:\\s*41")
        for token in ["41.0", "41e0", "-1", "18446744073709551616", "true", "\"41\""] {
            let changed = regex.stringByReplacingMatches(in: text, range: NSRange(text.startIndex..., in: text), withTemplate: "\"baseConfigurationRevision\":\(token)")
            XCTAssertThrowsError(try OwnedPhoneEdit(validating: Data(changed.utf8)), token)
        }
        let max = regex.stringByReplacingMatches(in: text, range: NSRange(text.startIndex..., in: text), withTemplate: "\"baseConfigurationRevision\":18446744073709551615")
        XCTAssertEqual(try OwnedPhoneEdit(validating: Data(max.utf8)).baseConfigurationRevision, UInt64.max)
        let object = try XCTUnwrap(JSONSerialization.jsonObject(with: intent().originalData) as? [String: Any])
        for key in ["skinReference", "gamepadCustomization", "appearanceBank", "orientation", "output"] {
            var modified = object; modified[key] = [:] as [String: String]
            XCTAssertThrowsError(try OwnedPhoneEdit(validating: JSONSerialization.data(withJSONObject: modified)))
        }
        XCTAssertThrowsError(try OwnedPhoneEdit(validating: Data(repeating: 32, count: OwnedPhoneEdit.maximumBytes + 1)))
    }
    func testSnapshotRevisionValidationDoesNotConstrainOpaqueNestedMetadata() throws {
        let text = #"{"type":"pong","timestamp":0,"configurationRevision":43,"future":{"configurationRevision":0.5,"schemaVersion":0.25,"zIndex":0.5}}"#
        XCTAssertEqual(try ControllerWireCodec.decode(Data(text.utf8), using: JSONDecoder()).configurationRevision, 43)
        XCTAssertThrowsError(try ControllerWireCodec.decode(Data(text.replacingOccurrences(of: "\"configurationRevision\":43", with: "\"configurationRevision\":43.0").utf8), using: JSONDecoder()))
    }
    func testFrozenWireReceiptsRetainHistoricalAndCurrentRevisions() throws {
        let values = try frames()
        XCTAssertEqual(values.count, 5)
        for value in values {
            let bytes = try ControllerWireCodec.encode(value, using: JSONEncoder())
            XCTAssertFalse(bytes.starts(with: [80,80]))
            let decoded = try ControllerWireCodec.decode(bytes, using: JSONDecoder())
            XCTAssertEqual(decoded.ownedPhoneEdit, value.ownedPhoneEdit)
            XCTAssertEqual(decoded.ownedPhoneEditResult, value.ownedPhoneEditResult)
            XCTAssertEqual(decoded.configurationRevision, value.configurationRevision)
        }
        XCTAssertEqual(values[2].ownedPhoneEditResult?.committedRevision, 42)
        XCTAssertEqual(values[2].configurationRevision, 43)
        var compact = ControllerMessage(type: .button, button: .preset(1), state: .down, timestamp: 0)
        XCTAssertTrue(try ControllerWireCodec.encode(compact, using: JSONEncoder()).starts(with: [80,80]))
        compact.ownedPhoneEdit = try intent()
        XCTAssertFalse(try ControllerWireCodec.encode(compact, using: JSONEncoder()).starts(with: [80,80]))
        XCTAssertThrowsError(try ControllerWireCodec.decode(ControllerWireCodec.encode(compact, using: JSONEncoder()), using: JSONDecoder()))
        compact.ownedPhoneEdit = nil; compact.configurationRevision = 41
        XCTAssertFalse(try ControllerWireCodec.encode(compact, using: JSONEncoder()).starts(with: [80,80]))
    }
    func testJournalPersistsOriginalBytesAndDeduplicatesRecoveryAcrossAuthorities() throws {
        let document = try data("owned-element-sequences/v1/after-reset-and-hide.json")
        let edit = try intent()
        var value = try journal()
        value = try value.recording(serverID: "mac-b", edit: edit, updatedAt: 101, recoveryDocument: document)
        let encoded = try value.encoded()
        let raw = try XCTUnwrap(JSONSerialization.jsonObject(with: encoded) as? [String: Any])
        XCTAssertEqual((raw["recoveryDocuments"] as? [String: Any])?.count, 1)
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let file = directory.appendingPathComponent("journal.json")
        try value.save(to: file)
        let restored = try OwnedPhoneEditJournal.load(from: file)
        XCTAssertEqual(restored.pending.count, 2)
        XCTAssertEqual(restored.pending[0].edit.originalData, edit.originalData)
        XCTAssertEqual(restored.pending[0].recoveryDocument, document)
        XCTAssertEqual(restored.uploads(forAuthenticatedServerID: "mac-a", negotiatedProfileFormat: 5).count, 1)
        XCTAssertTrue(restored.uploads(forAuthenticatedServerID: "mac-a", negotiatedProfileFormat: 4).isEmpty)
        XCTAssertEqual(try restored.recording(serverID: "mac-a", edit: edit, updatedAt: 200, recoveryDocument: Data()).pending.count, 2)
        try Data("corrupt".utf8).write(to: file)
        XCTAssertThrowsError(try OwnedPhoneEditJournal.load(from: file))
        XCTAssertThrowsError(try OwnedPhoneEditJournal().save(to: file))
        XCTAssertEqual(try Data(contentsOf: file), Data("corrupt".utf8))
        XCTAssertEqual(restored.pending.count, 2)
    }
    func testOnlyMatchingAuthenticatedReceiptRetiresAnEditAndNeverDowngradesSnapshot() throws {
        let value = try journal(); let values = try frames()
        let snapshot = ControllerMessage(type: .gamepadProfiles, timestamp: 0, configurationRevision: 43)
        XCTAssertEqual(try value.acknowledging(snapshot, fromAuthenticatedServerID: "mac-a", negotiatedProfileFormat: 5).pending.count, 1)
        XCTAssertEqual(try value.acknowledging(values[1], fromAuthenticatedServerID: "mac-b", negotiatedProfileFormat: 5).pending.count, 1)
        for failure in values.suffix(2) {
            XCTAssertEqual(try value.acknowledging(failure, fromAuthenticatedServerID: "mac-a", negotiatedProfileFormat: 5).pending.count, 1)
        }
        XCTAssertThrowsError(try value.acknowledging(values[1], fromAuthenticatedServerID: "mac-a", negotiatedProfileFormat: 4))
        var mixed = values[1]; mixed.configurationRevision = 99
        XCTAssertThrowsError(try value.acknowledging(mixed, fromAuthenticatedServerID: "mac-a", negotiatedProfileFormat: 5))
        let restored = try OwnedPhoneEditJournal(validating: value.encoded())
        XCTAssertTrue(try restored.acknowledging(values[2], fromAuthenticatedServerID: "mac-a", negotiatedProfileFormat: 5).pending.isEmpty)
        // The reducer returns only journal changes, never a replacement authority snapshot.
        XCTAssertEqual(values[2].configurationRevision, 43)
    }
    func testReceiptCannotAcknowledgeAnotherProfileOrBaseRevision() throws {
        let value = try journal()
        let array = try XCTUnwrap(JSONSerialization.jsonObject(with: data("owned-phone-wire/v1/frames.json")) as? [[String: Any]])
        for (key, replacement) in [("profileID", "ea000000-0000-0000-0000-000000000099" as Any), ("baseConfigurationRevision", 40 as Any), ("editID", "aa000000-0000-0000-0000-000000000099" as Any)] {
            var packet = array[1]; var result = try XCTUnwrap(packet["ownedPhoneEditResult"] as? [String: Any])
            result[key] = replacement; packet["ownedPhoneEditResult"] = result
            let frame = try ControllerWireCodec.decode(JSONSerialization.data(withJSONObject: packet), using: JSONDecoder())
            XCTAssertEqual(try value.acknowledging(frame, fromAuthenticatedServerID: "mac-a", negotiatedProfileFormat: 5).pending.count, 1)
        }
    }
    func testJournalRejectsDuplicateRecordsUnknownFieldsAndUnreferencedRecovery() throws {
        var envelope = try XCTUnwrap(JSONSerialization.jsonObject(with: journal().encoded()) as? [String: Any])
        let records = try XCTUnwrap(envelope["pending"] as? [[String: Any]])
        envelope["pending"] = records + records
        XCTAssertThrowsError(try OwnedPhoneEditJournal(validating: JSONSerialization.data(withJSONObject: envelope)))
        envelope["pending"] = records; envelope["future"] = true
        XCTAssertThrowsError(try OwnedPhoneEditJournal(validating: JSONSerialization.data(withJSONObject: envelope)))
        envelope.removeValue(forKey: "future"); envelope["pending"] = [] as [[String: Any]]
        XCTAssertThrowsError(try OwnedPhoneEditJournal(validating: JSONSerialization.data(withJSONObject: envelope)))
    }
    func testChangedIntentIdentityCorruptRecoveryAndReceiptInvariantsAreRejected() throws {
        let value = try journal()
        var object = try XCTUnwrap(JSONSerialization.jsonObject(with: intent().originalData) as? [String: Any])
        object["colorScheme"] = "light"
        let altered = try OwnedPhoneEdit(validating: JSONSerialization.data(withJSONObject: object))
        XCTAssertThrowsError(try value.recording(serverID: "mac-a", edit: altered, updatedAt: 200, recoveryDocument: data("owned-element-sequences/v1/after-reset-and-hide.json")))
        var envelope = try XCTUnwrap(JSONSerialization.jsonObject(with: value.encoded()) as? [String: Any])
        envelope["recoveryDocuments"] = [:] as [String: String]
        XCTAssertThrowsError(try OwnedPhoneEditJournal(validating: JSONSerialization.data(withJSONObject: envelope)))
        var packet = try XCTUnwrap(JSONSerialization.jsonObject(with: data("owned-phone-wire/v1/frames.json")) as? [[String: Any]])
        var receipt = try XCTUnwrap(packet[1]["ownedPhoneEditResult"] as? [String: Any]); receipt["committedRevision"] = 40
        packet[1]["ownedPhoneEditResult"] = receipt
        XCTAssertThrowsError(try ControllerWireCodec.decode(JSONSerialization.data(withJSONObject: packet[1]), using: JSONDecoder()))
    }
}
