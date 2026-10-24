import AppKit
import Foundation
import SwiftUI

@main
struct BuildHunterWarband {
    @MainActor static func main() throws {
        _ = NSApplication.shared
        let root = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
        let review = CommandLine.arguments.count > 2 ? CommandLine.arguments[2] : "native-review-1"
        let encoder = JSONEncoder(); encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        let decoder = JSONDecoder()
        let profileURL = root.appendingPathComponent("preview/profile.json")
        let profile = try decoder.decode(GamepadConfigurationProfile.self, from: Data(contentsOf: profileURL))
        let sourceURL = root.appendingPathComponent("skin-source.json")
        var source = try decoder.decode(ThumbleSkinWorkspace.self, from: Data(contentsOf: sourceURL))
        let artboard = try ThumbleSkinArtboard.capture(profile: profile, identifier: "classic-hunter-warband-captured-v1",
            safeAreas: [.portrait: .init(), .landscape: .init()])
        source.artboardID = artboard.id; source.capturedArtboards = [artboard]
        // Map full-canvas framing to the exact surviving native command union.
        // The source SVG remains at its original Warband point coordinates.
        for i in source.sourceAssets.indices where source.sourceAssets[i].purpose == .canvasArtwork {
            let orientation = source.sourceAssets[i].orientation!
            let variant = artboard.variants.first { $0.orientation == orientation }!
            let left = variant.controls.map { $0.frame.x }.min()!
            let top = variant.controls.map { $0.frame.y }.min()!
            let right = variant.controls.map { $0.frame.x + $0.frame.width }.max()!
            let bottom = variant.controls.map { $0.frame.y + $0.frame.height }.max()!
            let w = right-left, h = bottom-top
            let inset: CGFloat = 0.0000001
            source.sourceAssets[i].anchor = .init(group: "command", scaleX: (1-2*inset)/w, scaleY: (1-2*inset)/h,
                offsetX: (0.5-left)/w-0.5, offsetY: (0.5-top)/h-0.5)
        }
        try encoder.encode(source).write(to: sourceURL, options: .atomic)
        try encoder.encode(artboard).write(to: root.appendingPathComponent("preview/captured-artboard.json"), options: .atomic)
        let result = try ThumbleSkinCompiler.compile(source: root,
            buildDirectory: root.appendingPathComponent("build/compiled"),
            packageOutputURL: root.appendingPathComponent("dist/classic-hunter-warband-1.0.0.pocketpad"), clean: true, strict: true)
        let quality = ThumbleSkinQualityEvaluator.evaluate(package: result.package, workspace: result.workspace,
            capturedProfile: profile)
        let directory = root.appendingPathComponent("reviews/" + review)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try encoder.encode(quality).write(to: directory.appendingPathComponent("quality.json"))
        var items: [ThumbleNativeSkinPreviewItem] = []
        var layouts: [[String: Any]] = []
        for orientation in [GamepadEditorDeviceOrientation.portrait, .landscape] {
            for scheme in [ThumbleSkinColorScheme.light, .dark] {
                let rendered = profile.customization(for: orientation).applying(skinPackage: result.package,
                    orientation: orientation == .portrait ? .portrait : .landscape, colorScheme: scheme)
                let size = rendered.deviceCanvas.editorDeviceFrame.screenRect.size
                layouts.append(["orientation": orientation.rawValue, "scheme": scheme.rawValue,
                    "controls": rendered.resolvedControls(in: size).map { c -> [String: Any] in
                        ["id": c.id.id, "label": c.label, "centerX": c.center.x, "centerY": c.center.y,
                         "width": c.size.width, "height": c.size.height, "hitX": c.hitFrame.minX,
                         "hitY": c.hitFrame.minY, "hitWidth": c.hitFrame.width, "hitHeight": c.hitFrame.height]
                    }])
                for state in [GamepadControlPresentationState.normal, .pressed, .active, .disabled] {
                    let name = "\(orientation.rawValue)-\(scheme.rawValue)-\(state.rawValue)"
                    let item = ThumbleNativeSkinPreviewItem(title: name, customization: rendered,
                        colorScheme: scheme, state: state)
                    try ThumbleNativeSkinPreviewRenderer.writePNG(item: item, outputURL: directory.appendingPathComponent(name + ".png"), scale: 2)
                    items.append(item)
                }
            }
        }
        try ThumbleNativeSkinPreviewRenderer.writeContactSheet(items: items, skinName: "Classic Hunter Warband",
            outputURL: directory.appendingPathComponent("contact-sheet.png"), columns: 4)
        try JSONSerialization.data(withJSONObject: layouts, options: [.prettyPrinted, .sortedKeys])
            .write(to: directory.appendingPathComponent("native-layouts.json"))
        var installed = profile
        installed.applySkin(result.package)
        installed.customization = installed.customization.dehydratingAssets(from: result.package)
        installed.landscapeCustomization = installed.landscapeCustomization?.dehydratingAssets(from: result.package)
        installed.portraitCustomization = installed.portraitCustomization?.dehydratingAssets(from: result.package)
        installed.skinBaselineCustomization = installed.skinBaselineCustomization?.dehydratingAssets(from: result.package)
        installed.landscapeSkinBaselineCustomization = installed.landscapeSkinBaselineCustomization?.dehydratingAssets(from: result.package)
        installed.portraitSkinBaselineCustomization = installed.portraitSkinBaselineCustomization?.dehydratingAssets(from: result.package)
        try encoder.encode(installed).write(to: root.appendingPathComponent("preview/install-profile.json"), options: .atomic)
        print("Package bytes: \(result.packageData.count); quality issues: \(quality.issues.count)")
        for issue in quality.issues { print("\(issue.severity.rawValue) \(issue.code): \(issue.message)") }
        print("Contact sheet: \(directory.appendingPathComponent("contact-sheet.png").path)")
    }
}
