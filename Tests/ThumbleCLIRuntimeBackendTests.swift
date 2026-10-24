import Foundation
import XCTest
import Testing

final class ThumbleCLIRuntimeBackendTests: XCTestCase {
    private let id = UUID(uuidString: "AAAAAAAA-BBBB-5CCC-8DDD-EEEEEEEEEEEE")!
    private let instance = UUID(uuidString: "11111111-2222-4333-8444-555555555555")!

    func testRequestIsSeparateTaggedBoundedEnvelope() throws {
        let data = try ThumbleCLIRuntimeBackend.requestFrame(.tapControl("jump"), invocationID: id)
        XCTAssertEqual(String(decoding: data, as: UTF8.self),
            "{\"invocationID\":\"AAAAAAAA-BBBB-5CCC-8DDD-EEEEEEEEEEEE\",\"runtimeCommand\":{\"controlID\":\"jump\",\"type\":\"tap-control\"},\"schemaVersion\":1}\n")
        for control in ["", String(repeating: "x", count: 257), "jump\n", "../raw"] {
            XCTAssertThrowsError(try ThumbleCLIRuntimeBackend.requestFrame(.tapControl(control), invocationID: id))
        }
    }

    func testReceiverOwnedHeldTestsUseTypedBoundedControlRequests() throws {
        for state in [ButtonPressState.down, .up] {
            let data = try ThumbleCLIRuntimeBackend.requestFrame(.testControl("button:jump", state), invocationID: id)
            let object = try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
            let command = try XCTUnwrap(object["runtimeCommand"] as? [String: String])
            XCTAssertEqual(command, ["type": "test-control", "controlID": "button:jump", "state": state.rawValue])
        }
        XCTAssertThrowsError(try ThumbleCLIRuntimeBackend.requestFrame(.testControl("../raw", .down), invocationID: id))
    }

    func testPermissionAndPairingCommandsRequireTheirTypedAcknowledgements() throws {
        let commands: [(ThumbleCLIRuntimeBackend.Command, String)] = [
            (.accessibility(.status), "accessibilityTrusted"), (.accessibility(.prompt), "accessibilityTrusted"),
            (.accessibility(.open), "accessibilityTrusted"), (.cancelPairing, "pairingCancelled")]
        for (command, field) in commands {
            let data = try ThumbleCLIRuntimeBackend.requestFrame(command, invocationID: id)
            let object = try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
            let wire = try XCTUnwrap(object["runtimeCommand"] as? [String: String])
            if case .accessibility(let action) = command { XCTAssertEqual(wire, ["type":"accessibility", "action":action.rawValue]) }
            else { XCTAssertEqual(wire, ["type":"pairing-cancel"]) }
            for flag in ["true", "false"] {
                let backend = ThumbleCLIRuntimeBackend(transport: { _ in (0, self.frame(owner:"rust", response:"{\"ok\":true,\"\(field)\":\(flag)}")) })
                XCTAssertEqual(try backend.route(command, invocationID:id).owner, .rust)
            }
            for receipt in ["{\"ok\":true}", "{\"ok\":true,\"\(field)\":1}"] {
                let backend = ThumbleCLIRuntimeBackend(transport: { _ in (0, self.frame(owner:"rust",response:receipt)) })
                XCTAssertThrowsError(try backend.route(command, invocationID:id))
            }
        }
    }

    func testStrictResponseFramingBoundsAndCorrelation() throws {
        let good = frame(owner: "rust", response: "{\"ok\":true,\"released\":true}")
        XCTAssertEqual(try ThumbleCLIRuntimeBackend.decodeResponse(good, invocationID: id).owner, .rust)
        for bad in [Data(good.dropLast()), good + good, Data(repeating: 32, count: 65537),
                    frame(owner: "offline"), frame(owner: "none", response: "{\"ok\":true}"),
                    Data(String(decoding: good, as: UTF8.self).replacingOccurrences(of: id.uuidString, with: instance.uuidString).utf8)] {
            XCTAssertThrowsError(try ThumbleCLIRuntimeBackend.decodeResponse(bad, invocationID: id))
        }
    }

    func testUnreachableRustNeverInvokesLegacyFallback() throws {
        let backend = ThumbleCLIRuntimeBackend(transport: { _ in
            (1, self.frame(owner: "unreachable", ok: false, error: "authority_unreachable"))
        })
        XCTAssertThrowsError(try backend.route(.releaseAll, invocationID: id))
    }

