import Foundation

/// Scalar intent only: the receiver owns canonical paint, resources and outputs.
/// Immutable storage keeps reliable control messages small on input threads.
public struct OwnedPhoneEdit: Codable, Equatable, Sendable {
    public static let maximumBytes = 64 * 1024
    private let storage: Storage
    private final class Storage: Sendable {
        let packet: Packet
        let data: Data
        init(_ packet: Packet, data: Data) { self.packet = packet; self.data = data }
    }
    private struct Packet: Codable, Equatable, Sendable {
        let schemaVersion: Int
        let editID: UUID
        let profileID: UUID
        let baseConfigurationRevision: UInt64
        let controlEdits: [OwnedPhoneControlEdit]
        let removedControlIDs: [UUID]
        let colorScheme: OwnedPhoneColorScheme?
        enum CodingKeys: String, CodingKey, CaseIterable {
            case schemaVersion, editID, profileID, baseConfigurationRevision, controlEdits, removedControlIDs, colorScheme
        }
        init(from decoder: Decoder) throws {
            try OwnedPhoneCoding.checkKeys(decoder, allowed: CodingKeys.allCases.map(\.rawValue))
            let c = try decoder.container(keyedBy: CodingKeys.self)
            schemaVersion = try c.decode(Int.self, forKey: .schemaVersion)
            editID = try OwnedPhoneCoding.uuid(c.decode(String.self, forKey: .editID))
            profileID = try OwnedPhoneCoding.uuid(c.decode(String.self, forKey: .profileID))
            baseConfigurationRevision = try c.decode(UInt64.self, forKey: .baseConfigurationRevision)
            // Rust defaults absent collections but refuses explicit null collections.
            controlEdits = c.contains(.controlEdits) ? try c.decode([OwnedPhoneControlEdit].self, forKey: .controlEdits) : []
            removedControlIDs = c.contains(.removedControlIDs) ? try c.decode([String].self, forKey: .removedControlIDs).map(OwnedPhoneCoding.uuid) : []
            colorScheme = try c.decodeIfPresent(OwnedPhoneColorScheme.self, forKey: .colorScheme)
            guard schemaVersion == 1, baseConfigurationRevision > 0,
                  controlEdits.count + removedControlIDs.count <= 128,
                  !controlEdits.isEmpty || !removedControlIDs.isEmpty || colorScheme != nil
            else { throw OwnedPhoneCoding.Failure.invalidIntent }
            var owners = Set<UUID>()
            for edit in controlEdits { guard owners.insert(edit.controlID).inserted else { throw OwnedPhoneCoding.Failure.invalidIntent } }
            for id in removedControlIDs { guard owners.insert(id).inserted else { throw OwnedPhoneCoding.Failure.invalidIntent } }
        }
    }
    public var editID: UUID { storage.packet.editID }
    public var profileID: UUID { storage.packet.profileID }
    public var baseConfigurationRevision: UInt64 { storage.packet.baseConfigurationRevision }
    public var controlEdits: [OwnedPhoneControlEdit] { storage.packet.controlEdits }
    public var removedControlIDs: [UUID] { storage.packet.removedControlIDs }
    public var colorScheme: OwnedPhoneColorScheme? { storage.packet.colorScheme }
    /// Original request bytes remain stable in the offline journal and on retries.
    public var originalData: Data { storage.data }
    public init(validating data: Data) throws {
        guard data.count <= Self.maximumBytes else { throw OwnedPhoneCoding.Failure.tooLarge }
        try OwnedPhoneCoding.validateTokens(data)
        storage = try Storage(JSONDecoder().decode(Packet.self, from: data), data: data)
    }
    public init(from decoder: Decoder) throws {
        let packet = try Packet(from: decoder)
        let data = try JSONEncoder().encode(packet)
        guard data.count <= Self.maximumBytes else { throw OwnedPhoneCoding.Failure.tooLarge }
        storage = Storage(packet, data: data)
    }
    public func encode(to encoder: Encoder) throws { try storage.packet.encode(to: encoder) }
    public static func == (lhs: Self, rhs: Self) -> Bool { lhs.storage.packet == rhs.storage.packet }
}

public enum OwnedPhoneColorScheme: String, Codable, Sendable { case system, light, dark }

