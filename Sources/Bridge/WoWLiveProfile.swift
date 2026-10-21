import Foundation
import CryptoKit
import ImageIO
import UniformTypeIdentifiers

struct WoWLiveProfileRequest: Decodable {
    let schemaVersion: Int
    let wowProfileCommand: String
    let profileID: UUID
    let snapshotJSON: String
    let atlasPNGBase64: String
    let existingProfileJSON: String?
    let existingBindingsJSON: String?
}

struct WoWLiveProfileReply: Encodable {
    let schemaVersion = 2
    let sourceSHA256: String?
    let beforeProfile: ThumbleBridgeJSONValue?
    let document: ThumbleBridgeConfigurationDocument
}

enum WoWLiveProfile {
    struct Snapshot: Codable {
        struct Client: Codable { let flavor: String; let interface: Int }
        struct Character: Codable { let id: String; let name: String; let realm: String; let classTag: String }
        struct Action: Codable {
            let bank: ThumbleActionSet
            let button: VirtualGamepadButton
            let command: String
            let label: String
            let kind: String
            let slot: Int?
            let actionID: Int?
            let iconFileID: Int?
            let iconIndex: Int?
        }
        struct Device: Codable { let name: String; let vendorID: Int?; let productID: Int? }
        let schema: String
        let version: Int
        let client: Client
        let character: Character
        let actions: [Action]
        let icons: [Int]
        let devices: [Device]
        let consolePortVersion: String

        var profileID: UUID {
            let namespace = UUID(uuidString: "6ba7b811-9dad-11d1-80b4-00c04fd430c8")!
            var value = namespace.uuid
            let prefix = withUnsafeBytes(of: &value) { Data($0) }
            let name = "thumble:wow-live:\(client.flavor):\(character.id)"
            var bytes = Array(Insecure.SHA1.hash(data: prefix + Data(name.utf8)).prefix(16))
            bytes[6] = bytes[6] & 0x0f | 0x50; bytes[8] = bytes[8] & 0x3f | 0x80
            return UUID(uuid: (bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                               bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]))
        }