    func testAbsentAndLegacyCandidatesNeverReadDesktopStatus() throws {
        for (owner, error) in [("none", "runtime_not_running"), ("unreachable", "legacy_owner_possible")] {
            let backend = ThumbleCLIRuntimeBackend(transport: { _ in (1, self.frame(owner: owner, ok: false, error: error)) })
            for command in [ThumbleCLIRuntimeBackend.Command.status, .releaseAll, .launchProfile(.active)] {
                XCTAssertThrowsError(try backend.route(command, invocationID: id))
            }

        }
    }

    func testRustRouteNeverReadsLegacyAndRequiresAcknowledgement() throws {
        var requests: [String] = []
        let backend = ThumbleCLIRuntimeBackend(transport: { request in
            requests.append(String(decoding: request, as: UTF8.self))
            return (0, self.frame(owner: "rust", response: "{\"ok\":true,\"released\":true}"))
        })
        let result = try backend.route(.releaseAll, invocationID: id)
        XCTAssertEqual(result.owner, .rust)
        XCTAssertTrue(requests.last?.contains("release-all") == true)
    }

    func testAmbiguousLockCannotAuthorizeInput() throws {
        let backend = ThumbleCLIRuntimeBackend(transport: { _ in
            (1, self.frame(owner: "unreachable", ok: false, error: "legacy_owner_possible"))
        })
        XCTAssertThrowsError(try backend.route(.tapControl("jump"), invocationID: id))
    }

    func testEnvelopeTypesAndUnexpectedFieldsFailClosed() throws {
        let good = String(decoding: frame(owner: "none"), as: UTF8.self)
        for text in [good.replacingOccurrences(of: "\"ok\":true", with: "\"ok\":1"),
                     good.replacingOccurrences(of: "\"schemaVersion\":1", with: "\"schemaVersion\":true"),
                     good.replacingOccurrences(of: "\"schemaVersion\":1", with: "\"schemaVersion\":2"),
                     good.replacingOccurrences(of: "\"owner\":\"none\"", with: "\"owner\":\"none\",\"socketPath\":\"/tmp/evil\""),
                     good.replacingOccurrences(of: "\"owner\":\"none\"", with: "\"owner\":\"none\",\"owner\":\"none\""),
                     good.replacingOccurrences(of: "\"owner\":\"none\"", with: "\"owner\":\"none\",\"\\u006fwner\":\"none\"")] {
            XCTAssertThrowsError(try ThumbleCLIRuntimeBackend.decodeResponse(Data(text.utf8), invocationID: id))
        }
    }

    func testRustTapNeedsMatchingControlAcknowledgementAndNeverFallsBack() throws {
        let backend = ThumbleCLIRuntimeBackend(transport: { _ in
            (0, self.frame(owner: "rust", response: "{\"ok\":true,\"pressedControlID\":\"button:attack\"}"))
        })
        XCTAssertThrowsError(try backend.route(.tapControl("button:jump"), invocationID: id))
    }

    func testGamepadReadinessProjectionIsTypedAndRangeBounded() throws {
        func response(_ status: VirtualGamepadStatus) throws -> Data {
            let value = String(decoding: try JSONEncoder().encode(status), as: UTF8.self)
            return frame(owner: "rust", response: "{\"ok\":true,\"virtualGamepadStatus\":\(value)}")
        }
        let recording = try ThumbleCLIRuntimeBackend.decodeResponse(
            response(VirtualGamepadStatus(phase: .recording)), invocationID: id)
        XCTAssertEqual(recording.control?.virtualGamepadStatus?.phase, .recording)
        for invalid in [VirtualGamepadStatus(leftStickX: 2), VirtualGamepadStatus(leftTrigger: -0.1),
                        VirtualGamepadStatus(lastError: String(repeating: "x", count: 2049)),
                        VirtualGamepadStatus(pressedButtons: [.south, .south])] {
            XCTAssertThrowsError(try ThumbleCLIRuntimeBackend.decodeResponse(response(invalid), invocationID: id))
        }
    }

    func testExecutableValidationRejectsMissingBridgeBeforeLaunching() {
        XCTAssertThrowsError(try ThumbleCLIRuntimeBackend(executableURL:
            URL(fileURLWithPath: "/nonexistent-thumble-test/thumble-cli-bridge")))
    }

