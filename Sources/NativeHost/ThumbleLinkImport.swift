import Foundation

/// Shared by the Mac sheet and CLI. Builds an ordinary authority import; never writes state itself.
enum ThumbleLinkImport {
    enum Layout: String, CaseIterable, Identifiable, Sendable {
        case controller, grid
        var id: String { rawValue }
        var title: String { self == .controller ? "Controller" : "Button grid" }
    }

    struct Row: Identifiable, Sendable {
        let command: String
        let label: String
        let key: String?
        let issue: String?
        var note: String? = nil
        var id: String { command }
    }

    struct Plan: Sendable {
        let snapshot: ThumbleLinkSnapshot
        let profileName: String
        let isRefresh: Bool
        let rows: [Row]
        let warnings: [String]
        let configurationData: Data
        let baseConfiguration: Data
        let layout: Layout

        func requireUnchangedConfiguration(_ data: Data) throws {
            let before = try MacConfigurationBindings.decodeKeypadImport(data: baseConfiguration, sourceName: "Before")
            let after = try MacConfigurationBindings.decodeKeypadImport(data: data, sourceName: "After")
            guard before.profiles == after.profiles, before.profileSources == after.profileSources,
                  before.profileKeyBindings == after.profileKeyBindings,
                  before.profileOutputBindings == after.profileOutputBindings else {
                throw ThumbleLinkError("Your setups changed after review. Review this export again before importing.")
            }
        }
    }

    private struct LinkMetadata: Codable {
        let snapshot: ThumbleLinkSnapshot
        /// Retain past commands so deleted phone controls stay deleted across refreshes.
        let managedLabels: [String: String]
        let importedAt: Int64
        let layout: String?
    }

