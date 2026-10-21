import Foundation
import SwiftUI
import XCTest

final class OwnedDesignTokenReplacementTests: XCTestCase {
    func testReviewedDesignReplacesCollidingTokensAndRetainsLiveIcons() throws {
        var source = GamepadCustomization.blankCanvas
        var oldVisual = GamepadControlVisualStyle.empty
        oldVisual.normal.content = .init(iconScale: 2.2)
        let old = GamepadStyleToken(id: "ability", name: "Previous", visualStyle: oldVisual)
        let unrelated = GamepadStyleToken(id: "unrelated", name: "Unrelated", visualStyle: .init(normal: .init(opacity: 0.7)))
        source.styleLibrary = .init(styles: [old, unrelated])
        let icon = GamepadControlIcon(source: .asset, value: "live-ability", scale: 2.2, renderingMode: .original)
        var button = source.buttonCustomization(for: .preset(5))
        button.icon = icon
        source.setButtonCustomization(button, for: .preset(5))
        var newVisual = GamepadControlVisualStyle.empty
        newVisual.normal.content = .init(iconScale: 2.7, iconMask: .circle)
        let replacement = GamepadStyleToken(id: "ability", name: "Circular", visualStyle: newVisual)
        let skin = ThumbleSkin(base: .init(styleLibrary: .init(styles: [replacement])))
        let package = ThumbleSkinPackage(manifest: .init(identifier: "com.thumble.test.circular", version: "1.0.0", name: "Circular", author: .init(name: "Tests")), skin: skin)
        let result = ThumbleOwnedOperationBridge.applyingReviewedAppearance(to: source, package: package, scheme: .dark, actionSet: .base)
        XCTAssertEqual(result.styleLibrary.styles.first { $0.id == "ability" }, replacement.normalized)
        XCTAssertEqual(result.styleLibrary.styles.first { $0.id == "unrelated" }, unrelated.normalized)
        XCTAssertEqual(result.buttonCustomization(for: .preset(5)).icon, icon)
        XCTAssertEqual(result.elements.map(\.output), source.normalized.elements.map(\.output))
    }

}