        func validate() throws {
            func text(_ value: String, _ count: Int) -> Bool {
                !value.isEmpty && value.utf8.count <= count && !value.unicodeScalars.contains { CharacterSet.controlCharacters.contains($0) }
            }
            guard schema == "thumble.wow-live", version == 1,
                  (client.flavor == "classic-era" && client.interface == 11509)
                    || (client.flavor == "forever" && client.interface == 16001),
                  character.id.hasPrefix("Player-"), text(character.id, 96),
                  character.id.utf8.allSatisfy({ (48...57).contains($0) || (65...90).contains($0) || (97...122).contains($0) || $0 == 45 }),
                  text(character.name, 96), text(character.realm, 96), text(character.classTag, 32),
                  text(consolePortVersion, 32), actions.count <= 64, icons.count <= 64, devices.count <= 16,
                  Set(icons).count == icons.count, icons.allSatisfy({ $0 > 0 && $0 <= Int(Int32.max) }) else { throw Failure.invalidSnapshot }
            var controls: Set<String> = []
            for action in actions {
                guard controls.insert(action.bank.rawValue + "/" + action.button.rawValue).inserted,
                      text(action.command, 128), text(action.label, 128), text(action.kind, 32),
                      action.slot.map({ (1...180).contains($0) }) ?? true,
                      action.actionID.map({ $0 > 0 && $0 <= Int(Int32.max) }) ?? true else { throw Failure.invalidSnapshot }
                if let index = action.iconIndex, let file = action.iconFileID {
                    guard index > 0, index <= icons.count, icons[index - 1] == file else { throw Failure.invalidSnapshot }
                } else if action.iconIndex != nil || action.iconFileID != nil { throw Failure.invalidSnapshot }
            }
            guard devices.allSatisfy({ text($0.name, 128) }) else { throw Failure.invalidSnapshot }
        }
    }

    enum Failure: Error { case invalidSnapshot, invalidAtlas, foreignProfile, invalidRequest }
    private static let assetPrefix = "wow-live-icon-"

    static func prepare(_ request: WoWLiveProfileRequest) throws -> WoWLiveProfileReply {
        guard request.schemaVersion == 2, request.wowProfileCommand == "prepare-owned",
              request.snapshotJSON.utf8.count <= 32768,
              request.atlasPNGBase64.utf8.count <= 2_796_204,
              let atlasData = Data(base64Encoded: request.atlasPNGBase64), atlasData.count <= 2 * 1024 * 1024,
              request.existingProfileJSON?.utf8.count ?? 0 <= 8 * 1024 * 1024,
              request.existingBindingsJSON?.utf8.count ?? 0 <= 1024 * 1024 else { throw Failure.invalidRequest }
        let snapshot = try JSONDecoder().decodeUnique(Snapshot.self, from: Data(request.snapshotJSON.utf8))
        try snapshot.validate()
        guard snapshot.profileID == request.profileID else { throw Failure.invalidRequest }
        var profile: GamepadConfigurationProfile
        var metadata: [String: ThumbleBridgeJSONValue] = [:]
        var beforeProfile: ThumbleBridgeJSONValue?
        if let existing = request.existingProfileJSON {
            let data = Data(existing.utf8)
            guard case .object(let fields) = try JSONDecoder().decodeUnique(ThumbleBridgeJSONValue.self, from: data),
                  case .object(let storedMetadata) = fields["wowLive"],
                  storedMetadata["characterID"] == .string(snapshot.character.id),
                  storedMetadata["flavor"] == .string(snapshot.client.flavor) else { throw Failure.foreignProfile }
            let bindingValue = try request.existingBindingsJSON.map {
                try JSONDecoder().decodeUnique(ThumbleBridgeJSONValue.self, from: Data($0.utf8))
            } ?? .object([:])
            guard case .object(let bindings) = bindingValue else { throw Failure.invalidRequest }
            let envelope = ThumbleBridgeJSONValue.object([
                "schema": .string(ThumbleKeypadConfigurationExport.schemaIdentifier), "version": .integer(5),
                "profiles": .array([.object(fields)]), "activeProfileID": .string(snapshot.profileID.uuidString),
                "defaultProfileID": .string(snapshot.profileID.uuidString),
                "profileKeyBindings": .object(bindings["keyBindings"].map { [snapshot.profileID.uuidString: $0] } ?? [:]),
                "profileOutputBindings": .object(bindings["outputBindings"].map { [snapshot.profileID.uuidString: $0] } ?? [:])])
            let owned = try ThumbleOwnedConfigurationDocument(validating: JSONEncoder().encode(envelope))
            guard let decoded = owned.projectedProfiles.first, decoded.id == snapshot.profileID,
                  decoded.outputMode == .controller else { throw Failure.foreignProfile }
            profile = decoded
            metadata = storedMetadata
            if let bindings = request.existingBindingsJSON {
                struct Bindings: Decodable { let outputBindings: [String: MacControlOutputBinding]? }
                let decoded = try JSONDecoder().decodeUnique(Bindings.self, from: Data(bindings.utf8))
                if let supplied = decoded.outputBindings {
                    var outputs: [KeypadElementID: MacControlOutputBinding] = [:]
                    for (key, value) in supplied {
                        guard let id = KeypadElementID(rawValue: key) else { throw Failure.invalidRequest }
                        outputs[id] = value
                    }
                    MacConfigurationBindings.synchronizeElementOutputs(in: &profile, outputs: outputs)
                }
            }
        } else {
            guard request.existingBindingsJSON == nil else { throw Failure.invalidRequest }
            profile = GamepadControllerTemplate.wowForever.makeProfile()
            profile.id = snapshot.profileID
            profile.name = "\(snapshot.character.name) · \(snapshot.character.realm) · ConsolePort"
        }
        // Both projections use the same native codec. The authority merges their
        // changes into exact source bytes; these values never replace raw metadata.
        if request.existingProfileJSON != nil { beforeProfile = try canonicalProfile(profile, metadata: metadata) }

        let assets = try iconAssets(snapshot, atlasData: atlasData)
        try update(&profile, snapshot: snapshot, assets: assets, isNew: request.existingProfileJSON == nil, metadata: &metadata)
        metadata["version"] = .integer(2)
        metadata["characterID"] = .string(snapshot.character.id)
        metadata["flavor"] = .string(snapshot.client.flavor)
        metadata["characterName"] = .string(snapshot.character.name)
        metadata["realm"] = .string(snapshot.character.realm)
        metadata["classTag"] = .string(snapshot.character.classTag)
        var after = try canonicalProfile(profile, metadata: metadata)
        if beforeProfile != after {
            profile.updatedAt = Date.currentMilliseconds
            after = try canonicalProfile(profile, metadata: metadata)
        }
        let values = [after]
        let outputs = profile.initialMacOutputBindings
        func json<T: Encodable>(_ value: T) throws -> ThumbleBridgeJSONValue {
            try JSONDecoder().decode(ThumbleBridgeJSONValue.self, from: JSONEncoder().encode(value))
        }
        let keys = try json(MacConfigurationBindings.rawKeyBindings(outputs.keyboardBindings))
        let rawOutputs = try json(MacConfigurationBindings.rawOutputs(outputs))
        let document = ThumbleBridgeConfigurationDocument(profiles: values, activeProfileID: profile.id.uuidString,
            defaultProfileID: profile.id.uuidString, keyBindings: keys, outputBindings: rawOutputs,
            profileKeyBindings: [profile.id.uuidString: keys], profileOutputBindings: [profile.id.uuidString: rawOutputs])
        var ownedFields = try JSONDecoder().decode(ThumbleBridgeJSONValue.self, from: JSONEncoder().encode(document))
        guard case .object(var fields) = ownedFields else { throw Failure.invalidRequest }
        fields["schema"] = .string(ThumbleKeypadConfigurationExport.schemaIdentifier)
        fields["version"] = .integer(5)
        ownedFields = .object(fields)
        _ = try ThumbleOwnedConfigurationDocument(validating: JSONEncoder().encode(ownedFields))
        return WoWLiveProfileReply(sourceSHA256: request.existingProfileJSON.map { ProfileArtifactAdoptionMetadata.sha256(Data($0.utf8)) },
            beforeProfile: beforeProfile, document: document)
    }

    private static func canonicalProfile(_ profile: GamepadConfigurationProfile,
                                         metadata: [String: ThumbleBridgeJSONValue]) throws -> ThumbleBridgeJSONValue {
        guard case .object(var fields) = try JSONDecoder().decode(ThumbleBridgeJSONValue.self,
            from: JSONEncoder().encode(profile)) else { throw Failure.invalidRequest }
        for key in ["skinReference", "skinBaselineCustomization", "landscapeSkinBaselineCustomization",
                    "portraitSkinBaselineCustomization", "landscapeCustomization", "portraitCustomization", "orientationPreference"] {
            fields.removeValue(forKey: key)
        }
        fields["profileFormatVersion"] = .integer(5)
        fields["wowLive"] = .object(metadata)
        return .object(fields)
    }

    private static func iconAssets(_ snapshot: Snapshot, atlasData: Data) throws -> [GamepadAsset] {
        guard atlasData.starts(with: [137,80,78,71,13,10,26,10]),
              let source = CGImageSourceCreateWithData(atlasData as CFData, nil),
              let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
              let width = properties[kCGImagePropertyPixelWidth] as? Int,
              let height = properties[kCGImagePropertyPixelHeight] as? Int,
              width == height, width >= 128, width <= 2048,
              let atlas = CGImageSourceCreateImageAtIndex(source, 0, nil) else { throw Failure.invalidAtlas }
        return try snapshot.icons.enumerated().map { index, fileID in
            let cell = Double(width) / 8
            let rect = CGRect(x: Double(index % 8) * cell, y: Double(index / 8) * cell, width: cell, height: cell).integral
            guard let icon = atlas.cropping(to: rect) else { throw Failure.invalidAtlas }
            let data = NSMutableData()
            guard let destination = CGImageDestinationCreateWithData(data, UTType.png.identifier as CFString, 1, nil) else { throw Failure.invalidAtlas }
            CGImageDestinationAddImage(destination, icon, nil)
            guard CGImageDestinationFinalize(destination), data.length <= GamepadAsset.maximumStoredBytes else { throw Failure.invalidAtlas }
            return GamepadAsset(id: assetPrefix + String(fileID), name: snapshot.actions.first(where: { $0.iconFileID == fileID })?.label ?? "WoW icon",
                contentType: "image/png", data: data as Data, role: .icon)
        }
    }

    private static func update(_ profile: inout GamepadConfigurationProfile, snapshot: Snapshot, assets: [GamepadAsset], isNew: Bool, metadata: inout [String: ThumbleBridgeJSONValue]) throws {
        let previousBank = profile.appearanceBank
        let base = profile.customization
        var variants: [GamepadAppearanceBank.Variant] = []
        var updatedBase = base
        let oldIcons = object(metadata["managedIcons"]), oldAssets = object(metadata["managedAssets"])
        let oldIdentities = object(metadata["managedIconIdentities"])
        var newIcons = oldIcons, newAssets = oldAssets, newIdentities = oldIdentities
        for scheme in ThumbleSkinColorScheme.allCases {
            for bank in ThumbleActionSet.allCases {
                var layout = previousBank?.resolve(base, colorScheme: scheme, actionSet: bank) ?? base
                try updateAppearance(&layout, scheme: scheme, bank: bank, snapshot: snapshot, assets: assets, isNew: isNew,
                    oldIcons: oldIcons, oldAssets: oldAssets, oldIdentities: oldIdentities,
                    newIcons: &newIcons, newAssets: &newAssets, newIdentities: &newIdentities)
                variants.append(.init(colorScheme: scheme, actionSet: bank, appearance: GamepadAppearanceSnapshot(layout)))
                if scheme == .light && bank == .base { updatedBase = layout }
            }
        }
        profile.customization = updatedBase
        if var layout = profile.landscapeCustomization {
            try updateAppearance(&layout, scheme: .light, bank: .base, snapshot: snapshot, assets: assets, isNew: isNew,
                oldIcons: oldIcons, oldAssets: oldAssets, oldIdentities: oldIdentities,
                    newIcons: &newIcons, newAssets: &newAssets, newIdentities: &newIdentities)
            profile.landscapeCustomization = layout
        }
        if var layout = profile.portraitCustomization {
            try updateAppearance(&layout, scheme: .light, bank: .base, snapshot: snapshot, assets: assets, isNew: isNew,
                oldIcons: oldIcons, oldAssets: oldAssets, oldIdentities: oldIdentities,
                    newIcons: &newIcons, newAssets: &newAssets, newIdentities: &newIdentities)
            profile.portraitCustomization = layout
        }
        profile.appearanceBank = try GamepadAppearanceBank(variants: variants)
        metadata["managedIcons"] = .object(newIcons)
        metadata["managedAssets"] = .object(newAssets)
        metadata["managedIconIdentities"] = .object(newIdentities)
    }

    private static func object(_ value: ThumbleBridgeJSONValue?) -> [String: ThumbleBridgeJSONValue] {
        if case .object(let fields) = value { return fields }; return [:]
    }
    // Identity ownership is independent of authored placement, scale and tint.
    // Existing full stamps remain the conservative migration path; a user choosing
    // a different asset or removing an icon still opts that control out of refresh.
    private static func iconIdentity(_ icon: GamepadControlIcon?) -> ThumbleBridgeJSONValue {
        guard let icon else { return .null }
        return .string(icon.source.rawValue + "/" + icon.value)
    }

    private static func stamp<T: Encodable>(_ value: T) throws -> String {
        let encoder = JSONEncoder(); encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
        return ProfileArtifactAdoptionMetadata.sha256(try encoder.encode(value))
    }

    private static func updateAppearance(_ layout: inout GamepadCustomization, scheme: ThumbleSkinColorScheme, bank: ThumbleActionSet,
                                         snapshot: Snapshot, assets: [GamepadAsset], isNew: Bool,
                                         oldIcons: [String: ThumbleBridgeJSONValue], oldAssets: [String: ThumbleBridgeJSONValue],
                                         oldIdentities: [String: ThumbleBridgeJSONValue],
                                         newIcons: inout [String: ThumbleBridgeJSONValue], newAssets: inout [String: ThumbleBridgeJSONValue],
                                         newIdentities: inout [String: ThumbleBridgeJSONValue]) throws {
        let byButton = Dictionary(uniqueKeysWithValues: snapshot.actions.filter { $0.bank == bank }.map { ($0.button, $0) })
        var library = layout.assetLibrary.assets
        for asset in assets {
            if let index = library.firstIndex(where: { $0.id == asset.id }) {
                let current = try stamp(library[index])
                guard isNew || oldAssets[asset.id] == .string(current) else { continue }
                library[index] = asset
            } else { library.append(asset) }
            newAssets[asset.id] = .string(try stamp(asset))
        }
        layout.assetLibrary = GamepadAssetLibrary(assets: library)
        for index in layout.elements.indices {
            let element = layout.elements[index]
            guard element.kind == .button, let output = element.output,
                  output.keyboard == nil, output.gamepadButtons.count == 1,
                  let button = output.gamepadButtons.first else { continue }
            let icon = element.layout.icon
            let address = scheme.rawValue + "/" + bank.rawValue + "/" + element.id.uuidString.lowercased()
            let currentStamp = try stamp(icon)
            // Compare with the last generated appearance, including nil. A user
            // may remove an icon or choose another icon from the managed library.
            let identity = iconIdentity(icon)
            let owned = oldIdentities[address].map { $0 == identity }
                ?? (oldIcons[address].map { $0 == .string(currentStamp) } ?? (icon == nil))
            guard isNew || owned else { continue }
            let fileID = byButton[button]?.iconFileID
            var appearance = element.layout
            appearance.icon = fileID.map {
                var refreshed = icon ?? GamepadControlIcon(source: .asset, value: assetPrefix + String($0), placement: .top, renderingMode: .original)
                refreshed.source = .asset
                refreshed.value = assetPrefix + String($0)
                return refreshed
            }
            newIdentities[address] = iconIdentity(appearance.icon)
            newIcons[address] = .string(try stamp(appearance.icon))
            layout.elements[index].layout = appearance
            if let buttonID = element.defaultControlID { layout.buttonCustomizations[buttonID] = appearance }
            if let customIndex = layout.customButtons.firstIndex(where: { $0.id == element.id }) {
                layout.customButtons[customIndex].layout = appearance
            }
        }
    }
}
