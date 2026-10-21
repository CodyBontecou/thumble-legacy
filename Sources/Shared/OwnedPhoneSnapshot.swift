import Foundation

/// A received canonical snapshot and its native display projection. Only original
/// document bytes enter the cache/recovery journal; projected profiles are never
/// serialized back into the owned document.
public struct OwnedPhoneSnapshot {
    public static let maximumBytes = ControllerWireCodec.maximumInboundPayloadSize
    private let storage: Storage
    private final class Storage {
        let serverID: String
        let revision: UInt64
        let document: ThumbleOwnedConfigurationDocument
        let bindings: [GamepadProfileBindingPresentations]
        init(serverID: String, revision: UInt64, document: ThumbleOwnedConfigurationDocument,
             bindings: [GamepadProfileBindingPresentations]) {
            self.serverID = serverID; self.revision = revision; self.document = document; self.bindings = bindings
        }
    }
    public var serverID: String { storage.serverID }
    public var configurationRevision: UInt64 { storage.revision }
    public var documentData: Data { storage.document.originalData }
    public var profiles: [GamepadConfigurationProfile] { storage.document.projectedProfiles }
    public var activeProfileID: UUID { storage.document.activeProfileID }
    public var defaultProfileID: UUID? { storage.document.defaultProfileID }
    public var bindingPresentations: [GamepadProfileBindingPresentations] { storage.bindings }