    static func prepare(text: String, currentConfiguration: Data, name: String? = nil,
                        layout requestedLayout: Layout? = nil) throws -> Plan {
        let snapshot = try ThumbleLinkSnapshot.parse(text)
        let current = try MacConfigurationBindings.decodeKeypadImport(data: currentConfiguration, sourceName: "Current Setups")
        let existing = current.profiles.first { $0.id == snapshot.profileID }
        let previous = try metadata(for: existing, sources: current.profileSources)
        if let previous, previous.snapshot.character.id != snapshot.character.id {
            throw ThumbleLinkError("This setup belongs to a different WoW character.")
        }
        let profileName = name?.trimmingCharacters(in: .whitespacesAndNewlines) ?? existing?.name ?? snapshot.suggestedName
        guard !profileName.isEmpty, profileName.count <= 128,
              !profileName.unicodeScalars.contains(where: { CharacterSet.controlCharacters.contains($0) }),
              !current.profiles.contains(where: { $0.id != snapshot.profileID && $0.name.caseInsensitiveCompare(profileName) == .orderedSame }) else {
            throw ThumbleLinkError("Choose a unique setup name of 1–128 characters. An unrelated setup already using this name will not be replaced.")
        }
        let rows = snapshot.actions.map(resolve)
        guard existing != nil || rows.contains(where: { $0.key != nil }) else {
            throw ThumbleLinkError("No supported keyboard bindings were found. Bind an action to a keyboard key in WoW, then export again.")
        }
        var profile = existing ?? GamepadConfigurationProfile(id: snapshot.profileID, name: profileName,
            customization: GamepadCustomization(elements: []))
        if existing != nil {
            let raw = current.profileOutputBindings[profile.id.uuidString] ?? [:]
            let outputs = Dictionary(uniqueKeysWithValues: raw.compactMap { key, value in
                KeypadElementID(rawValue: key).map { ($0, value) }
            })
            MacConfigurationBindings.synchronizeElementOutputs(in: &profile, outputs: outputs)
        }
        profile.name = profileName
        profile.outputMode = .keyboard
        let oldLabels = previous?.managedLabels ?? [:]
        var warnings = snapshot.warnings
        let supportedCount = rows.filter { $0.key != nil }.count
        if requestedLayout == .controller && supportedCount > ThumbleLinkControllerLayout.capacity {
            throw ThumbleLinkError("Controller layout supports up to 24 bound commands. Export fewer bars or choose Button grid.")
        }
        let layout = requestedLayout ?? (previous?.layout).flatMap(Layout.init(rawValue:))
            ?? (existing == nil && supportedCount <= ThumbleLinkControllerLayout.capacity ? .controller : .grid)
        guard layout != .controller || supportedCount <= ThumbleLinkControllerLayout.capacity else {
            throw ThumbleLinkError("This export has more than 24 bound commands. Export fewer bars or choose Button grid to rebuild the layout.")
        }
        if existing == nil && layout == .grid && requestedLayout == nil {
            warnings.append("More than 24 commands are bound. Button grid keeps every binding; export fewer bars to use Controller layout.")
        }
        let rebuildLayout = requestedLayout != nil || existing == nil
        if requestedLayout != nil && existing != nil {
            warnings.append("The selected layout rebuilds imported control positions and appearance. Your keyboard bindings, labels, and deleted controls are preserved.")
        }
        warnings.append("Snapshot only: re-export after changing WoW bindings, bars, forms, or characters. Ability pictures and cooldowns are not synchronized in version 1.")
        warnings.append("Keyboard output uses Mac physical key positions based on a US layout. Test punctuation on your keyboard layout.")
        if existing != nil { warnings.append("Refresh preserves positions, styling, deleted controls, and manually edited labels or outputs. Unchanged imported outputs are refreshed; missing bindings are cleared.") }
        try update(&profile.customization, snapshot: snapshot, rows: rows, previousLabels: oldLabels,
                   portrait: false, layout: layout, rebuildLayout: rebuildLayout)
        if var landscape = profile.landscapeCustomization {
            try update(&landscape, snapshot: snapshot, rows: rows, previousLabels: oldLabels,
                       portrait: false, layout: layout, rebuildLayout: rebuildLayout)
            profile.landscapeCustomization = landscape
        }
        if var portrait = profile.portraitCustomization {
            try update(&portrait, snapshot: snapshot, rows: rows, previousLabels: oldLabels,
                       portrait: true, layout: layout, rebuildLayout: rebuildLayout)
            profile.portraitCustomization = portrait
        } else if existing == nil {
            var portrait = GamepadCustomization(deviceCanvas: GamepadDeviceCanvas(frameID: "iphone-17-pro-portrait"), elements: [])
            try update(&portrait, snapshot: snapshot, rows: rows, previousLabels: oldLabels,
                       portrait: true, layout: layout, rebuildLayout: rebuildLayout)
            profile.portraitCustomization = portrait
        }
        profile.updatedAt = Date.currentMilliseconds
        var labels = oldLabels
        let boundCommands = Set(rows.filter { $0.key != nil }.map(\.command))
        for action in snapshot.actions where boundCommands.contains(action.command) || oldLabels[action.command] != nil {
            labels[action.command] = action.label
        }
        var source: ThumbleBridgeJSONValue
        if let existingSource = current.profileSources[profile.id] { source = existingSource }
        else { source = try JSONDecoder().decode(ThumbleBridgeJSONValue.self, from: JSONEncoder().encode(profile)) }
        guard case .object(var fields) = source else { throw ThumbleLinkError("The existing setup metadata could not be read.") }
        fields["thumbleLink"] = try JSONDecoder().decode(ThumbleBridgeJSONValue.self,
            from: JSONEncoder().encode(LinkMetadata(snapshot: snapshot, managedLabels: labels,
                importedAt: Date.currentMilliseconds, layout: layout.rawValue)))
        source = .object(fields)
        let outputs = profile.initialMacOutputBindings
        let envelope = MacConfigurationBindings.KeypadExportEnvelope(profiles: [profile], activeProfileID: profile.id,
            defaultProfileID: nil,
            profileKeyBindings: [profile.id.uuidString: MacConfigurationBindings.rawKeyBindings(outputs.keyboardBindings)],
            profileOutputBindings: [profile.id.uuidString: MacConfigurationBindings.rawOutputs(outputs)],
            preserving: [profile.id: source])
        let data = try JSONEncoder().encode(envelope)
        // Run the same import boundary before either caller can mutate authority state.
        _ = try MacConfigurationBindings.decodeKeypadImport(data: data, sourceName: profileName)
        var reviewedRows = rows
        for index in reviewedRows.indices {
            let row = reviewedRows[index]
            let id = snapshot.elementID(for: row.command)
            if let element = profile.customization.elements.first(where: { $0.id == id }) {
                if element.output != element.defaultOutput {
                    let actual = element.output.flatMap(KeypadBindingFormatter.format)?.compactText ?? "Unmapped"
                    reviewedRows[index].note = "Keeping your phone output: \(actual)"
                }
            } else if oldLabels[row.command] != nil {
                reviewedRows[index].note = "Keeping this phone control deleted"
            }
        }
        return Plan(snapshot: snapshot, profileName: profileName, isRefresh: existing != nil,
                    rows: reviewedRows, warnings: warnings, configurationData: data,
                    baseConfiguration: currentConfiguration, layout: layout)
    }

