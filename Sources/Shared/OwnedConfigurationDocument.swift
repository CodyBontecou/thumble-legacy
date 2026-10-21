import Foundation

/// Explicit v5 interchange reader. The original bytes are authoritative; native
/// profiles are display/edit preparation only and must not be re-exported as v4.
public struct ThumbleOwnedConfigurationDocument {
    public static let version = 5
    public static let maximumBytes = 8 * 1024 * 1024
    private let storage: Storage
    public var originalData: Data { storage.data }
    public var projectedProfiles: [GamepadConfigurationProfile] { storage.profiles }
    public var activeProfileID: UUID { storage.active }
    public var defaultProfileID: UUID? { storage.defaultID }

    public enum ReadError: Error { case invalidEnvelope, invalidProfile, tooLarge }
    private final class Storage {
        let data: Data
        let profiles: [GamepadConfigurationProfile]
        let active: UUID
        let defaultID: UUID?
        init(data: Data, profiles: [GamepadConfigurationProfile], active: UUID, defaultID: UUID?) {
            self.data = data; self.profiles = profiles; self.active = active; self.defaultID = defaultID
        }
    }
    public init(validating data: Data) throws {
        storage = try Workspace(data).decode()
    }
    private final class Workspace {
        let data: Data
        var root: [String: Any] = [:]
        var profiles: [GamepadConfigurationProfile] = []
        init(_ data: Data) { self.data = data }
        func decode() throws -> Storage {
            guard data.count <= maximumBytes else { throw ReadError.tooLarge }
            guard !data.starts(with: [0xef, 0xbb, 0xbf]) else { throw ReadError.invalidEnvelope }
            try JSONDecoder.validateUniqueKeys(in: data)
            guard let root = try JSONSerialization.jsonObject(with: data) as? [String: Any] else { throw ReadError.invalidEnvelope }
            self.root = root
            try validateEnvelope()
            try decodeProfiles()
            guard let text = root["activeProfileID"] as? String, let active = PortableProfileArtifact.canonicalUUID(text),
                  root.keys.contains("defaultProfileID") else { throw ReadError.invalidEnvelope }
            let defaultID: UUID?
            if root["defaultProfileID"] is NSNull { defaultID = nil }
            else {
                guard let text = root["defaultProfileID"] as? String, let id = PortableProfileArtifact.canonicalUUID(text) else { throw ReadError.invalidEnvelope }
                defaultID = id
            }
            try ThumbleKeypadConfigurationExport.validateProfileReferences(profiles: profiles, activeProfileID: active, defaultProfileID: defaultID)
            try validateBindings(active: active)
            return Storage(data: data, profiles: profiles, active: active, defaultID: defaultID)
        }
        private func validateEnvelope() throws {
            guard root["schema"] as? String == ThumbleKeypadConfigurationExport.schemaIdentifier,
                  PortableProfileArtifact.exactInteger(root["version"]) == 5 else { throw ReadError.invalidEnvelope }
            let known = ["schema", "version", "exportedAt", "profiles", "activeProfileID", "defaultProfileID", "keyBindings", "outputBindings", "profileKeyBindings", "profileOutputBindings"]
            let forbidden = ["serverid", "trustedclients", "authtoken", "artifactversion", "contenthash", "catalogrevision"]
            for key in root.keys {
                guard !forbidden.contains(normalized(key)), !hasReservedAlias(key, known: known) else { throw ReadError.invalidEnvelope }
            }
            if let time = root["exportedAt"] {
                guard let time = PortableProfileArtifact.exactInteger(time), time >= 0 else { throw ReadError.invalidEnvelope }
            }
        }
        private func decodeProfiles() throws {
            guard let records = root["profiles"] as? [[String: Any]], (1...256).contains(records.count) else { throw ReadError.invalidEnvelope }
            var seen = Set<UUID>()
            for record in records {
                let profile = try decodeProfile(record)
                guard seen.insert(profile.id).inserted else { throw ReadError.invalidProfile }
                profiles.append(profile)
            }
        }
        private func decodeProfile(_ record: [String: Any]) throws -> GamepadConfigurationProfile {
            let retired = ["skinReference", "skinBaselineCustomization", "landscapeSkinBaselineCustomization", "portraitSkinBaselineCustomization", "landscapeCustomization", "portraitCustomization", "orientationPreference"].map(normalized)
            let known = ["id", "name", "updatedAt", "customization", "outputMode", "launchTarget", "appearanceBank", "appearanceResourceVersion", "appearanceResources", "profileFormatVersion"]
            guard PortableProfileArtifact.exactInteger(record["profileFormatVersion"]) == 5,
                  let idText = record["id"] as? String, PortableProfileArtifact.canonicalUUID(idText) != nil,
                  let name = record["name"] as? String, !name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
                  name.unicodeScalars.count <= 256,
                  let time = PortableProfileArtifact.exactInteger(record["updatedAt"]), time >= 0,
                  let layout = record["customization"] as? [String: Any],
                  let canvas = layout["deviceCanvas"] as? [String: Any], let frameID = canvas["frameID"] as? String,
                  let frame = GamepadEditorDeviceCatalog.frame(forStoredID: frameID), frame.isLandscape, frame.id == frameID
            else { throw ReadError.invalidProfile }
            for key in record.keys {
                guard !retired.contains(normalized(key)), !hasReservedAlias(key, known: known) else { throw ReadError.invalidProfile }
            }
            // This transient native projection is never the interchange authority.
            // Remove only its already-validated format tag; the stored source stays v5.
            var projection = record
            projection.removeValue(forKey: "profileFormatVersion")
            projection["orientationPreference"] = "landscape"
            let bytes = try JSONSerialization.data(withJSONObject: projection)
            return try JSONDecoder().decodeUnique(GamepadConfigurationProfile.self, from: bytes)
        }
        private func validateBindings(active: UUID) throws {
            let declared = Dictionary(uniqueKeysWithValues: profiles.map { ($0.id.uuidString.lowercased(), Set($0.customization.elements.map(\.id))) })
            for (field, output) in [("profileKeyBindings", false), ("profileOutputBindings", true)] {
                if let raw = root[field] {
                    guard let maps = raw as? [String: Any], maps.count <= 512 else { throw ReadError.invalidEnvelope }
                    try PortableProfileArtifact.validateBindingMaps(maps, declaredByProfile: declared, output: output)
                }
            }
            for (field, output) in [("keyBindings", false), ("outputBindings", true)] {
                if let raw = root[field] {
                    guard let map = raw as? [String: Any] else { throw ReadError.invalidEnvelope }
                    try PortableProfileArtifact.validateBindingMaps([active.uuidString: map], declaredByProfile: declared, output: output)
                }
            }
        }
        private func normalized(_ text: String) -> String {
            String(text.unicodeScalars.filter { (65...90).contains($0.value) || (97...122).contains($0.value) || (48...57).contains($0.value) }).lowercased()
        }
        private func hasReservedAlias(_ key: String, known: [String]) -> Bool {
            known.contains { $0 != key && normalized($0) == normalized(key) }
        }
    }
}

