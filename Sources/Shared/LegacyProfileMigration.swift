import Foundation
import CryptoKit

/// Native preparation for the lossless authority migration. This never writes
/// a user profile or substitutes for source reconciliation/backup/CAS.
public enum ThumbleLegacyProfileMigration {
    public enum SourceLayout: String { case primary = "customization", landscape = "landscapeCustomization", portrait = "portraitCustomization" }
    public enum ReviewReason: String { case portraitArranged, alternateLayoutArchived, canvasOrientationCorrected }
    public enum RecoveryRequired: Error, Equatable {
        case invalidSource, alreadyOwnedSource, tooLarge, variantOnlyControls([UUID]), changedExecutableControls
    }
    public struct Preparation {
        private let storage: Storage
        public var originalData: Data { storage.originalData }
        public var originalProfile: GamepadConfigurationProfile { storage.original }
        public var sourceCustomization: GamepadCustomization { storage.source }
        public var sourceLayout: SourceLayout { storage.slot }
        public var sourceOrientation: GamepadEditorDeviceOrientation { storage.orientation }
        /// Typed known fields only. Not a v5 export; raw metadata must be merged
        /// by the authority before destination validation and atomic commit.
        public var stagingProfile: GamepadConfigurationProfile { storage.staging }
        public var reviewReasons: [ReviewReason] { storage.review }
        fileprivate init(_ storage: Storage) { self.storage = storage }
    }
    fileprivate final class Storage {
        let originalData: Data
        let original: GamepadConfigurationProfile
        let source: GamepadCustomization
        let slot: SourceLayout
        let orientation: GamepadEditorDeviceOrientation
        let staging: GamepadConfigurationProfile
        let review: [ReviewReason]
        init(data: Data, original: GamepadConfigurationProfile, source: GamepadCustomization,
             slot: SourceLayout, orientation: GamepadEditorDeviceOrientation,
             staging: GamepadConfigurationProfile, review: [ReviewReason]) {
            originalData = data; self.original = original; self.source = source
            self.slot = slot; self.orientation = orientation; self.staging = staging; self.review = review
        }
    }
    public static func prepare(profileData: Data, package: ThumbleSkinPackage?) throws -> Preparation {
        try Workspace(profileData, package: package).prepare()
    }
    private struct Header: Decodable {
        let id: String
        let name: String
        let updatedAt: Int64
        let hasLandscape: Bool
        let hasPortrait: Bool
        let primaryFrame: String?
        let landscapeFrame: String?
        let portraitFrame: String?
        init(from decoder: Decoder) throws {
            let fields = try decoder.container(keyedBy: Keys.self)
            let version = try fields.decodeIfPresent(Int.self, forKey: .profileFormatVersion) ?? 4
            if version != 4 {
                throw RecoveryRequired.alreadyOwnedSource
            }
            let hasBank: Bool
            if fields.contains(.appearanceBank) { hasBank = !(try fields.decodeNil(forKey: .appearanceBank)) } else { hasBank = false }
            if hasBank || fields.contains(.appearanceResources) || fields.contains(.appearanceResourceVersion) {
                throw RecoveryRequired.alreadyOwnedSource
            }
            hasLandscape = try fields.contains(.landscapeCustomization) && !fields.decodeNil(forKey: .landscapeCustomization)
            hasPortrait = try fields.contains(.portraitCustomization) && !fields.decodeNil(forKey: .portraitCustomization)
            primaryFrame = try fields.decodeIfPresent(LayoutHeader.self, forKey: .customization)?.deviceCanvas?.frameID
            landscapeFrame = try fields.decodeIfPresent(LayoutHeader.self, forKey: .landscapeCustomization)?.deviceCanvas?.frameID
            portraitFrame = try fields.decodeIfPresent(LayoutHeader.self, forKey: .portraitCustomization)?.deviceCanvas?.frameID
            id = try fields.decode(String.self, forKey: .id)
            name = try fields.decode(String.self, forKey: .name)
            updatedAt = try fields.decode(Int64.self, forKey: .updatedAt)
            guard PortableProfileArtifact.canonicalUUID(id) != nil,
                  !name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
                  name.unicodeScalars.count <= 256, updatedAt >= 0 else { throw RecoveryRequired.invalidSource }
        }
        private struct LayoutHeader: Decodable { let deviceCanvas: CanvasHeader? }
        private struct CanvasHeader: Decodable { let frameID: String }
        enum Keys: String, CodingKey { case id, name, updatedAt, profileFormatVersion, appearanceBank, appearanceResources, appearanceResourceVersion, customization, landscapeCustomization, portraitCustomization }
    }
    private final class Workspace {
        let data: Data
        let package: ThumbleSkinPackage?
        var original: GamepadConfigurationProfile?
        var header: Header?
        var source: GamepadCustomization?
        var canonical: GamepadCustomization?
        var variants: [GamepadAppearanceBank.Variant] = []
        var orientation = GamepadEditorDeviceOrientation.landscape
        var slot = SourceLayout.primary
        var review: [ReviewReason] = []
        init(_ data: Data, package: ThumbleSkinPackage?) { self.data = data; self.package = package }
        func prepare() throws -> Preparation {
            try decodeSource()
            try chooseSource()
            try materializeAppearance()
            try prepareLandscape()
            return try finish()
        }
        private func decodeSource() throws {
            guard data.count <= ThumbleOwnedConfigurationDocument.maximumBytes else { throw RecoveryRequired.tooLarge }
            header = try JSONDecoder().decodeUnique(Header.self, from: data)
            original = try JSONDecoder().decode(GamepadConfigurationProfile.self, from: data)
        }
        private func chooseSource() throws {
            try chooseSourceSlot()
            try validateSourceFrame()
            source = original?.customization(for: orientation)
            try validateControlMembership()
            recordSourceReview()
        }
        private func chooseSourceSlot() throws {
            guard let original, let header else { throw RecoveryRequired.invalidSource }
            if header.hasLandscape { slot = .landscape; orientation = .landscape }
            else if original.orientationPreference == .portrait {
                slot = header.hasPortrait ? .portrait : .primary; orientation = .portrait
            } else if original.orientationPreference == .landscape || primaryIsLandscape() {
                slot = .primary; orientation = .landscape
            } else {
                slot = header.hasPortrait ? .portrait : .primary; orientation = .portrait
            }
        }
        private func validateSourceFrame() throws {
            guard let header else { throw RecoveryRequired.invalidSource }
            let stored: String?
            switch slot {
            case .primary: stored = header.primaryFrame
            case .landscape: stored = header.landscapeFrame
            case .portrait: stored = header.portraitFrame
            }
            if let stored, !stored.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
               GamepadEditorDeviceCatalog.frame(forStoredID: stored) == nil { throw RecoveryRequired.invalidSource }
        }
        private func primaryIsLandscape() -> Bool {
            original?.customization.deviceCanvas.editorDeviceFrame.isLandscape ?? false
        }
        // Each projection owns at most one large legacy layout temporary.
        private func primaryIDs() -> Set<UUID> { Set(original?.customization.elements.map(\.id) ?? []) }
        private func landscapeIDs() -> Set<UUID> { Set(original?.landscapeCustomization?.elements.map(\.id) ?? []) }
        private func portraitIDs() -> Set<UUID> { Set(original?.portraitCustomization?.elements.map(\.id) ?? []) }
        private func selectedIDs() -> Set<UUID> { Set(source?.elements.map(\.id) ?? []) }
        private func validateControlMembership() throws {
            var allIDs = primaryIDs()
            allIDs.formUnion(landscapeIDs())
            allIDs.formUnion(portraitIDs())
            let omitted = allIDs.subtracting(selectedIDs()).sorted { $0.uuidString < $1.uuidString }
            guard omitted.isEmpty else { throw RecoveryRequired.variantOnlyControls(omitted) }
        }
        private func recordSourceReview() {
            if header?.hasPortrait == true || slot != .primary { review.append(.alternateLayoutArchived) }
            if source?.deviceCanvas.editorDeviceFrame.orientation != orientation { review.append(.canvasOrientationCorrected) }
        }
        private func materializeAppearance() throws {
            guard let original, let source else { throw RecoveryRequired.invalidSource }
            variants = try ThumbleLegacyAppearanceMaterializer.materialize(original, orientation: orientation, package: package)
            guard let reference = variants.first(where: { $0.colorScheme == .light && $0.actionSet == .base }) else { throw RecoveryRequired.invalidSource }
            canonical = reference.appearance.applying(to: source)
        }
        private func prepareLandscape() throws {
            guard let original, var canonical else { throw RecoveryRequired.invalidSource }
            let frame = canonical.deviceCanvas.editorDeviceFrame
            canonical.deviceCanvas = GamepadDeviceCanvas(frameID: GamepadEditorDeviceFrame(spec: frame.spec, orientation: orientation).id)
            if orientation == .portrait {
                // The ordinary copy tool arranges visible controls. Migration
                // must retain the coordinate basis of hidden controls as well.
                let hidden = Dictionary(uniqueKeysWithValues: canonical.elements.map { ($0.id, $0.layout.isHidden) })
                var visible = canonical
                for index in visible.elements.indices { visible.elements[index].layout.isHidden = false }
                for key in Array(visible.buttonCustomizations.keys) { visible.buttonCustomizations[key]?.isHidden = false }
                for index in visible.customButtons.indices { visible.customButtons[index].layout.isHidden = false }
                var geometry = GamepadConfigurationProfile(id: original.id, name: original.name,
                    primaryCustomization: visible, outputMode: original.outputMode, launchTarget: original.launchTarget, updatedAt: original.updatedAt)
                geometry.copyLayoutVariant(from: .portrait, to: .landscape, automaticallyArrange: true)
                canonical = geometry.customization
                for index in canonical.elements.indices { canonical.elements[index].layout.isHidden = hidden[canonical.elements[index].id] ?? false }
                for key in Array(canonical.buttonCustomizations.keys) { canonical.buttonCustomizations[key]?.isHidden = hidden[key.uuid] ?? false }
                for index in canonical.customButtons.indices { canonical.customButtons[index].layout.isHidden = hidden[canonical.customButtons[index].id] ?? false }
                canonical.normalizeInPlace()
                review.append(.portraitArranged)
            }
            canonical.updatedAt = source?.updatedAt ?? canonical.updatedAt
            self.canonical = canonical
        }
        private func finish() throws -> Preparation {
            guard let original, let source, let canonical, canonical.deviceCanvas.editorDeviceFrame.isLandscape else { throw RecoveryRequired.invalidSource }
            // Appearance/geometry are allowed to change; every other declared
            // field (including analog/pointer settings and presentation IDs) is not.
            guard canonical.elements.count == source.elements.count else { throw RecoveryRequired.changedExecutableControls }
            var declarations = canonical.elements
            for index in declarations.indices { declarations[index].layout = source.elements[index].layout }
            guard declarations == source.elements else { throw RecoveryRequired.changedExecutableControls }
            var staging = GamepadConfigurationProfile(id: original.id, name: original.name,
                primaryCustomization: canonical, orientationPreference: .landscape, outputMode: original.outputMode,
                launchTarget: original.launchTarget, updatedAt: original.updatedAt)
            if original.skinReference != nil { staging.appearanceBank = try GamepadAppearanceBank(variants: variants) }
            return Preparation(Storage(data: data, original: original, source: source, slot: slot,
                orientation: orientation, staging: staging, review: review))
        }
    }
}