    private static func metadata(for profile: GamepadConfigurationProfile?, sources: [UUID: ThumbleBridgeJSONValue]) throws -> LinkMetadata? {
        guard let profile else { return nil }
        guard case .object(let fields) = sources[profile.id], let value = fields["thumbleLink"],
              let metadata = try? JSONDecoder().decode(LinkMetadata.self, from: JSONEncoder().encode(value)),
              metadata.snapshot.profileID == profile.id else {
            throw ThumbleLinkError("The matching setup has no valid Thumble Link metadata. It will not be overwritten.")
        }
        return metadata
    }

    private static func resolve(_ action: ThumbleLinkSnapshot.Action) -> Row {
        if let reason = action.unavailableReason { return Row(command: action.command, label: action.label, key: nil, issue: reason) }
        if let key = action.keys.first(where: { ThumbleLinkKeyBinding.resolve($0) != nil }) {
            return Row(command: action.command, label: action.label, key: key, issue: nil)
        }
        return Row(command: action.command, label: action.label, key: nil,
                   issue: action.keys.isEmpty ? "Unbound in WoW" : "Unsupported keys: \(action.keys.joined(separator: ", "))")
    }

    private static func update(_ customization: inout GamepadCustomization, snapshot: ThumbleLinkSnapshot,
                               rows: [Row], previousLabels: [String: String], portrait: Bool,
                               layout: Layout, rebuildLayout: Bool) throws {
        var result = customization.normalized
        let actions = Dictionary(uniqueKeysWithValues: snapshot.actions.map { ($0.command, $0) })
        let resolved = Dictionary(uniqueKeysWithValues: rows.map { ($0.command, $0) })
        let managed = Set(previousLabels.keys).union(actions.keys)
        let commandsByID = Dictionary(uniqueKeysWithValues: managed.map { (snapshot.elementID(for: $0), $0) })
        var present = Set<UUID>()
        for index in result.elements.indices {
            let id = result.elements[index].id
            present.insert(id)
            guard let command = commandsByID[id] else { continue }
            let row = resolved[command]
            let output = KeypadElementOutputBinding(keyboard: row?.key.flatMap(ThumbleLinkKeyBinding.resolve))
            // The default is the last imported baseline, so explicit user edits win.
            if result.elements[index].output == result.elements[index].defaultOutput {
                result.elements[index].output = output
            }
            result.elements[index].defaultOutput = output
            if let old = previousLabels[command],
               result.elements[index].label == String(old.trimmingCharacters(in: .whitespacesAndNewlines).prefix(GamepadCustomization.maximumLabelLength)),
               let label = row?.label {
                result.elements[index].label = label
                result.elements[index].presentation = layout == .controller
                    ? ThumbleLinkControllerLayout.presentation(command: command, label: label)
                    : GamepadControlPresentation(actionID: "wow.\(command.lowercased())",
                        caption: label, accessibilityName: label)
            }
        }
        let additions = rows.filter { $0.key != nil && !present.contains(snapshot.elementID(for: $0.command)) && previousLabels[$0.command] == nil }
        guard result.elements.count + additions.count <= GamepadCustomization.maximumCustomButtons else {
            throw ThumbleLinkError("This refresh would exceed the setup’s 128-control limit. Export fewer action bars.")
        }
        let total = max(1, result.elements.count + additions.count)
        let columns = portrait ? min(4, total) : min(8, total)
        let rowCount = (total + columns - 1) / columns
        for row in additions {
            let index = result.elements.count
            let output = KeypadElementOutputBinding(keyboard: row.key.flatMap(ThumbleLinkKeyBinding.resolve))
            let layout = GamepadButtonCustomization(centerX: 0.08 + 0.84 * (CGFloat(index % columns) + 0.5) / CGFloat(columns),
                centerY: 0.16 + 0.76 * (CGFloat(index / columns) + 0.5) / CGFloat(rowCount),
                widthScale: portrait ? 0.9 : 1.0, heightScale: 0.8, shape: .roundedRectangle)
            result.elements.append(KeypadElement(id: snapshot.elementID(for: row.command), label: row.label,
                layout: layout, output: output, defaultOutput: output,
                presentation: GamepadControlPresentation(actionID: "wow.\(row.command.lowercased())",
                    caption: row.label, accessibilityName: row.label)))
        }
        let newIDs = Set(additions.map { snapshot.elementID(for: $0.command) })
        if layout == .controller {
            try ThumbleLinkControllerLayout.apply(to: &result, snapshot: snapshot, rows: rows,
                portrait: portrait, restyleIDs: rebuildLayout ? Set(commandsByID.keys) : newIDs,
                rebuildCanvas: rebuildLayout)
        } else if rebuildLayout {
            let imported = result.elements.indices.filter { commandsByID[result.elements[$0].id] != nil }
            let count = max(1, imported.count)
            let gridColumns = portrait ? min(4, count) : min(8, count)
            let gridRows = (count + gridColumns - 1) / gridColumns
            for (position, index) in imported.enumerated() {
                result.elements[index].layout = GamepadButtonCustomization(
                    centerX: 0.08 + 0.84 * (CGFloat(position % gridColumns) + 0.5) / CGFloat(gridColumns),
                    centerY: 0.16 + 0.76 * (CGFloat(position / gridColumns) + 0.5) / CGFloat(gridRows),
                    widthScale: portrait ? 0.9 : 1, heightScale: 0.8, shape: .roundedRectangle)
                result.elements[index].visualRole = nil
            }
        }
        // Sync legacy appearance mirrors from canonical elements, preserving unrelated controls.
        let importedIDs = Set(commandsByID.keys)
        result.customButtons.removeAll { importedIDs.contains($0.id) }
        for id in importedIDs { result.labelOverrides.removeValue(forKey: KeypadElementID(id)); result.buttonCustomizations.removeValue(forKey: KeypadElementID(id)) }
        result.updatedAt = Date.currentMilliseconds
        customization = result.normalized
    }
}