    func testReceiverLifecycleAcceptsOnlyKnownOptionsAndBoundsThePort() throws {
        typealias Command = ThumbleCLIRuntimeBackend.ReceiverCommand
        XCTAssertEqual(try Command.parse(["start"]).arguments, ["start"])
        XCTAssertEqual(try Command.parse(["start", "--no-input", "--port", "0", "--allow-config-write", "--no-bonjour"]).arguments,
                       ["start", "--port", "0", "--no-bonjour", "--no-input", "--allow-config-write"])
        XCTAssertEqual(try Command.parse(["start", "--port", "65535"]).arguments, ["start", "--port", "65535"])
        for command in ["stop", "restart"] { XCTAssertEqual(try Command.parse([command]).arguments, [command]) }
        for arguments in [[], ["run"], ["stop", "--port", "42"], ["restart", "--no-input"],
                          ["start", "--port"], ["start", "--port", "65536"], ["start", "--port", "-1"],
                          ["start", "--port", "+42"], ["start", "--port", "42.0"], ["start", "--port", ""],
                          ["start", "--state-dir", "/tmp/other"], ["start", "--no-input", "--no-input"],
                          ["start", "--port", "42", "--port", "43"]] {
            XCTAssertThrowsError(try Command.parse(arguments), "\(arguments)")
        }
    }

    func testHeadlessStatusDistinguishesStoppedMissingAndUnreachableAuthority() throws {
        let stopped = ThumbleCLIRuntimeBackend(transport: { _ in (0, self.frame(owner: "none")) })
        XCTAssertNil(try stopped.headlessStatus(invocationID: id))
        let good = ThumbleCLIRuntimeBackend(transport: { _ in (0, try self.headlessStatusFrame()) })
        XCTAssertEqual(try good.headlessStatus(invocationID: id)?.control?.status?.serviceName, "Headless receiver")
        for frame in [self.frame(owner: "unreachable", ok: false, error: "legacy_owner_possible"),
                      self.frame(owner: "unreachable", ok: false, error: "authority_unreachable"),
                      self.frame(owner: "rust", response: "{\"ok\":true}")] {
            let backend = ThumbleCLIRuntimeBackend(transport: { _ in (1, frame) })
            XCTAssertThrowsError(try backend.headlessStatus(invocationID: id))
        }
    }

    func testReceiverLifecycleUsesValidatedSiblingProcessAndReportsFailure() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true,
                                               attributes: [.posixPermissions: 0o700])
        defer { try? FileManager.default.removeItem(at: directory) }
        let host = directory.appendingPathComponent("thumble-host")
        let script = """
        #!/bin/sh
        case "$1" in
        start) printf '%s\\n' "$@" ;;
        stop) printf 'receiver rejected stop\\n' >&2; exit 7 ;;
        restart) exec /bin/sleep 10 ;;
        esac
        """
        try Data(script.utf8).write(to: host)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: host.path)
        let command = try ThumbleCLIRuntimeBackend.ReceiverCommand.parse(["start", "--port", "0", "--no-input"])
        let output = try ThumbleCLIRuntimeBackend.runReceiver(command, executableURL: host)
        XCTAssertEqual(String(decoding: output, as: UTF8.self), "start\n--port\n0\n--no-input\n")
        XCTAssertThrowsError(try ThumbleCLIRuntimeBackend.runReceiver(.stop, executableURL: host)) {
            XCTAssertEqual($0.localizedDescription, "receiver rejected stop")
        }
        XCTAssertThrowsError(try ThumbleCLIRuntimeBackend.runReceiver(.restart, executableURL: host, timeout: 0.05)) {
            guard case .timeout = $0 as? ThumbleCLIRuntimeBackend.BackendError else { return XCTFail("Expected timeout: \($0)") }
        }
        let link = directory.appendingPathComponent("linked-host")
        try FileManager.default.createSymbolicLink(at: link, withDestinationURL: host)
        XCTAssertThrowsError(try ThumbleCLIRuntimeBackend.runReceiver(command, executableURL: link))
        try FileManager.default.setAttributes([.posixPermissions: 0o722], ofItemAtPath: host.path)
        XCTAssertThrowsError(try ThumbleCLIRuntimeBackend.runReceiver(command, executableURL: host))
    }

    func testHeadlessPairingPayloadUsesVerifiedReceiverWithoutLegacyStatus() throws {
        let backend = ThumbleCLIRuntimeBackend(transport: { _ in
            (0, try self.headlessStatusFrame())
        })
        let route = try backend.route(.status, invocationID: id)
        let response = route
        XCTAssertEqual(response.owner, .rust)
        let payload = try XCTUnwrap(response.control?.status).pairingPayload()
        XCTAssertEqual(payload.urls, ["ws://192.168.1.42:9123"])
        XCTAssertEqual(payload.pairingCode, "123456")
        XCTAssertEqual(payload.serverID, instance.uuidString)
        XCTAssertEqual(payload.serviceName, "Headless receiver")
        XCTAssertEqual(payload.normalizedServiceType, "_pocketpad._tcp")
        XCTAssertNotNil(PairingPayload.decode(from: String(decoding: try JSONEncoder().encode(payload), as: UTF8.self)))
    }

    func testIncompleteHeadlessPairingCannotCreatePayloadOrConsultDesktop() throws {
        for field in ["urls", "pairingCode", "serverID"] {
            let backend = ThumbleCLIRuntimeBackend(transport: { _ in
                (0, try self.headlessStatusFrame(omitting: field))
            })
            let route = try backend.route(.status, invocationID: id)
            let response = route
        XCTAssertEqual(response.owner, .rust)
            XCTAssertThrowsError(try XCTUnwrap(response.control?.status).pairingPayload())
        }
    }

    private func headlessStatusFrame(omitting field: String? = nil) throws -> Data {
        var status: [String: Any] = [
            "pid": 42, "port": 9123, "serviceName": "Headless receiver",
            "accessibilityTrusted": true, "inputEnabled": true,
            "output": ["mode": "recording"], "urls": ["ws://192.168.1.42:9123"],
            "pairingCode": "123456", "serverID": instance.uuidString
        ]
        if let field { status.removeValue(forKey: field) }
        let json = try JSONSerialization.data(withJSONObject: ["ok": true, "status": status])
        return frame(owner: "rust", response: String(decoding: json, as: UTF8.self))
    }

    private func frame(owner: String, ok: Bool = true, response: String? = nil, error: String? = nil) -> Data {
        var text = "{\"schemaVersion\":1,\"ok\":\(ok),\"invocationID\":\"\(id.uuidString)\",\"owner\":\"\(owner)\""
        if let response { text += ",\"response\":\(response)" }
        if let error { text += ",\"error\":{\"code\":\"\(error)\",\"message\":\"Unavailable\"}" }
        return Data((text + "}\n").utf8)
    }
}


