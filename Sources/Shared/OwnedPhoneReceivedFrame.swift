import Foundation

/// Separates reliable connection metadata from canonical owned source before the
/// legacy Codable display model sees a profile. Never persist `message` as v4.
public struct OwnedPhoneReceivedFrame {
    private let storage: Storage
    private final class Storage {
        let snapshot: OwnedPhoneSnapshot
        let message: ControllerMessage
        init(snapshot: OwnedPhoneSnapshot, message: ControllerMessage) { self.snapshot = snapshot; self.message = message }
    }
    public var snapshot: OwnedPhoneSnapshot { storage.snapshot }
    public var message: ControllerMessage { storage.message }
    public init(wireData: Data, expectedServerID: String?) throws {
        guard wireData.count <= OwnedPhoneSnapshot.maximumBytes else { throw OwnedPhoneSnapshot.Failure.tooLarge }
        let fields = try JSONDecoder.rawObjectFields(in: wireData, named: ["type", "timestamp", "sentAt", "message", "realtimeToken",
            "authToken", "serverID", "capabilities", "supportedProfileFormats", "profileFormatVersion", "configurationRevision",
            "virtualGamepadStatus", "inputProtocolVersion", "inputGeneration", "gamepadProfileID", "defaultGamepadProfileID"])
        let announced = try fields["serverID"].map { try JSONDecoder().decode(String.self, from: $0) }
        guard let server = expectedServerID ?? announced else { throw OwnedPhoneSnapshot.Failure.authorityMismatch }
        let snapshot = try OwnedPhoneSnapshot(wireData: wireData, fromAuthenticatedServerID: server)
        var metadata = Data([123])
        for (index, key) in fields.keys.sorted().enumerated() {
            if index > 0 { metadata.append(44) }
            metadata.append(try JSONEncoder().encode(key)); metadata.append(58); metadata.append(fields[key]!)
        }
        metadata.append(125)
        var message = try ControllerWireCodec.decode(metadata, using: JSONDecoder())
        message.gamepadProfiles = snapshot.profiles
        message.gamepadCustomization = snapshot.profiles.first(where: { $0.id == snapshot.activeProfileID })?.customization
        message.bindingPresentations = snapshot.bindingPresentations
        storage = Storage(snapshot: snapshot, message: message)
    }
}