/// A native keyboard controller, independent of any skin's fixed control UUIDs.
/// Placement never chooses an output: each control keeps its imported binding.
private enum ThumbleLinkControllerLayout {
    static let capacity = 24

    private struct Slot {
        let x: CGFloat
        let y: CGFloat
        let width: CGFloat
        let height: CGFloat
        let role: GamepadVisualRole
    }

    static func apply(to customization: inout GamepadCustomization, snapshot: ThumbleLinkSnapshot,
                      rows: [ThumbleLinkImport.Row], portrait: Bool, restyleIDs: Set<UUID>, rebuildCanvas: Bool) throws {
        guard !restyleIDs.isEmpty else { return }
        let assignments = assignedSlots(snapshot: snapshot, rows: rows)
        let slots = geometry(portrait: portrait, compact: rows.filter { $0.key != nil }.count > 16)
        let canvas = CGSize(width: portrait ? 402 : 874, height: portrait ? 874 : 402)
        let base: CGFloat = portrait ? 80.4 : 86
        var occupied = customization.resolvedControls(in: canvas).filter { control in
            control.elementID.map { !restyleIDs.contains($0) } ?? true
        }.map(\.hitFrame)
        for index in customization.elements.indices {
            let id = customization.elements[index].id
            guard restyleIDs.contains(id), let assignment = assignments[id] else { continue }
            let candidates = [assignment.slot] + slots.indices.filter { $0 != assignment.slot }
            guard let selected = candidates.first(where: { candidate in
                let slot = slots[candidate]
                let frame = CGRect(x: slot.x - slot.width / 2, y: slot.y - slot.height / 2,
                    width: slot.width, height: slot.height)
                return !occupied.contains { frame.insetBy(dx: -4, dy: -4).intersects($0) }
            }) else {
                throw ThumbleLinkError("There is no free controller position for a new binding. Choose Controller to rebuild positions, or export fewer commands.")
            }
            let slot = slots[selected]
            occupied.append(CGRect(x: slot.x - slot.width / 2, y: slot.y - slot.height / 2,
                width: slot.width, height: slot.height))
            let label = customization.elements[index].label
            let importedLabel = String(assignment.row.label.trimmingCharacters(in: .whitespacesAndNewlines)
                .prefix(GamepadCustomization.maximumLabelLength))
            customization.elements[index].layout = GamepadButtonCustomization(
                centerX: slot.x / canvas.width, centerY: slot.y / canvas.height,
                widthScale: slot.width / base, heightScale: slot.height / base,
                hitInsets: .all(0), shape: .roundedRectangle,
                lightFillColor: color("#F7F2E7"), darkFillColor: color("#253638"),
                visualStyle: style(role: assignment.role), cornerRadius: 8, shadowStrength: 0.2)
            customization.elements[index].visualRole = assignment.role
            customization.elements[index].presentation = presentation(command: assignment.row.command,
                label: (label == importedLabel || label == assignment.row.label) ? assignment.row.label : label,
                usesShortCaption: label == importedLabel || label == assignment.row.label)
        }
        if rebuildCanvas {
            customization.deviceCanvas = GamepadDeviceCanvas(frameID: portrait ? "iphone-17-pro-portrait" : "iphone-17-pro-landscape")
            customization.controlScale = .standard
            customization.accentStyle = .monochrome
            customization.showsButtonLabels = true
            customization.backgroundFillStyle = nil
            customization.backgroundLightFillStyle = nil
            customization.backgroundDarkFillStyle = nil
            customization.backgroundLightColor = color("#E8E4D8")
            customization.backgroundDarkColor = color("#111A1C")
            customization.topBarActivationRegion.lightFillColor = color("#F7F2E7")
            customization.topBarActivationRegion.darkFillColor = color("#253638")
            customization.topBarActivationRegion.visualStyle = style(role: .menu)
        }
    }

