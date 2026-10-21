import XCTest

final class VirtualGamepadOutputPolicyTests: XCTestCase {
    func testCameraAndClickSettingsRoundTripWithoutChangingOldTrackpads() throws {
        let old = Data(#"{"sensitivity":1.2,"scrollSensitivity":0.85,"tapToClick":true,"twoFingerScroll":true,"naturalScrolling":true}"#.utf8)
        let decoder = JSONDecoder()
        let original = try decoder.decode(GamepadTrackpadSettings.self, from: old)
        XCTAssertNil(original.clickButton)
        XCTAssertTrue(original.movesCursor)
        let click = GamepadTrackpadSettings(clickButton: .right, movesCursor: false)
        XCTAssertEqual(try decoder.decode(GamepadTrackpadSettings.self, from: JSONEncoder().encode(click)), click)
        let camera = GamepadJoystickOutputSettings(analogTarget: .mouseLook, sendsDigitalDirections: true).normalized
        XCTAssertFalse(camera.sendsDigitalDirections)
        XCTAssertNil(camera.analogTarget.stick)
        XCTAssertEqual(try decoder.decode(GamepadJoystickOutputSettings.self, from: JSONEncoder().encode(camera)), camera)
    }

    func testKeyboardModeGatesDirectOutputsTriggersAndAnalogControls() {
        var customization = GamepadCustomization.defaultValue
        customization.elements.append(KeypadElement(output: .init(gamepadButtons: [.south])))
        customization.customButtons.append(GamepadCustomButton(controlKind: .trigger))
        customization.customButtons.append(GamepadCustomButton(
            controlKind: .joystick,
            joystickOutputSettings: .init(analogTarget: .leftStick)
        ))
        XCTAssertFalse(VirtualGamepadOutputPolicy.needsDevice(
            outputMode: .keyboard, hasMappedGamepadButtons: true, customization: customization
        ))
        XCTAssertTrue(VirtualGamepadOutputPolicy.needsDevice(
            outputMode: .custom, hasMappedGamepadButtons: false, customization: customization
        ))
    }

    func testKeyboardModeFiltersDirectMixedBindingsBeforeTheyAreCaptured() {
        let direct = MacControlOutputBinding(shared: .init(
            keyboard: .init(keyCode: 49), gamepadButtons: [.south, .leftTriggerButton]
        ))
        let keyboardOnly = direct.filtered(for: .keyboard)
        XCTAssertEqual(keyboardOnly.keyboard, direct.keyboard)
        XCTAssertTrue(keyboardOnly.gamepadButtons.isEmpty)
        XCTAssertTrue(MacControlOutputBinding.gamepadButton(.south).filtered(for: .keyboard).isEmpty)
        XCTAssertEqual(direct.filtered(for: .custom), direct)
    }

    func testControllerModeExplicitlyRequestsDeviceEvenWithoutBindings() {
        XCTAssertTrue(VirtualGamepadOutputPolicy.needsDevice(
            outputMode: .controller, hasMappedGamepadButtons: false, customization: .defaultValue
        ))
    }
}
