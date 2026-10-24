// Workspace-only native evidence harness; compile in the same module as Sources/Shared.
// Usage: render-native-import PROFILE.json OUTPUT_DIRECTORY [HINTS_BY_CONTROL_ID.json]
// PROFILE must be anonymized. Hints must come from actual resolved binding formatter.
import AppKit
import Foundation
import SwiftUI

@main
struct RenderNativeImport {
    @MainActor
    static func main() throws {
        let args = CommandLine.arguments
        guard args.count == 3 || args.count == 4 else {
            throw NSError(domain: "RenderNativeImport", code: 1, userInfo: [NSLocalizedDescriptionKey:
                "Usage: render-native-import PROFILE.json OUTPUT_DIRECTORY [HINTS_BY_CONTROL_ID.json]"])
        }
        _ = NSApplication.shared
        let decoder = JSONDecoder()
        let input = try Data(contentsOf: URL(fileURLWithPath: args[1]))
        let profile: GamepadConfigurationProfile
        if String(decoding: input, as: UTF8.self).hasPrefix(ThumbleLinkSnapshot.prefix) {
            let base = GamepadConfigurationProfile(name: "Existing", customization: GamepadCustomization(elements: []))
            let configuration = try JSONEncoder().encode(ThumbleKeypadConfigurationExport(profiles: [base],
                activeProfileID: base.id, defaultProfileID: base.id))
            let plan = try ThumbleLinkImport.prepare(text: String(decoding: input, as: UTF8.self),
                currentConfiguration: configuration, layout: .controller)
            profile = try MacConfigurationBindings.decodeKeypadImport(data: plan.configurationData, sourceName: "Review").profiles[0]
        } else {
            profile = try decoder.decode(GamepadConfigurationProfile.self, from: input)
        }
        let hints: [String: String] = args.count == 4
            ? try decoder.decode([String: String].self, from: Data(contentsOf: URL(fileURLWithPath: args[3])))
            : Dictionary(uniqueKeysWithValues: profile.customization.elements.compactMap { element in
                element.output.flatMap(KeypadBindingFormatter.format).map { (element.inputID.rawValue, $0.compactText) }
            })
        let output = URL(fileURLWithPath: args[2], isDirectory: true)
        try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
        try JSONEncoder().encode(profile).write(to: output.appendingPathComponent("profile.json"))
        var items: [ThumbleNativeSkinPreviewItem] = []
        var layouts: [[String: Any]] = []
        for orientation in [GamepadEditorDeviceOrientation.portrait, .landscape] {
            for scheme in [ThumbleSkinColorScheme.light, .dark] {
                // Resolves profile-owned appearance banks without applying a skin package.
                let customization = profile.resolvedCustomization(for: orientation, colorScheme: scheme,
                    skinPackage: nil)
                let canvas = customization.deviceCanvas.editorDeviceFrame.screenRect.size
                let controls = customization.resolvedControls(in: canvas)
                let known = Set(controls.map { $0.id.id })
                let actualHints = hints.filter { known.contains($0.key) }
                layouts.append([
                    "orientation": orientation.rawValue, "scheme": scheme.rawValue,
                    "width": canvas.width, "height": canvas.height,
                    "safeAreaContract": "Reference canvas is already the authored safe-content viewport; zero explicit insets.",
                    "controls": controls.map { control -> [String: Any] in
                        ["id": control.id.id, "caption": control.visualLegend,
                         "role": control.visualRole.rawValue, "hidden": control.layoutCustomization.isHidden,
                         "centerX": control.center.x, "centerY": control.center.y,
                         "width": control.size.width, "height": control.size.height,
                         "hitX": control.hitFrame.minX, "hitY": control.hitFrame.minY,
                         "hitWidth": control.hitFrame.width, "hitHeight": control.hitFrame.height]
                    }
                ])
                for state in [GamepadControlPresentationState.normal, .pressed, .active, .disabled] {
                    let title = "\(orientation.rawValue)-\(scheme.rawValue)-\(state.rawValue)"
                    let item = ThumbleNativeSkinPreviewItem(title: title, customization: customization,
                        colorScheme: scheme, state: state, safeAreaInsets: .init(),
                        secondaryBindingTexts: actualHints)
                    try ThumbleNativeSkinPreviewRenderer.writePNG(item: item,
                        outputURL: output.appendingPathComponent(title + ".png"), scale: 2)
                    items.append(item)
                }
            }
        }
        try ThumbleNativeSkinPreviewRenderer.writeContactSheet(items: items,
            skinName: "Classic Hunter Link — native import review", outputURL: output.appendingPathComponent("contact-sheet.png"),
            columns: 4, scale: 1)
        try JSONSerialization.data(withJSONObject: layouts, options: [.prettyPrinted, .sortedKeys])
            .write(to: output.appendingPathComponent("native-layouts.json"), options: .atomic)
        print("Wrote \(items.count) native2x frames, contact-sheet.png and native-layouts.json to \(output.path)")
    }
}