    static func presentation(command: String, label: String, usesShortCaption: Bool = true) -> GamepadControlPresentation {
        let short = ["MOVEFORWARD": "Up", "MOVEBACKWARD": "Back", "STRAFELEFT": "Strafe L",
                     "STRAFERIGHT": "Strafe R", "TARGETNEARESTENEMY": "Target",
                     "TARGETPREVIOUSENEMY": "Previous", "TARGETSELF": "Self", "TOGGLEWORLDMAP": "Map"]
        return GamepadControlPresentation(actionID: "wow.\(command.lowercased())",
            legend: (usesShortCaption ? short[command] : nil) ?? String(label.prefix(64)), accessibilityName: label)
    }

    private struct Assignment {
        let slot: Int
        let row: ThumbleLinkImport.Row
        let role: GamepadVisualRole
    }

    private static func assignedSlots(snapshot: ThumbleLinkSnapshot, rows: [ThumbleLinkImport.Row]) -> [UUID: Assignment] {
        let fixed = ["MOVEFORWARD": 0, "MOVEBACKWARD": 1, "STRAFELEFT": 2, "STRAFERIGHT": 3,
                     "JUMP": 7, "TOGGLEWORLDMAP": 10, "OPENALLBAGS": 11, "TOGGLECHARACTER0": 12,
                     "TARGETNEARESTENEMY": 13, "TARGETPREVIOUSENEMY": 14, "TARGETSELF": 15]
        let kinds = Dictionary(uniqueKeysWithValues: snapshot.actions.map { ($0.command, $0.kind) })
        var occupied = Set<Int>()
        var assignments: [UUID: Assignment] = [:]
        let bound = rows.filter { $0.key != nil }
        func assign(_ row: ThumbleLinkImport.Row, slot: Int) {
            occupied.insert(slot)
            let role: GamepadVisualRole
            if let position = fixed[row.command] {
                role = position < 4 ? .movement : position == 7 ? .secondaryAction : position < 13 ? .menu : .utility
            } else { role = kinds[row.command] == "item" ? .secondaryAction : .primaryAction }
            assignments[snapshot.elementID(for: row.command)] = Assignment(slot: slot, row: row, role: role)
        }
        for row in bound {
            if let slot = fixed[row.command] { assign(row, slot: slot) }
        }
        for row in bound where fixed[row.command] == nil {
            let preferred = kinds[row.command] == "item" ? [8, 9, 4, 5, 6, 7] : [4, 5, 6, 7, 8, 9]
            if let slot = (preferred + [10, 11, 12, 13, 14, 15, 0, 1, 2, 3] + Array(16..<capacity)).first(where: { !occupied.contains($0) }) {
                assign(row, slot: slot)
            }
        }
        return assignments
    }

