import Foundation

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
            id = try fields.decode(String.self, forKey: .id)
            name = try fields.decode(String.self, forKey: .name)
            updatedAt = try fields.decode(Int64.self, forKey: .updatedAt)
            guard PortableProfileArtifact.canonicalUUID(id) != nil,
                  !name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
                  name.unicodeScalars.count <= 256, updatedAt >= 0 else { throw RecoveryRequired.invalidSource }
        }
        enum Keys: String, CodingKey { case id, name, updatedAt, profileFormatVersion, appearanceBank, appearanceResources, appearanceResourceVersion, landscapeCustomization, portraitCustomization }
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