/// Explicit owned editing boundary. Typed views describe an operation's changes;
/// the caller merges them into the exact source and owns revision/persistence.
public struct ThumbleOwnedOperationBridgeRequest: Decodable, Sendable {
    public let schemaVersion: Int
    public let invocationID: UUID
    public let ownedConfigurationCommand: String
    public let documentData: String
    public let operation: ThumbleBridgeJSONValue
    public let nowMillis: Int64
    public init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: Key.self)
        guard values.allKeys.allSatisfy({ ["schemaVersion", "invocationID", "ownedConfigurationCommand", "documentData", "operation", "nowMillis"].contains($0.stringValue) }) else { throw ThumbleOwnedOperationBridgeError.invalidRequest }
        schemaVersion = try values.decode(Int.self, forKey: .init("schemaVersion"))
        invocationID = try values.decode(UUID.self, forKey: .init("invocationID"))
        ownedConfigurationCommand = try values.decode(String.self, forKey: .init("ownedConfigurationCommand"))
        documentData = try values.decode(String.self, forKey: .init("documentData"))
        operation = try values.decode(ThumbleBridgeJSONValue.self, forKey: .init("operation"))
        nowMillis = try values.decode(Int64.self, forKey: .init("nowMillis"))
    }
    private struct Key: CodingKey {
        let stringValue: String
        var intValue: Int? { nil }
        init(_ text: String) { stringValue = text }
        init?(stringValue: String) { self.stringValue = stringValue }
        init?(intValue: Int) { return nil }
    }
}
public struct ThumbleOwnedOperationBridgeResponse: Encodable, Sendable {
    public let schemaVersion = 1
    public let invocationID: UUID
    public let sourceSHA256: String
    public let beforeDocumentData: String
    public let afterDocumentData: String
    public let changed: Bool
    public let changedPaths: [String]
}
public enum ThumbleOwnedOperationBridgeError: String, Error {
    case invalidRequest = "owned_invalid_request"
    case invalidDocument = "owned_invalid_document"
    case retiredOperation = "owned_retired_operation"
    case invalidResult = "owned_invalid_result"
    case tooLarge = "owned_too_large"
}
public enum ThumbleOwnedOperationBridge {
    public static func transform(_ request: ThumbleOwnedOperationBridgeRequest) throws -> ThumbleOwnedOperationBridgeResponse {
        try Workspace(request).run()
    }
    private final class Workspace {
        let request: ThumbleOwnedOperationBridgeRequest
        var source = Data()
        var root: [String: Any] = [:]
        var beforeProfiles: [GamepadConfigurationProfile] = []
        var before = Data()
        var after = Data()
        var paths: [String] = []
        let encoder: JSONEncoder = {
            let value = JSONEncoder(); value.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]; return value
        }()
        init(_ request: ThumbleOwnedOperationBridgeRequest) { self.request = request }
        func run() throws -> ThumbleOwnedOperationBridgeResponse {
            try decodeSource()
            before = try projection(beforeProfiles)
            try applyOperation()
            guard before.count <= ThumbleOwnedConfigurationDocument.maximumBytes,
                  after.count <= ThumbleOwnedConfigurationDocument.maximumBytes else { throw ThumbleOwnedOperationBridgeError.tooLarge }
            _ = try ThumbleOwnedConfigurationDocument(validating: before)
            _ = try ThumbleOwnedConfigurationDocument(validating: after)
            let changed = before != after
            if changed && paths.isEmpty { paths = ["/defaultProfileID"] }
            return ThumbleOwnedOperationBridgeResponse(invocationID: request.invocationID,
                sourceSHA256: ProfileArtifactAdoptionMetadata.sha256(source),
                beforeDocumentData: before.base64EncodedString(), afterDocumentData: after.base64EncodedString(),
                changed: changed, changedPaths: changed ? paths : [])
        }
        private func decodeSource() throws {
            guard request.schemaVersion == 1, request.ownedConfigurationCommand == "apply", request.nowMillis >= 0,
                  request.documentData.utf8.count <= ((ThumbleOwnedConfigurationDocument.maximumBytes + 2) / 3) * 4,
                  let bytes = Data(base64Encoded: request.documentData), bytes.base64EncodedString() == request.documentData,
                  bytes.count <= ThumbleOwnedConfigurationDocument.maximumBytes else { throw ThumbleOwnedOperationBridgeError.invalidRequest }
            source = bytes
            let document = try ThumbleOwnedConfigurationDocument(validating: bytes)
            guard let object = try JSONSerialization.jsonObject(with: bytes) as? [String: Any] else { throw ThumbleOwnedOperationBridgeError.invalidDocument }
            let names = Set(["schema", "version", "exportedAt", "activeProfileID", "defaultProfileID", "keyBindings", "outputBindings", "profileKeyBindings", "profileOutputBindings"])
            root = object.filter { names.contains($0.key) }
            for key in ["keyBindings", "outputBindings", "profileKeyBindings", "profileOutputBindings"] where root[key] == nil { root[key] = [String: Any]() }
            beforeProfiles = document.projectedProfiles
        }
        private func profileObject(_ profile: GamepadConfigurationProfile) throws -> [String: Any] {
            // A legacy template may still refer to a package. Reject it rather
            // than discarding that reference and silently losing its appearance.
            guard profile.skinReference == nil else { throw ThumbleOwnedOperationBridgeError.retiredOperation }
            guard var object = try JSONSerialization.jsonObject(with: encoder.encode(profile)) as? [String: Any] else { throw ThumbleOwnedOperationBridgeError.invalidResult }
            for key in ["skinReference", "skinBaselineCustomization", "landscapeSkinBaselineCustomization", "portraitSkinBaselineCustomization", "landscapeCustomization", "portraitCustomization", "orientationPreference"] { object.removeValue(forKey: key) }
            object["profileFormatVersion"] = 5
            return object
        }
        private func projection(_ profiles: [GamepadConfigurationProfile]) throws -> Data {
            var result = root
            var records: [[String: Any]] = []
            for profile in profiles { records.append(try profileObject(profile)) }
            result["profiles"] = records
            return try JSONSerialization.data(withJSONObject: result, options: [.sortedKeys, .withoutEscapingSlashes])
        }
        private func canonicalOperation() throws -> ThumbleBridgeJSONValue {
            guard case .object(var fields) = request.operation, case .string(let type)? = fields["type"] else { throw ThumbleOwnedOperationBridgeError.invalidRequest }
            guard !type.hasPrefix("orientation.") else { throw ThumbleOwnedOperationBridgeError.retiredOperation }
            if let variant = fields["variant"] {
                guard variant == .string("primary") || variant == .string("landscape") else { throw ThumbleOwnedOperationBridgeError.retiredOperation }
                fields["variant"] = .string("primary")
            }
            return .object(fields)
        }
        private func applyOperation() throws {
            // Internal native-model views can use the legacy model's orientation
            // mirror. Those fields never enter the owned result or persistence.
            var document = root
            var profiles: [Any] = []
            for profile in beforeProfiles { profiles.append(try JSONSerialization.jsonObject(with: encoder.encode(profile))) }
            document["profiles"] = profiles
            let active = document["activeProfileID"]
            if document["defaultProfileID"] is NSNull { document["defaultProfileID"] = active }
            for key in ["schema", "version", "exportedAt"] { document.removeValue(forKey: key) }
            for key in ["keyBindings", "outputBindings", "profileKeyBindings", "profileOutputBindings"] where document[key] == nil { document[key] = [String: Any]() }
            let operation = try canonicalOperation()
            let object = try JSONSerialization.jsonObject(with: encoder.encode(operation))
            let bytes = try JSONSerialization.data(withJSONObject: ["schemaVersion":1, "nowMillis":request.nowMillis, "document":document, "operation":object], options: [.sortedKeys])
            let legacy = try JSONDecoder().decodeUnique(ThumbleConfigurationBridgeRequest.self, from: bytes)
            let response = try ThumbleConfigurationBridge.transform(legacy)
            paths = response.changedPaths
            try captureResult(response.document, operation: operation)
        }
        private func captureResult(_ document: ThumbleBridgeConfigurationDocument, operation: ThumbleBridgeJSONValue) throws {
            var result: [GamepadConfigurationProfile] = []
            for raw in document.profiles {
                var profile = try JSONDecoder().decode(GamepadConfigurationProfile.self, from: encoder.encode(raw))
                profile.customization = profile.customization.normalized
                profile.landscapeCustomization = nil; profile.portraitCustomization = nil
                profile.orientationPreference = .landscape
                try reconcileBank(&profile, operation: operation)
                result.append(profile)
            }
            root["activeProfileID"] = document.activeProfileID
            if !(root["defaultProfileID"] is NSNull) { root["defaultProfileID"] = document.defaultProfileID }
            if case .object(let fields) = operation, (fields["type"] == .string("profile.default") || fields["makeDefault"] == .bool(true)) { root["defaultProfileID"] = document.defaultProfileID }
            for (key, value) in [("keyBindings", document.keyBindings), ("outputBindings", document.outputBindings)] {
                root[key] = try JSONSerialization.jsonObject(with: encoder.encode(value))
            }
            root["profileKeyBindings"] = try JSONSerialization.jsonObject(with: encoder.encode(document.profileKeyBindings))
            root["profileOutputBindings"] = try JSONSerialization.jsonObject(with: encoder.encode(document.profileOutputBindings))
            after = try projection(result)
        }
        private func reconcileBank(_ profile: inout GamepadConfigurationProfile, operation: ThumbleBridgeJSONValue) throws {
            guard let bank = profile.appearanceBank else { return }
            let current = GamepadAppearanceSnapshot(profile.customization)
            guard bank.variants[0].appearance.controlIDs != current.controlIDs else { return }
            var duplicates: [String: String] = [:]
            if case .object(let fields) = operation, fields["type"] == .string("element.duplicate"),
               case .array(let sources)? = fields["elementIDs"], case .array(let destinations)? = fields["newElementIDs"] {
                for (source, destination) in zip(sources, destinations) {
                    if case .string(let oldID) = source, case .string(let newID) = destination {
                        duplicates[newID.lowercased()] = oldID.lowercased()
                    }
                }
            }
            profile.appearanceBank = try GamepadAppearanceBank(variants: bank.variants.map {
                .init(colorScheme: $0.colorScheme, actionSet: $0.actionSet,
                      appearance: $0.appearance.reconcilingControls(with: current, duplicates: duplicates))
            })
        }
    }
}