/// Internal pure preparation transport. Bytes stay separate from display JSON
/// values so unknown source fields never enter a normalized projection.
public struct ThumbleLegacyProfileBridgeRequest: Decodable, Sendable {
    public let schemaVersion: Int
    public let invocationID: UUID
    public let legacyProfileCommand: String
    public let profileData: Data
    public let packageData: Data?

    public init(from decoder: Decoder) throws {
        let fields = try decoder.container(keyedBy: Field.self)
        guard Set(fields.allKeys.map(\.stringValue)).isSubset(of:
            ["schemaVersion", "invocationID", "legacyProfileCommand", "profileData", "packageData"])
        else { throw ThumbleLegacyProfileBridgeError.invalidRequest }
        schemaVersion = try fields.decode(Int.self, forKey: Field("schemaVersion"))
        invocationID = try fields.decode(UUID.self, forKey: Field("invocationID"))
        legacyProfileCommand = try fields.decode(String.self, forKey: Field("legacyProfileCommand"))
        profileData = try Self.data(fields.decode(String.self, forKey: Field("profileData")),
                                    maximum: ThumbleOwnedConfigurationDocument.maximumBytes)
        if let encoded = try fields.decodeIfPresent(String.self, forKey: Field("packageData")) {
            packageData = try Self.data(encoded, maximum: ThumbleSkinPackageCodec.maximumArchiveBytes)
        } else { packageData = nil }
    }
    private static func data(_ encoded: String, maximum: Int) throws -> Data {
        guard encoded.utf8.count <= ((maximum + 2) / 3) * 4,
              let data = Data(base64Encoded: encoded), data.count <= maximum,
              data.base64EncodedString() == encoded else { throw ThumbleLegacyProfileBridgeError.invalidRequest }
        return data
    }
    private struct Field: CodingKey {
        let stringValue: String
        let intValue: Int? = nil
        init(_ string: String) { stringValue = string }
        init?(stringValue: String) { self.init(stringValue) }
        init?(intValue: Int) { return nil }
    }
}