    public enum Failure: Error { case invalidSnapshot, authorityMismatch, tooLarge }
    private struct Header: Decodable {
        let type: ControllerMessageType
        let serverID: String?
        let profileFormatVersion: Int
        let configurationRevision: UInt64
    }
    public init(wireData: Data, fromAuthenticatedServerID serverID: String) throws {
        guard wireData.count <= Self.maximumBytes else { throw Failure.tooLarge }
        try JSONDecoder.validateUniqueKeys(in: wireData, integerKeys: ["profileFormatVersion", "configurationRevision"], integerKeyDepthLimit: 1)
        let header = try JSONDecoder().decode(Header.self, from: wireData)
        guard [.hello, .pairingAccepted, .gamepadProfiles, .gamepadCustomization].contains(header.type),
              header.profileFormatVersion == 5, header.configurationRevision > 0 else { throw Failure.invalidSnapshot }
        guard Self.validServer(serverID), header.serverID == nil || header.serverID == serverID else { throw Failure.authorityMismatch }
        let fields = try JSONDecoder.rawObjectFields(in: wireData, named: ["gamepadProfiles", "gamepadProfileID", "defaultGamepadProfileID", "bindingPresentations"])
        guard let profiles = fields["gamepadProfiles"], let active = fields["gamepadProfileID"] else { throw Failure.invalidSnapshot }
        var document = Data(#"{"schema":"com.codybontecou.pocketpad.keypad-configuration","version":5,"profiles":"#.utf8)
        document.append(profiles)
        document.append(Data(#", "activeProfileID":"#.utf8)); document.append(active)
        document.append(Data(#", "defaultProfileID":"#.utf8)); document.append(fields["defaultGamepadProfileID"] ?? Data("null".utf8))
        document.append(125)
        let decoded = try ThumbleOwnedConfigurationDocument(validating: document)
        let bindings = try fields["bindingPresentations"].map { try JSONDecoder().decodeUnique([GamepadProfileBindingPresentations].self, from: $0) } ?? []
        storage = Storage(serverID: serverID, revision: header.configurationRevision, document: decoded, bindings: bindings)
    }
    public init(serverID: String, configurationRevision: UInt64, documentData: Data,
                bindingPresentations: [GamepadProfileBindingPresentations] = []) throws {
        guard Self.validServer(serverID), configurationRevision > 0 else { throw Failure.invalidSnapshot }
        guard documentData.count <= Self.maximumBytes else { throw Failure.tooLarge }
        storage = try Storage(serverID: serverID, revision: configurationRevision,
                              document: ThumbleOwnedConfigurationDocument(validating: documentData), bindings: bindingPresentations)
    }
    private static func validServer(_ value: String) -> Bool {
        !value.isEmpty && value == value.trimmingCharacters(in: .whitespacesAndNewlines) && value.utf8.count <= 512
    }
}

/// Atomic local cache transaction: authority snapshot and pending edits change
/// together. Authentication tokens are deliberately absent from this envelope.
public struct OwnedPhoneCache {
    // The local transaction contains two bounded wire-sized records encoded as
    // base64; this is a disk bound, independent of the 8 MiB network frame bound.
    public static let maximumBytes = 24 * 1024 * 1024
    public let snapshot: OwnedPhoneSnapshot?
    public let journal: OwnedPhoneEditJournal
    public let localSelection: UUID?
    public let localDefault: UUID?
    private struct Envelope: Codable {
        let version: Int
        let serverID: String?
        let configurationRevision: UInt64?
        let documentData: Data?
        let bindingPresentations: [GamepadProfileBindingPresentations]
        let journalData: Data
        let localSelection: UUID?
        let localDefault: UUID?
        enum CodingKeys: String, CodingKey, CaseIterable { case version, serverID, configurationRevision, documentData, bindingPresentations, journalData, localSelection, localDefault }
        init(snapshot: OwnedPhoneSnapshot?, journalData: Data, localSelection: UUID?, localDefault: UUID?) {
            self.localSelection = localSelection; self.localDefault = localDefault
            version = 1; serverID = snapshot?.serverID; configurationRevision = snapshot?.configurationRevision
            documentData = snapshot?.documentData; bindingPresentations = snapshot?.bindingPresentations ?? []; self.journalData = journalData
        }
        init(from decoder: Decoder) throws {
            try OwnedPhoneCoding.checkKeys(decoder, allowed: CodingKeys.allCases.map(\.rawValue))
            let c = try decoder.container(keyedBy: CodingKeys.self)
            version = try c.decode(Int.self, forKey: .version)
            serverID = try c.decodeIfPresent(String.self, forKey: .serverID)
            configurationRevision = try c.decodeIfPresent(UInt64.self, forKey: .configurationRevision)
            documentData = try c.decodeIfPresent(Data.self, forKey: .documentData)
            bindingPresentations = try c.decode([GamepadProfileBindingPresentations].self, forKey: .bindingPresentations)
            journalData = try c.decode(Data.self, forKey: .journalData)
            localSelection = try c.decodeIfPresent(UUID.self, forKey: .localSelection)
            localDefault = try c.decodeIfPresent(UUID.self, forKey: .localDefault)
        }
    }
    public init(snapshot: OwnedPhoneSnapshot?, journal: OwnedPhoneEditJournal, localSelection: UUID? = nil, localDefault: UUID? = nil) {
        self.snapshot = snapshot; self.journal = journal; self.localSelection = localSelection; self.localDefault = localDefault
    }
    public func encoded() throws -> Data {
        try Self.validateLocalProfiles(snapshot: snapshot, selection: localSelection, defaultID: localDefault)
        let value = Envelope(snapshot: snapshot, journalData: try journal.encoded(), localSelection: localSelection, localDefault: localDefault)
        let data = try JSONEncoder().encode(value)
        guard data.count <= Self.maximumBytes else { throw OwnedPhoneSnapshot.Failure.tooLarge }
        return data
    }
    public init(validating data: Data) throws {
        guard data.count <= Self.maximumBytes else { throw OwnedPhoneSnapshot.Failure.tooLarge }
        try JSONDecoder.validateUniqueKeys(in: data, integerKeys: ["version", "configurationRevision"], integerKeyDepthLimit: 1)
        let envelope = try JSONDecoder().decode(Envelope.self, from: data)
        guard envelope.version == 1 else { throw OwnedPhoneSnapshot.Failure.invalidSnapshot }
        journal = try OwnedPhoneEditJournal(validating: envelope.journalData)
        if let server = envelope.serverID, let revision = envelope.configurationRevision, let document = envelope.documentData {
            snapshot = try OwnedPhoneSnapshot(serverID: server, configurationRevision: revision, documentData: document,
                                              bindingPresentations: envelope.bindingPresentations)
        } else {
            guard envelope.serverID == nil, envelope.configurationRevision == nil, envelope.documentData == nil,
                  envelope.bindingPresentations.isEmpty else { throw OwnedPhoneSnapshot.Failure.invalidSnapshot }
            snapshot = nil
        }
        localSelection = envelope.localSelection; localDefault = envelope.localDefault
        try Self.validateLocalProfiles(snapshot: snapshot, selection: localSelection, defaultID: localDefault)
    }
    private static func validateLocalProfiles(snapshot: OwnedPhoneSnapshot?, selection: UUID?, defaultID: UUID?) throws {
        let ids = Set(snapshot?.profiles.map(\.id) ?? [])
        guard selection.map(ids.contains) ?? true, defaultID.map(ids.contains) ?? true else { throw OwnedPhoneSnapshot.Failure.invalidSnapshot }
    }
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
