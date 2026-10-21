import Foundation
import CoreGraphics

/// Converts phone editor changes into the scalar-only protocol. A native render
/// projection is never used as a replacement document, paint bank or output map.
public enum OwnedPhoneLayoutAdapter {
    public enum Failure: Error { case unsupportedEdit, missingControl, invalidOrientation }
    public static func intent(profileID: UUID, before: GamepadCustomization, after: GamepadCustomization,
                              baseConfigurationRevision: UInt64, editID: UUID = UUID()) throws -> OwnedPhoneEdit? {
        try Workspace(profileID: profileID, before: before, after: after,
                      revision: baseConfigurationRevision, editID: editID).prepare()
    }
    private final class Workspace {
        let profileID: UUID
        let before: GamepadCustomization
        let after: GamepadCustomization
        let revision: UInt64
        let editID: UUID
        init(profileID: UUID, before: GamepadCustomization, after: GamepadCustomization, revision: UInt64, editID: UUID) {
            self.profileID = profileID; self.before = before.normalized; self.after = after.normalized
            self.revision = revision; self.editID = editID
        }
        func prepare() throws -> OwnedPhoneEdit? {
            guard before.deviceCanvas.editorDeviceFrame.isLandscape, after.deviceCanvas.editorDeviceFrame.isLandscape else { throw Failure.invalidOrientation }
            let originals = Dictionary(uniqueKeysWithValues: before.elements.map { ($0.id, $0) })
            let updated = Dictionary(uniqueKeysWithValues: after.elements.map { ($0.id, $0) })
            guard Set(updated.keys).isSubset(of: Set(originals.keys)) else { throw Failure.unsupportedEdit }
            var edits: [[String: Any]] = []
            var removed: [String] = []
            for element in before.elements {
                guard let next = updated[element.id] else { removed.append(element.id.uuidString); continue }
                let oldStyle = style(element, in: before)
                let newStyle = style(next, in: after)
                var fields: [String: Any] = ["controlID": element.id.uuidString]
                try optionalPosition("centerX", oldStyle.centerX, newStyle.centerX, into: &fields)
                try optionalPosition("centerY", oldStyle.centerY, newStyle.centerY, into: &fields)
                for (key, old, new) in [("widthScale", oldStyle.widthScale, newStyle.widthScale),
                                         ("heightScale", oldStyle.heightScale, newStyle.heightScale),
                                         ("rotationDegrees", oldStyle.rotationDegrees, newStyle.rotationDegrees)] where old != new { fields[key] = Double(new) }
                if oldStyle.zIndex != newStyle.zIndex { fields["zIndex"] = newStyle.zIndex }
                if oldStyle.isHidden != newStyle.isHidden { fields["isHidden"] = newStyle.isHidden }
                if oldStyle.isLocationLocked != newStyle.isLocationLocked { fields["isLocationLocked"] = newStyle.isLocationLocked }
                if fields.count > 1 { edits.append(fields) }
            }
            var packet: [String: Any] = ["schemaVersion": 1, "editID": editID.uuidString, "profileID": profileID.uuidString,
                                         "baseConfigurationRevision": NSNumber(value: revision)]
            if !edits.isEmpty { packet["controlEdits"] = edits }
            if !removed.isEmpty { packet["removedControlIDs"] = removed }
            if before.colorSchemePreference != after.colorSchemePreference { packet["colorScheme"] = after.colorSchemePreference.rawValue }
            if packet.count == 4 {
                guard samePresentation(before, after) else { throw Failure.unsupportedEdit }
                return nil
            }
            let intent = try OwnedPhoneEdit(validating: JSONSerialization.data(withJSONObject: packet, options: [.sortedKeys]))
            let preview = try applying(intent, to: before)
            guard samePresentation(preview, after) else { throw Failure.unsupportedEdit }
            return intent
        }
        private func optionalPosition(_ key: String, _ old: CGFloat?, _ new: CGFloat?, into fields: inout [String: Any]) throws {
            guard old != new else { return }
            // The wire cannot clear a coordinate to an implicit preset position.
            guard let new else { throw Failure.unsupportedEdit }
            fields[key] = Double(new)
        }
    }
    private static func style(_ element: KeypadElement, in layout: GamepadCustomization) -> GamepadButtonCustomization {
        if element.kind == .button, let builtin = element.defaultControlID { return layout.buttonCustomization(for: builtin) }
        return element.layout
    }
    private static func samePresentation(_ lhs: GamepadCustomization, _ rhs: GamepadCustomization) -> Bool {
        var left = lhs.normalized; var right = rhs.normalized
        left.updatedAt = 0; right.updatedAt = 0
        return left == right
    }
    public static func applying(_ intent: OwnedPhoneEdit, to layout: GamepadCustomization) throws -> GamepadCustomization {
        var result = layout.normalized
        let declared = Set(result.elements.map(\.id))
        let requested = Set(intent.controlEdits.map(\.controlID)).union(intent.removedControlIDs)
        guard requested.isSubset(of: declared) else { throw Failure.missingControl }
        for id in intent.removedControlIDs {
            if let element = result.elements.first(where: { $0.id == id }), element.kind == .button,
               let builtin = element.defaultControlID {
                // Native element.delete keeps preset ownership/bindings and hides it.
                var paint = result.buttonCustomization(for: builtin); paint.isHidden = true
                result.setButtonCustomization(paint, for: builtin)
            } else {
                result.removeCustomButton(id: id)
                result.buttonCustomizations.removeValue(forKey: KeypadElementID(id))
                result.labelOverrides.removeValue(forKey: KeypadElementID(id))
            }
        }
        for edit in intent.controlEdits {
            guard let index = result.elements.firstIndex(where: { $0.id == edit.controlID }) else { throw Failure.missingControl }
            var paint = style(result.elements[index], in: result)
            if let value = edit.centerX { paint.centerX = CGFloat(value) }
            if let value = edit.centerY { paint.centerY = CGFloat(value) }
            if let value = edit.widthScale { paint.widthScale = CGFloat(value) }
            if let value = edit.heightScale { paint.heightScale = CGFloat(value) }
            if let value = edit.rotationDegrees { paint.rotationDegrees = CGFloat(value) }
            if let value = edit.zIndex { paint.zIndex = value }
            if let value = edit.isHidden { paint.isHidden = value }
            if let value = edit.isLocationLocked { paint.isLocationLocked = value }
            if result.elements[index].kind == .button, let builtin = result.elements[index].defaultControlID {
                result.setButtonCustomization(paint, for: builtin)
            } else {
                result.elements[index].layout = paint
                if let mirror = result.customButtons.firstIndex(where: { $0.id == edit.controlID }) { result.customButtons[mirror].layout = paint }
            }
        }
        if let scheme = intent.colorScheme, let value = GamepadColorSchemePreference(rawValue: scheme.rawValue) { result.colorSchemePreference = value }
        return result.normalized
    }
}