public struct ThumbleLegacyProfileBridgeResponse: Encodable, Sendable {
    public let schemaVersion = 1
    public let invocationID: UUID
    public let profileID: UUID
    public let sourceSHA256: String
    public let packageSHA256: String?
    public let sourceLayout: String
    public let sourceOrientation: String
    public let reviewReasons: [String]
    public let beforeCustomizationData: Data
    public let afterProfileData: Data
}

public enum ThumbleLegacyProfileBridgeError: String, Error {
    case invalidRequest = "invalid_legacy_preparation_request"
    case invalidPackage = "invalid_legacy_package"
    case missingPackage = "legacy_package_missing"
    case appearanceRecovery = "legacy_appearance_recovery_required"
    case profileRecovery = "legacy_profile_recovery_required"
    case tooLarge = "legacy_preparation_too_large"
}

public enum ThumbleLegacyProfileBridge {
    public static func transform(_ request: ThumbleLegacyProfileBridgeRequest) throws -> ThumbleLegacyProfileBridgeResponse {
        guard request.schemaVersion == 1, request.legacyProfileCommand == "prepare" else {
            throw ThumbleLegacyProfileBridgeError.invalidRequest
        }
        return try Workspace(request).run()
    }
    private final class Workspace {
        let request: ThumbleLegacyProfileBridgeRequest
        var preparation: ThumbleLegacyProfileMigration.Preparation?
        init(_ request: ThumbleLegacyProfileBridgeRequest) { self.request = request }
        func run() throws -> ThumbleLegacyProfileBridgeResponse {
            let package: ThumbleSkinPackage?
            do { package = try request.packageData.map { try ThumbleSkinPackageCodec.decode($0) } }
            catch { throw ThumbleLegacyProfileBridgeError.invalidPackage }
            do { preparation = try ThumbleLegacyProfileMigration.prepare(profileData: request.profileData, package: package) }
            catch ThumbleLegacyAppearanceMaterializer.RecoveryRequired.missingPackage { throw ThumbleLegacyProfileBridgeError.missingPackage }
            catch is ThumbleLegacyAppearanceMaterializer.RecoveryRequired { throw ThumbleLegacyProfileBridgeError.appearanceRecovery }
            catch { throw ThumbleLegacyProfileBridgeError.profileRecovery }
            return try response()
        }
        private func literalSource() throws -> GamepadCustomization {
            guard let preparation else { throw ThumbleLegacyProfileBridgeError.profileRecovery }
            switch preparation.sourceLayout {
            case .primary: return preparation.originalProfile.customization
            case .landscape: return try landscapeSource()
            case .portrait: return try portraitSource()
            }
        }
        private func landscapeSource() throws -> GamepadCustomization {
            guard let layout = preparation?.originalProfile.landscapeCustomization else { throw ThumbleLegacyProfileBridgeError.profileRecovery }
            return layout
        }
        private func portraitSource() throws -> GamepadCustomization {
            guard let layout = preparation?.originalProfile.portraitCustomization else { throw ThumbleLegacyProfileBridgeError.profileRecovery }
            return layout
        }
        private func response() throws -> ThumbleLegacyProfileBridgeResponse {
            guard let preparation, request.packageData == nil || preparation.originalProfile.skinReference != nil else {
                throw ThumbleLegacyProfileBridgeError.invalidRequest
            }
            let encoder = JSONEncoder()
            encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
            let before = try encoder.encode(literalSource())
            let after = try encoder.encode(preparation.stagingProfile)
            guard before.count <= ThumbleOwnedConfigurationDocument.maximumBytes,
                  after.count <= ThumbleOwnedConfigurationDocument.maximumBytes else { throw ThumbleLegacyProfileBridgeError.tooLarge }
            return ThumbleLegacyProfileBridgeResponse(invocationID: request.invocationID, profileID: preparation.originalProfile.id,
                sourceSHA256: Self.digest(request.profileData), packageSHA256: request.packageData.map(Self.digest),
                sourceLayout: preparation.sourceLayout.rawValue, sourceOrientation: preparation.sourceOrientation.rawValue,
                reviewReasons: preparation.reviewReasons.map(\.rawValue), beforeCustomizationData: before, afterProfileData: after)
        }
        private static func digest(_ data: Data) -> String { SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined() }
    }
}
