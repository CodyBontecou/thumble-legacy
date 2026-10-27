import Foundation
import UIKit
import XCTest

private enum AnalogAppHTTP {
  static func request(port: Int, path: String) async throws -> Data {
    var request = URLRequest(url: URL(string: "http://127.0.0.1:\(port)\(path)")!)
    request.timeoutInterval = 6
    let (data, response) = try await URLSession.shared.data(for: request)
    guard (response as? HTTPURLResponse)?.statusCode == 200, data.count <= 16_384 else {
      throw NSError(domain: "saved_app_http", code: 1)
    }
    return data
  }
  static func check(port: Int, path: String) async throws {
    let value =
      try JSONSerialization.jsonObject(with: await request(port: port, path: path))
      as? [String: Any]
    guard value?["ok"] as? Bool == true else { throw NSError(domain: "saved_app_http", code: 2) }
  }
}

/// Actual React route and production native map admission, using the app's real Keychain.
/// Raw UI logs/xcresult are private because typing the synthetic invitation can be recorded.
@MainActor final class SavedAnalogUITests: XCTestCase {
  func testActualReactSavedAnalogMultiTouchAndDismissal() async throws {
    let url = try XCTUnwrap(
      Bundle(for: Self.self).url(
        forResource: "SavedControllerAppProbeConfig", withExtension: "json"))
    let config = try XCTUnwrap(
      JSONSerialization.jsonObject(with: Data(contentsOf: url)) as? [String: Any])
    let port = try XCTUnwrap(config["port"] as? Int)
    let mode = try XCTUnwrap(config["mode"] as? String)
    XCTAssertTrue((1...65535).contains(port) && ["analog", "analog-unsupported"].contains(mode))
    let app = XCUIApplication()
    XCUIDevice.shared.orientation = .landscapeLeft
    app.launch()
    defer {
      app.terminate()
      XCUIDevice.shared.orientation = .portrait
    }
    let home = app.descendants(matching: .any)["home-connect"].firstMatch
    XCTAssertTrue(home.waitForExistence(timeout: 20))
    home.tap()
    let invitation = try await AnalogAppHTTP.request(port: port, path: "/begin")
    let text = try XCTUnwrap(String(data: invitation, encoding: .utf8))
    let field = app.secureTextFields["pairing-invitation"]
    XCTAssertTrue(field.waitForExistence(timeout: 10))
    let enabled = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "enabled == true"), object: field)
    XCTAssertEqual(XCTWaiter.wait(for: [enabled], timeout: 10), .completed)
    // Exercise the product's intended native Paste flow instead of spending the
    // test budget switching software-keyboard layouts for each JSON character.
    field.tap()
    guard app.keyboards.firstMatch.waitForExistence(timeout: 5) else {
      XCTFail("Native invitation keyboard did not become ready")
      throw NSError(domain: "saved_app_ui", code: 5)
    }
    UIPasteboard.general.setItems(
      [[UIPasteboard.typeAutomatic: text]],
      options: [.localOnly: true, .expirationDate: Date().addingTimeInterval(60)])
    defer { if UIPasteboard.general.string == text { UIPasteboard.general.items = [] } }
    field.typeKey("v", modifierFlags: .command)
    // Paste completion is asynchronous. Returning early can discard pending text.
    // Compare only length; never put invitation text in an assertion or predicate.
    let pasted = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "value.length == %d", text.utf16.count), object: field)
    guard await XCTWaiter.fulfillment(of: [pasted], timeout: 5) == .completed else {
      XCTFail("Native Paste did not complete before Done")
      throw NSError(domain: "saved_app_ui", code: 4)
    }
    field.typeText("\n")
    let pair = app.alerts.buttons["Pair"]
    guard pair.waitForExistence(timeout: 10) else {
      let hierarchy = XCTAttachment(string: app.debugDescription)
      hierarchy.name = "private-post-keyboard-hierarchy"
      hierarchy.lifetime = .keepAlways
      add(hierarchy)
      XCTFail("Pair action was unavailable after keyboard dismissal")
      throw NSError(domain: "saved_app_ui", code: 2)
    }
    pair.tap()
    let done = app.buttons["connection-done"]
    XCTAssertTrue(done.waitForExistence(timeout: 10))
    if mode == "analog-unsupported" {
      let error = app.staticTexts[
        "Ask your agent to start the receiver and open pairing, then try again."]
      XCTAssertTrue(error.waitForExistence(timeout: 20))
      XCTAssertFalse(app.staticTexts["A"].exists)
      XCTAssertFalse(app.staticTexts["saved-profile-summary"].exists)
      done.tap()
      XCTAssertTrue(home.waitForExistence(timeout: 10))
      try await AnalogAppHTTP.check(port: port, path: "/finish")
      return
    }
    guard app.staticTexts["connection-ready"].waitForExistence(timeout: 20) else {
      XCTFail("Native saved controller did not become ready")
      throw NSError(domain: "saved_app_ui", code: 3)
    }
    try await AnalogAppHTTP.check(port: port, path: "/fresh")
    let summary = app.staticTexts["saved-profile-summary"]
    XCTAssertTrue(summary.waitForExistence(timeout: 10))
    XCTAssertEqual(summary.label, "2 saved profiles")
    let label = app.staticTexts["South"].firstMatch
    XCTAssertTrue(label.waitForExistence(timeout: 10))
    XCTAssertFalse(app.staticTexts["B"].exists)
    XCTAssertFalse(app.staticTexts["North"].exists)
    let surface = app.descendants(matching: .any)["controller-surface"].firstMatch
    XCTAssertTrue(surface.exists)
    XCTAssertGreaterThanOrEqual(surface.frame.height, 240)
    XCTAssertLessThanOrEqual(done.frame.maxY, surface.frame.minY + 1)
    XCTAssertGreaterThanOrEqual(UIScreen.main.scale, 1)
    let bounds = surface.frame
    let stick = bounds.height * 0.3
    let triggerHeight = bounds.height * 0.11
    let centers = [
      CGPoint(x: bounds.minX + bounds.width * 0.2, y: bounds.minY + bounds.height * 0.6),
      CGPoint(x: bounds.minX + bounds.width * 0.8, y: bounds.minY + bounds.height * 0.6),
      CGPoint(x: bounds.minX + bounds.width * 0.35, y: bounds.minY + bounds.height * 0.15),
      CGPoint(x: bounds.minX + bounds.width * 0.65, y: bounds.minY + bounds.height * 0.15),
      CGPoint(x: bounds.midX, y: bounds.minY + bounds.height * 0.82),
    ]
    for (index, name) in ["Movement", "Camera", "Left trigger", "Right trigger", "South"]
      .enumerated()
    {
      let label = app.staticTexts[name].firstMatch
      XCTAssertTrue(label.waitForExistence(timeout: 5))
      XCTAssertEqual(label.frame.midX, centers[index].x, accuracy: 2)
      XCTAssertEqual(label.frame.midY, centers[index].y, accuracy: 2)
    }
    let points = [
      CGPoint(x: centers[0].x + stick * 0.236, y: centers[0].y),
      CGPoint(x: centers[1].x, y: centers[1].y - stick * 0.236),
      CGPoint(x: centers[2].x, y: centers[2].y + triggerHeight * 0.082),
      CGPoint(x: centers[3].x - stick * 0.082, y: centers[3].y),
      centers[4],
    ]
    XCTAssertTrue(points.allSatisfy { bounds.contains($0) && $0.y > done.frame.maxY })
    // XCTest transforms AX layout coordinates into its current screen orientation.
    let origin = app.coordinate(withNormalizedOffset: .zero)
    let screenPoints = points.map {
      origin.withOffset(CGVector(dx: $0.x, dy: $0.y)).screenPoint
    }
    let donePoint = done.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).screenPoint
    print("saved_analog_qa transformed=\(zip(points, screenPoints).contains { $0.0 != $0.1 })")
    let observation = Task.detached {
      try await Task.sleep(for: .milliseconds(300))
      try await AnalogAppHTTP.check(port: port, path: "/analog-start")
      try await Task.sleep(for: .milliseconds(1600))
      try await AnalogAppHTTP.check(port: port, path: "/analog-held")
    }
    let injected = await withCheckedContinuation { continuation in
      ThumbleSavedAnalogGesture(
        app, screenPoints.map { NSValue(cgPoint: $0) }, donePoint
      ) { succeeded in
        continuation.resume(returning: succeeded)
      }
    }
    XCTAssertTrue(injected, "Installed XCTest must support the isolated five-pointer QA gesture")
    try await observation.value
    XCTAssertTrue(home.waitForExistence(timeout: 10))
    try await AnalogAppHTTP.check(port: port, path: "/neutral")
    try await AnalogAppHTTP.check(port: port, path: "/finish")
  }
}