struct RuntimeProfileLaunchTests {
    private let invocation = UUID(uuidString: "AAAAAAAA-BBBB-5CCC-8DDD-EEEEEEEEEEEE")!
    private let profile = UUID(uuidString: "11111111-2222-4333-8444-555555555555")!

    @Test func launchCarriesOnlyTheTypedProfileSelector() throws {
        for selector in [ThumbleCLIProfileBackend.ProfileSelector.active, .default, .id(profile), .name("Work shortcuts")] {
            let data = try ThumbleCLIRuntimeBackend.requestFrame(.launchProfile(selector), invocationID: invocation)
            let root = try #require(JSONSerialization.jsonObject(with: data) as? [String: Any])
            let command = try #require(root["runtimeCommand"] as? [String: Any])
            #expect(Set(command.keys) == ["type", "profile"])
            #expect(command["type"] as? String == "launch-profile")
        }
        for name in ["", "line\nbreak", String(repeating: "x", count: 257)] {
            #expect(throws: (any Error).self) {
                try ThumbleCLIRuntimeBackend.requestFrame(.launchProfile(.name(name)), invocationID: invocation)
            }
        }
    }

    @Test func launchAcknowledgementMustMatchTheRequestedProfile() throws {
        let other = UUID()
        for acknowledged in [profile, other] {
            let backend = ThumbleCLIRuntimeBackend(transport: { _ in
                (0, self.frame(profile: acknowledged, phase: "resolving"))
            })
            if acknowledged == profile {
                let route = try backend.route(.launchProfile(.id(profile)), invocationID: invocation)
                let response = route
                #expect(response.owner == .rust)
                #expect(response.control?.applicationLaunch?.profileID == profile)
            } else {
                #expect(throws: (any Error).self) {
                    try backend.route(.launchProfile(.id(profile)), invocationID: invocation)
                }
            }
        }
    }

    @Test func launchProjectionRejectsInvalidPhaseAndFailureMetadata() throws {
        for (phase, code) in [("unknown", nil), ("failed", nil), ("resolving", "unexpected"), ("failed", "path/unsafe")] as [(String, String?)] {
            #expect(throws: (any Error).self) {
                try ThumbleCLIRuntimeBackend.decodeResponse(frame(profile: profile, phase: phase, code: code), invocationID: invocation)
            }
        }
        let failed = try ThumbleCLIRuntimeBackend.decodeResponse(frame(profile: profile, phase: "failed", code: "application_not_found"), invocationID: invocation)
        #expect(failed.control?.applicationLaunch?.errorCode == "application_not_found")
    }

    private func frame(profile: UUID, phase: String, code: String? = nil) -> Data {
        var launch: [String: Any] = ["launchID": UUID().uuidString, "profileID": profile.uuidString, "phase": phase]
        if let code { launch["errorCode"] = code }
        let root: [String: Any] = ["schemaVersion": 1, "ok": true, "invocationID": invocation.uuidString,
            "owner": "rust", "response": ["ok": true, "applicationLaunch": launch]]
        return (try! JSONSerialization.data(withJSONObject: root)) + Data([10])
    }
}
