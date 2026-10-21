import Foundation

/// Reducer for the retained Apple phone transport. Every result is a proposed
/// transaction; the caller saves its cache before publishing it or sending edits.
public struct OwnedPhoneSession {
    private let storage: Storage
    private final class Storage {
        let cache: OwnedPhoneCache
        let profiles: [GamepadConfigurationProfile]
        let review: Set<UUID>
        init(_ cache: OwnedPhoneCache, profiles: [GamepadConfigurationProfile], review: Set<UUID>) {
            self.cache = cache; self.profiles = profiles; self.review = review
        }
    }
    public enum Failure: Error { case missingSnapshot, awaitingSnapshot, profileMissing, invalidTime }
    public var cache: OwnedPhoneCache { storage.cache }
    public var profiles: [GamepadConfigurationProfile] { storage.profiles }
    public var profilesNeedingReview: Set<UUID> { storage.review }
    public init(cache: OwnedPhoneCache) {
        var profiles = cache.snapshot?.profiles ?? []
        var review: Set<UUID> = []
        if let server = cache.snapshot?.serverID {
            for pending in cache.journal.pending where pending.serverID == server {
                guard let index = profiles.firstIndex(where: { $0.id == pending.edit.profileID }) else { review.insert(pending.edit.profileID); continue }
                do {
                    profiles[index].customization = try OwnedPhoneLayoutAdapter.applying(pending.edit, to: profiles[index].customization)
                    profiles[index].updatedAt = max(profiles[index].updatedAt, pending.updatedAt)
                } catch { review.insert(pending.edit.profileID) }
            }
        }
        storage = Storage(cache, profiles: profiles, review: review)
    }
    public func receiving(_ snapshot: OwnedPhoneSnapshot) -> Self {
        Self(cache: .init(snapshot: snapshot, journal: cache.journal))
    }
    public func recording(profileID: UUID, customization: GamepadCustomization, nowMillis: Int64,
                          editID: UUID = UUID()) throws -> Self {
        guard nowMillis >= 0 else { throw Failure.invalidTime }
        guard let snapshot = cache.snapshot else { throw Failure.missingSnapshot }
        guard let profile = profiles.first(where: { $0.id == profileID }) else { throw Failure.profileMissing }
        guard let intent = try OwnedPhoneLayoutAdapter.intent(profileID: profileID, before: profile.customization,
            after: customization, baseConfigurationRevision: snapshot.configurationRevision, editID: editID) else { return self }
        let journal = try cache.journal.recording(serverID: snapshot.serverID, edit: intent, updatedAt: nowMillis, recoveryDocument: snapshot.documentData)
        return Self(cache: .init(snapshot: snapshot, journal: journal, localSelection: cache.localSelection, localDefault: cache.localDefault))
    }
    public func acknowledging(_ message: ControllerMessage, fromAuthenticatedServerID serverID: String) throws -> Self {
        guard let snapshot = cache.snapshot, snapshot.serverID == serverID else { throw Failure.missingSnapshot }
        let journal = try cache.journal.acknowledging(message, fromAuthenticatedServerID: serverID, negotiatedProfileFormat: 5)
        if let receipt = message.ownedPhoneEditResult, let committed = receipt.committedRevision,
           snapshot.configurationRevision < committed { throw Failure.awaitingSnapshot }
        return Self(cache: .init(snapshot: snapshot, journal: journal, localSelection: cache.localSelection, localDefault: cache.localDefault))
    }
    public func retryingConflict(_ message: ControllerMessage, fromAuthenticatedServerID serverID: String,
                                editID: UUID = UUID()) throws -> Self {
        guard let snapshot = cache.snapshot else { throw Failure.missingSnapshot }
        let journal = try cache.journal.retryingConflict(message, fromAuthenticatedServerID: serverID, using: snapshot, editID: editID)
        return Self(cache: .init(snapshot: snapshot, journal: journal, localSelection: cache.localSelection, localDefault: cache.localDefault))
    }
    public func selecting(_ profileID: UUID, makeDefault: Bool = false) throws -> Self {
        guard profiles.contains(where: { $0.id == profileID }) else { throw Failure.profileMissing }
        return Self(cache: .init(snapshot: cache.snapshot, journal: cache.journal,
            localSelection: makeDefault ? cache.localSelection : profileID,
            localDefault: makeDefault ? profileID : cache.localDefault))
    }
    /// Original intent bytes are embedded verbatim on every reliable replay.
    public static func wireData(for pending: OwnedPhoneEditJournal.Pending) throws -> Data {
        guard pending.edit.originalData.count <= OwnedPhoneEdit.maximumBytes else { throw OwnedPhoneSnapshot.Failure.tooLarge }
        var bytes = Data(#"{"type":"owned_phone_edit","timestamp":0,"profileFormatVersion":5,"ownedPhoneEdit":"#.utf8)
        bytes.append(pending.edit.originalData); bytes.append(125)
        return bytes
    }
}
