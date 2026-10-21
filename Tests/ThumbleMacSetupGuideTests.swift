import Foundation
import XCTest

final class ThumbleMacSetupGuideTests: XCTestCase {
    func testPrerequisitesAppearBeforePairingAndEditor() {
        XCTAssertEqual(
            MacOnboardingStep.allCases,
            [.welcome, .iPhoneApp, .localNetwork, .permissions, .connect, .editor]
        )
        XCTAssertEqual(MacOnboardingStep.allCases.map(\.tag), [
            "01 / WELCOME", "02 / IPHONE APP", "03 / LOCAL NETWORK",
            "04 / ACCESSIBILITY", "05 / CONNECT", "06 / EDITOR"
        ])
    }

    func testAppStorePlaceholderHasNoNavigableURL() {
        XCTAssertNil(ThumbleMacSetupGuide.iOSAppStoreURL)
        XCTAssertTrue(ThumbleMacSetupGuide.iOSAppStorePlaceholder.contains("coming soon"))
        XCTAssertTrue(ThumbleMacSetupGuide.text.contains(ThumbleMacSetupGuide.iOSAppStorePlaceholder))
        XCTAssertFalse(ThumbleMacSetupGuide.text.contains("https://apps.apple.com"))
    }

    func testLocalNetworkInstructionsCoverBothDevicesAndDeniedAccess() {
        let instructions = MacOnboardingStep.localNetwork.instructions
        XCTAssertEqual(instructions.map(\.title), ["On your Mac", "On your iPhone", "Choose Allow when prompted"])
        XCTAssertTrue(instructions[0].text.contains("System Settings → Privacy & Security → Local Network"))
        XCTAssertTrue(instructions[0].text.contains("Thumble Mac"))
        XCTAssertTrue(instructions[0].text.contains("macOS 15 or later"))
        XCTAssertTrue(instructions[1].text.contains("Settings → Privacy & Security → Local Network → enable Thumble"))
        XCTAssertTrue(instructions[2].text.contains("Don’t Allow"))
        XCTAssertTrue(ThumbleMacSetupGuide.localNetworkAvailabilityNote.contains("macOS 14 has no Local Network switch"))
    }

    func testSettingsDestinationFallsBackOnMacOS14() {
        XCTAssertEqual(
            ThumbleMacSetupGuide.localNetworkSettingsURL(macOSMajorVersion: 14).absoluteString,
            "x-apple.systempreferences:com.apple.preference.security?Privacy"
        )
        for version in [15, 26] {
            XCTAssertEqual(
                ThumbleMacSetupGuide.localNetworkSettingsURL(macOSMajorVersion: version).absoluteString,
                "x-apple.systempreferences:com.apple.preference.security?Privacy_LocalNetwork"
            )
        }
    }

    func testCLIGuideUsesHeadlessSetupAndRetainsPhoneAndPermissionActions() {
        let text = ThumbleMacSetupGuide.text
        for command in ["server start", "status --json", "server addresses", "app ios-app", "app local-network-settings", "accessibility status", "accessibility open", "pairing payload", "profile list", "layout validate PROFILE --json", "layout preview PROFILE -o preview.png", "app setup-guide"] {
            XCTAssertTrue(text.contains("thumble \(command)"), command)
        }
        for retired in ["app open", "app screenshot", "app replay-onboarding", "app editor-view", "Mac toolbar"] {
            XCTAssertFalse(text.contains(retired), retired)
        }
        XCTAssertTrue(text.contains("private connection material"))
    }

    func testInstructionIdentifiersAreUniqueWithinEachSlide() {
        for step in MacOnboardingStep.allCases {
            XCTAssertEqual(Set(step.instructions.map(\.id)).count, step.instructions.count)
        }
    }
}