    private static func geometry(portrait: Bool, compact: Bool) -> [Slot] {
        if portrait {
            var slots: [Slot] = [
                Slot(x: 106, y: 446, width: 52, height: 52, role: .movement),
                Slot(x: 106, y: 566, width: 52, height: 52, role: .movement),
                Slot(x: 46, y: 506, width: 52, height: 52, role: .movement),
                Slot(x: 166, y: 506, width: 52, height: 52, role: .movement),
                Slot(x: 254, y: 466, width: 76, height: 76, role: .primaryAction),
                Slot(x: 338, y: 466, width: 76, height: 76, role: .primaryAction),
                Slot(x: 254, y: 550, width: 76, height: 76, role: .primaryAction),
                Slot(x: 338, y: 550, width: 76, height: 76, role: .secondaryAction),
                Slot(x: 115, y: compact ? 662 : 700, width: 144, height: 60, role: .secondaryAction),
                Slot(x: 287, y: compact ? 662 : 700, width: 144, height: 60, role: .secondaryAction)
            ]
            for x: CGFloat in [85, 201, 317] {
                slots.append(Slot(x: x, y: 238, width: 108, height: 52, role: .menu))
            }
            for x: CGFloat in [85, 201, 317] {
                slots.append(Slot(x: x, y: 298, width: 108, height: 52, role: .utility))
            }
            for index in 0..<8 {
                slots.append(Slot(x: 58 + CGFloat(index % 4) * 94, y: index < 4 ? 734 : 804,
                    width: 82, height: 52, role: .primaryAction))
            }
            return slots
        }
        var slots: [Slot] = [
            Slot(x: 184, y: 174, width: 72, height: 64, role: .movement),
            Slot(x: 184, y: 334, width: 72, height: 64, role: .movement),
            Slot(x: 104, y: 254, width: 72, height: 64, role: .movement),
            Slot(x: 264, y: 254, width: 72, height: 64, role: .movement),
            Slot(x: 638, y: 210, width: 104, height: 72, role: .primaryAction),
            Slot(x: 750, y: 210, width: 104, height: 72, role: .primaryAction),
            Slot(x: 638, y: 298, width: 104, height: 72, role: .primaryAction),
            Slot(x: 750, y: 298, width: 104, height: 72, role: .secondaryAction),
            Slot(x: 437, y: 214, width: 116, height: 64, role: .secondaryAction),
            Slot(x: 437, y: 294, width: 116, height: 64, role: .secondaryAction)
        ]
        for x: CGFloat in [103, 191, 279] {
            slots.append(Slot(x: x, y: 98, width: 80, height: 48, role: .menu))
        }
        for x: CGFloat in [595, 683, 771] {
            slots.append(Slot(x: x, y: 98, width: 80, height: 48, role: .utility))
        }
        for index in 0..<8 {
            slots.append(Slot(x: 300 + CGFloat(index) * 70, y: 374,
                width: 60, height: 44, role: .primaryAction))
        }
        return slots
    }

    private static func style(role: GamepadVisualRole) -> GamepadControlVisualStyle {
        let edge = color(role == .movement ? "#668464" : "#94743F")
        return GamepadControlVisualStyle(
            normal: GamepadControlStateStyle(strokeColor: edge, strokeWidth: 1.3,
                shadowColor: color("#000000", alpha: 0.22), shadowRadius: 2, shadowY: 2,
                scale: 1, content: GamepadControlContentStyle(fontSize: 13, fontWeight: .semibold,
                    fontDesign: .rounded, lineLimit: 2, alignment: .center, labelPadding: 5)),
            pressed: GamepadControlStateStyle(strokeColor: edge, strokeWidth: 2.5,
                shadowRadius: 0, innerShadowColor: color("#000000", alpha: 0.3),
                innerShadowRadius: 3, innerShadowY: 2, scale: 1),
            active: GamepadControlStateStyle(strokeColor: edge, strokeWidth: 3.5, shadowRadius: 0, scale: 1),
            disabled: GamepadControlStateStyle(strokeColor: color("#737873"), strokeWidth: 1, shadowRadius: 0, opacity: 0.8, scale: 1),
            hapticStyle: .light)
    }

    private static func color(_ hex: String, alpha: CGFloat = 1) -> GamepadRGBAColor {
        GamepadRGBAColor(hexString: hex, alpha: alpha) ?? .defaultValue
    }
}