public struct OwnedPhoneControlEdit: Codable, Equatable, Sendable {
    public let controlID: UUID
    public let centerX: Double?
    public let centerY: Double?
    public let widthScale: Double?
    public let heightScale: Double?
    public let rotationDegrees: Double?
    public let isHidden: Bool?
    public let isLocationLocked: Bool?
    public let zIndex: Int?
    enum CodingKeys: String, CodingKey, CaseIterable {
        case controlID, centerX, centerY, widthScale, heightScale, rotationDegrees, isHidden, isLocationLocked, zIndex
    }
    public init(from decoder: Decoder) throws {
        try OwnedPhoneCoding.checkKeys(decoder, allowed: CodingKeys.allCases.map(\.rawValue))
        let c = try decoder.container(keyedBy: CodingKeys.self)
        controlID = try OwnedPhoneCoding.uuid(c.decode(String.self, forKey: .controlID))
        centerX = try c.decodeIfPresent(Double.self, forKey: .centerX)
        centerY = try c.decodeIfPresent(Double.self, forKey: .centerY)
        widthScale = try c.decodeIfPresent(Double.self, forKey: .widthScale)
        heightScale = try c.decodeIfPresent(Double.self, forKey: .heightScale)
        rotationDegrees = try c.decodeIfPresent(Double.self, forKey: .rotationDegrees)
        isHidden = try c.decodeIfPresent(Bool.self, forKey: .isHidden)
        isLocationLocked = try c.decodeIfPresent(Bool.self, forKey: .isLocationLocked)
        zIndex = try c.decodeIfPresent(Int.self, forKey: .zIndex)
        guard centerX != nil || centerY != nil || widthScale != nil || heightScale != nil || rotationDegrees != nil || isHidden != nil || isLocationLocked != nil || zIndex != nil else { throw OwnedPhoneCoding.Failure.invalidIntent }
        for (value, range) in [(centerX, 0.0...1.0), (centerY, 0.0...1.0), (widthScale, 0.001...12.0), (heightScale, 0.001...12.0), (rotationDegrees, -36000.0...36000.0)] {
            guard value.map({ $0.isFinite && range.contains($0) }) ?? true else { throw OwnedPhoneCoding.Failure.invalidIntent }
        }
        guard zIndex.map({ (-100...100).contains($0) }) ?? true else { throw OwnedPhoneCoding.Failure.invalidIntent }
    }
}

public enum OwnedPhoneEditStatus: String, Codable, Sendable { case applied, replayed, conflict, rejected }

public struct OwnedPhoneEditResult: Codable, Equatable, Sendable {
    public let schemaVersion: Int
    public let editID: UUID
    public let profileID: UUID
    public let baseConfigurationRevision: UInt64
    public let configurationRevision: UInt64
    public let committedRevision: UInt64?
    public let changed: Bool
    public let status: OwnedPhoneEditStatus
    public let errorCode: String?
    enum CodingKeys: String, CodingKey, CaseIterable {
        case schemaVersion, editID, profileID, baseConfigurationRevision, configurationRevision, committedRevision, changed, status, errorCode
    }
    public init(from decoder: Decoder) throws {
        try OwnedPhoneCoding.checkKeys(decoder, allowed: CodingKeys.allCases.map(\.rawValue))
        let c = try decoder.container(keyedBy: CodingKeys.self)
        schemaVersion = try c.decode(Int.self, forKey: .schemaVersion)
        editID = try OwnedPhoneCoding.uuid(c.decode(String.self, forKey: .editID))
        profileID = try OwnedPhoneCoding.uuid(c.decode(String.self, forKey: .profileID))
        baseConfigurationRevision = try c.decode(UInt64.self, forKey: .baseConfigurationRevision)
        configurationRevision = try c.decode(UInt64.self, forKey: .configurationRevision)
        committedRevision = try c.decodeIfPresent(UInt64.self, forKey: .committedRevision)
        changed = try c.decode(Bool.self, forKey: .changed)
        status = try c.decode(OwnedPhoneEditStatus.self, forKey: .status)
        errorCode = try c.decodeIfPresent(String.self, forKey: .errorCode)
        guard schemaVersion == 1, baseConfigurationRevision > 0, configurationRevision > 0 else { throw OwnedPhoneCoding.Failure.invalidReceipt }
        if status == .applied || status == .replayed {
            guard let committedRevision, committedRevision >= baseConfigurationRevision,
                  configurationRevision >= committedRevision, changed == (committedRevision > baseConfigurationRevision), errorCode == nil
            else { throw OwnedPhoneCoding.Failure.invalidReceipt }
        } else {
            guard committedRevision == nil, !changed, let errorCode, !errorCode.isEmpty, errorCode.utf8.count <= 128 else { throw OwnedPhoneCoding.Failure.invalidReceipt }
        }
    }
}

enum OwnedPhoneCoding {
    enum Failure: Error { case invalidIntent, invalidReceipt, tooLarge, invalidJournal }
    struct Key: CodingKey {
        let stringValue: String
        var intValue: Int? { nil }
        init?(stringValue: String) { self.stringValue = stringValue }
        init?(intValue: Int) { return nil }
    }
    static func checkKeys(_ decoder: Decoder, allowed: [String]) throws {
        let keys = try decoder.container(keyedBy: Key.self).allKeys.map(\.stringValue)
        guard Set(keys).isSubset(of: Set(allowed)) else { throw Failure.invalidIntent }
    }
    static func uuid(_ text: String) throws -> UUID {
        guard let id = UUID(uuidString: text), id.uuidString.lowercased() == text.lowercased(), id != UUID(uuid: (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)) else { throw Failure.invalidIntent }
        return id
    }
    static let integerKeys: Set<String> = ["schemaVersion", "baseConfigurationRevision", "configurationRevision", "committedRevision", "zIndex"]
    static func validateTokens(_ data: Data) throws {
        try JSONDecoder.validateUniqueKeys(in: data, integerKeys: integerKeys)
    }
}
