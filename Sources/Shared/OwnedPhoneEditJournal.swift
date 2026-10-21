import Foundation
import CryptoKit

/// Durable phone intent queue. Snapshots never acknowledge edits. Only an exact
/// receipt from the same authenticated authority can retire a pending record.
public struct OwnedPhoneEditJournal: Sendable {
    public static let maximumEdits = 512
    public static let maximumBytes = ControllerWireCodec.maximumInboundPayloadSize
    public struct Pending: Sendable {
        public let serverID: String
        public let edit: OwnedPhoneEdit
        public let updatedAt: Int64
        public let recoveryDocument: Data
    }
    public let pending: [Pending]
    public init() { pending = [] }
    private init(_ values: [Pending]) { pending = values }
    private struct Record: Codable {
        let serverID: String
        let intentJSON: String
        let updatedAt: Int64
        let recovery: String
        enum CodingKeys: String, CodingKey, CaseIterable { case serverID, intentJSON, updatedAt, recovery }
        init(_ value: Pending) throws {
            serverID = value.serverID; updatedAt = value.updatedAt
            guard let text = String(data: value.edit.originalData, encoding: .utf8) else { throw OwnedPhoneCoding.Failure.invalidJournal }
            intentJSON = text; recovery = OwnedPhoneEditJournal.digest(value.recoveryDocument)
        }
        init(from decoder: Decoder) throws {
            try OwnedPhoneCoding.checkKeys(decoder, allowed: CodingKeys.allCases.map(\.rawValue))
            let c = try decoder.container(keyedBy: CodingKeys.self)
            serverID = try c.decode(String.self, forKey: .serverID)
            intentJSON = try c.decode(String.self, forKey: .intentJSON)
            updatedAt = try c.decode(Int64.self, forKey: .updatedAt)
            recovery = try c.decode(String.self, forKey: .recovery)
        }
    }
    private struct Envelope: Codable {
        let version: Int
        let pending: [Record]
        let recoveryDocuments: [String: Data]
        enum CodingKeys: String, CodingKey, CaseIterable { case version, pending, recoveryDocuments }
        init(pending: [Record], recoveryDocuments: [String: Data]) {
            version = 1; self.pending = pending; self.recoveryDocuments = recoveryDocuments
        }
        init(from decoder: Decoder) throws {
            try OwnedPhoneCoding.checkKeys(decoder, allowed: CodingKeys.allCases.map(\.rawValue))
            let c = try decoder.container(keyedBy: CodingKeys.self)
            version = try c.decode(Int.self, forKey: .version)
            pending = try c.decode([Record].self, forKey: .pending)
            recoveryDocuments = try c.decode([String: Data].self, forKey: .recoveryDocuments)
        }
    }
    private static func digest(_ bytes: Data) -> String {
        SHA256.hash(data: bytes).map { String(format: "%02x", $0) }.joined()
    }
    private static func validServer(_ id: String) -> Bool {
        !id.isEmpty && id == id.trimmingCharacters(in: .whitespacesAndNewlines) && id.utf8.count <= 512
    }
    public func recording(serverID: String, edit: OwnedPhoneEdit, updatedAt: Int64, recoveryDocument: Data) throws -> Self {
        guard Self.validServer(serverID), updatedAt >= 0 else { throw OwnedPhoneCoding.Failure.invalidJournal }
        if let previous = pending.first(where: { $0.serverID == serverID && $0.edit.editID == edit.editID }) {
            guard previous.edit.originalData == edit.originalData else { throw OwnedPhoneCoding.Failure.invalidJournal }
            return self
        }
        guard pending.count < Self.maximumEdits else { throw OwnedPhoneCoding.Failure.tooLarge }
        let document = try ThumbleOwnedConfigurationDocument(validating: recoveryDocument)
        guard document.projectedProfiles.contains(where: { $0.id == edit.profileID }) else { throw OwnedPhoneCoding.Failure.invalidJournal }
        let result = Self(pending + [Pending(serverID: serverID, edit: edit, updatedAt: updatedAt, recoveryDocument: recoveryDocument)])
        _ = try result.encoded()
        return result
    }
    /// Stable first-per-profile order; never regenerate an in-flight edit ID or
    /// silently change its base revision to make a conflict appear successful.
    public func uploads(forAuthenticatedServerID serverID: String, negotiatedProfileFormat: Int) -> [Pending] {
        guard Self.validServer(serverID), negotiatedProfileFormat == 5 else { return [] }
        var owners = Set<UUID>()
        return pending.filter { $0.serverID == serverID && owners.insert($0.edit.profileID).inserted }
    }
    public func acknowledging(_ message: ControllerMessage, fromAuthenticatedServerID serverID: String, negotiatedProfileFormat: Int) throws -> Self {
        guard message.type == .ownedPhoneEditResult else { return self }
        guard Self.validServer(serverID), negotiatedProfileFormat == 5, message.profileFormatVersion == 5,
              let receipt = message.ownedPhoneEditResult, message.configurationRevision == receipt.configurationRevision,
              message.ownedPhoneEdit == nil, message.gamepadProfiles == nil, message.gamepadCustomization == nil,
              message.skinPackages == nil, message.skinReference == nil
        else { throw OwnedPhoneCoding.Failure.invalidReceipt }
        guard receipt.status == .applied || receipt.status == .replayed else { return self }
        return Self(pending.filter { value in
            !(value.serverID == serverID && value.edit.editID == receipt.editID && value.edit.profileID == receipt.profileID && value.edit.baseConfigurationRevision == receipt.baseConfigurationRevision)
        })
    }
    /// Replace an uncertain request only after an authenticated conflict proves
    /// it did not commit, and a newer full snapshot provides the retry base.
    public func retryingConflict(_ message: ControllerMessage, fromAuthenticatedServerID serverID: String,
                                using snapshot: OwnedPhoneSnapshot, editID: UUID = UUID()) throws -> Self {
        guard message.type == .ownedPhoneEditResult else { throw OwnedPhoneCoding.Failure.invalidReceipt }
        _ = try acknowledging(message, fromAuthenticatedServerID: serverID, negotiatedProfileFormat: 5)
        guard let receipt = message.ownedPhoneEditResult, receipt.status == .conflict,
              receipt.errorCode == "owned_phone_revision_conflict" || receipt.errorCode == "owned_phone_source_changed"
        else { throw OwnedPhoneCoding.Failure.invalidReceipt }
        guard let index = pending.firstIndex(where: { $0.serverID == serverID && $0.edit.editID == receipt.editID &&
            $0.edit.profileID == receipt.profileID && $0.edit.baseConfigurationRevision == receipt.baseConfigurationRevision }) else { return self }
        guard snapshot.serverID == serverID, snapshot.configurationRevision >= receipt.configurationRevision,
              snapshot.configurationRevision > receipt.baseConfigurationRevision, editID != receipt.editID,
              !pending.contains(where: { $0.serverID == serverID && $0.edit.editID == editID }) else { throw OwnedPhoneCoding.Failure.invalidReceipt }
        let original = pending[index]
        var fields = try JSONDecoder.rawObjectFields(in: original.edit.originalData,
            named: ["schemaVersion", "editID", "profileID", "baseConfigurationRevision", "controlEdits", "removedControlIDs", "colorScheme"])
        fields["editID"] = try JSONEncoder().encode(editID)
        fields["baseConfigurationRevision"] = try JSONEncoder().encode(snapshot.configurationRevision)
        var bytes = Data([123])
        for (i, key) in fields.keys.sorted().enumerated() {
            if i > 0 { bytes.append(44) }
            bytes.append(try JSONEncoder().encode(key)); bytes.append(58); bytes.append(fields[key]!)
        }
        bytes.append(125)
        let retry = try OwnedPhoneEdit(validating: bytes)
        guard var profile = snapshot.profiles.first(where: { $0.id == retry.profileID }) else { throw OwnedPhoneLayoutAdapter.Failure.missingControl }
        for value in pending where value.serverID == serverID && value.edit.profileID == retry.profileID {
            let operation = value.edit.editID == original.edit.editID ? retry : value.edit
            profile.customization = try OwnedPhoneLayoutAdapter.applying(operation, to: profile.customization)
        }
        var next = pending
        next[index] = Pending(serverID: serverID, edit: retry, updatedAt: original.updatedAt, recoveryDocument: snapshot.documentData)
        let result = Self(next); _ = try result.encoded(); return result
    }
    public func encoded() throws -> Data {
        guard pending.count <= Self.maximumEdits else { throw OwnedPhoneCoding.Failure.tooLarge }
        var recovery: [String: Data] = [:]
        let records = try pending.map { value in
            let record = try Record(value); recovery[record.recovery] = value.recoveryDocument; return record
        }
        let data = try JSONEncoder().encode(Envelope(pending: records, recoveryDocuments: recovery))
        guard data.count <= Self.maximumBytes else { throw OwnedPhoneCoding.Failure.tooLarge }
        return data
    }
    public init(validating data: Data) throws {
        guard data.count <= Self.maximumBytes else { throw OwnedPhoneCoding.Failure.tooLarge }
        try JSONDecoder.validateUniqueKeys(in: data, integerKeys: ["version", "updatedAt"])
        let envelope = try JSONDecoder().decode(Envelope.self, from: data)
        guard envelope.version == 1, envelope.pending.count <= Self.maximumEdits,
              envelope.recoveryDocuments.count <= Self.maximumEdits else { throw OwnedPhoneCoding.Failure.invalidJournal }
        // Decode each deduplicated recovery document once, retain its exact bytes.
        var profiles: [String: Set<UUID>] = [:]
        for (key, bytes) in envelope.recoveryDocuments {
            guard Self.digest(bytes) == key else { throw OwnedPhoneCoding.Failure.invalidJournal }
            profiles[key] = Set(try ThumbleOwnedConfigurationDocument(validating: bytes).projectedProfiles.map(\.id))
        }
        var seen = Set<String>()
        var used = Set<String>()
        var values: [Pending] = []
        for record in envelope.pending {
            let edit = try OwnedPhoneEdit(validating: Data(record.intentJSON.utf8))
            guard Self.validServer(record.serverID), record.updatedAt >= 0,
                  seen.insert(record.serverID + ":" + edit.editID.uuidString).inserted,
                  profiles[record.recovery]?.contains(edit.profileID) == true,
                  let recovery = envelope.recoveryDocuments[record.recovery]
            else { throw OwnedPhoneCoding.Failure.invalidJournal }
            used.insert(record.recovery)
            values.append(Pending(serverID: record.serverID, edit: edit, updatedAt: record.updatedAt, recoveryDocument: recovery))
        }
        guard used == Set(envelope.recoveryDocuments.keys) else { throw OwnedPhoneCoding.Failure.invalidJournal }
        pending = values
    }
    /// Callers install the returned journal in memory only after this write
    /// succeeds; corrupt existing files throw and are never reset to empty.
    public func save(to url: URL) throws {
        if FileManager.default.fileExists(atPath: url.path) { _ = try Self.load(from: url) }
        let bytes = try encoded()
        #if os(iOS)
        try bytes.write(to: url, options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication])
        #else
        try bytes.write(to: url, options: .atomic)
        #endif
    }
    public static func load(from url: URL) throws -> Self {
        try Self(validating: Data(contentsOf: url, options: .mappedIfSafe))
    }
}
