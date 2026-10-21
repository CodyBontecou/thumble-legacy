import XCTest
import Foundation
import Network
@testable import Thumble

@MainActor
private final class OwnedPhoneMockPeer {
    private let queue = DispatchQueue(label: "Thumble.owned-phone.tests")
    private var listener: NWListener?
    private var connection: NWConnection?
    private var startup: CheckedContinuation<UInt16, Error>?
    var onMessage: ((Data, [String: Any]) -> Void)?
    private(set) var intents: [Data] = []
    func start() async throws -> UInt16 {
        let parameters = NWParameters.tcp
        parameters.requiredLocalEndpoint = .hostPort(host: "127.0.0.1", port: .any)
        let options = NWProtocolWebSocket.Options(); options.autoReplyPing = true
        parameters.defaultProtocolStack.applicationProtocols.insert(options, at: 0)
        let listener = try NWListener(using: parameters, on: .any)
        self.listener = listener
        listener.newConnectionHandler = { [weak self] connection in Task { @MainActor in self?.accept(connection) } }
        listener.stateUpdateHandler = { [weak self] state in
            Task { @MainActor in
                guard let self, let startup = self.startup else { return }
                switch state {
                case .ready:
                    self.startup = nil; startup.resume(returning: listener.port!.rawValue)
                case .failed(let error): self.startup = nil; startup.resume(throwing: error)
                default: break
                }
            }
        }
        return try await withCheckedThrowingContinuation { continuation in startup = continuation; listener.start(queue: queue) }
    }
    private func accept(_ connection: NWConnection) {
        self.connection = connection
        connection.stateUpdateHandler = { [weak self] state in
            guard case .ready = state else { return }
            Task { @MainActor in self?.receive(on: connection) }
        }
        connection.start(queue: queue)
    }
    private func receive(on connection: NWConnection) {
        connection.receiveMessage { [weak self] data, _, _, error in
            Task { @MainActor in
                guard let self, self.connection === connection, error == nil else { return }
                if let data, let object = try? JSONSerialization.jsonObject(with: data) as? [String:Any] {
                    if object["type"] as? String == "owned_phone_edit" { self.intents.append(data) }
                    self.onMessage?(data, object)
                }
                self.receive(on: connection)
            }
        }
    }
    func send(_ bytes: Data) {
        let context = NWConnection.ContentContext(identifier: "owned-test", metadata: [NWProtocolWebSocket.Metadata(opcode: .binary)])
        connection?.send(content: bytes, contentContext: context, isComplete: true, completion: .idempotent)
    }
    func stop() { connection?.cancel(); connection = nil; listener?.cancel(); listener = nil }
}

@MainActor
final class OwnedPhoneClientTests: XCTestCase {
    private var originalDefaults: [String:Any]?
    private var directory: URL!
    private var client: ControllerClient?
    private var peer: OwnedPhoneMockPeer?
    private var source = Data()
    private var document: ThumbleOwnedConfigurationDocument!
    private var cacheURL: URL { directory.appendingPathComponent("cache.json") }
    override func setUpWithError() throws {
        let domain = Bundle.main.bundleIdentifier!
        originalDefaults = UserDefaults.standard.persistentDomain(forName: domain)
        UserDefaults.standard.removePersistentDomain(forName: domain)
        directory = FileManager.default.temporaryDirectory.appendingPathComponent("owned-client-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        source = try Data(contentsOf: XCTUnwrap(Bundle(for: Self.self).url(forResource: "document", withExtension: "json")))
        document = try ThumbleOwnedConfigurationDocument(validating: source)
    }
    override func tearDownWithError() throws {
        client?.disconnect(); client = nil; peer?.stop(); peer = nil
        let domain = Bundle.main.bundleIdentifier!
        UserDefaults.standard.removePersistentDomain(forName: domain)
        if let originalDefaults { UserDefaults.standard.setPersistentDomain(originalDefaults, forName: domain) }
        try? FileManager.default.removeItem(at: directory)
    }
    private func waitFor(_ label: String, _ condition: () -> Bool) async throws {
        let deadline = ContinuousClock.now.advanced(by: .seconds(8))
        while !condition(), ContinuousClock.now < deadline { try await Task.sleep(for: .milliseconds(10)) }
        XCTAssertTrue(condition(), "\(label); client state \(String(describing: client?.state)), error \(client?.lastError ?? "none")")
    }
    private func changed(_ layout: GamepadCustomization, x: CGFloat, id: UUID? = nil) -> GamepadCustomization {
        var result = layout
        let elementIndex = id.flatMap { id in result.elements.firstIndex(where: { $0.id == id }) } ?? 0
        let element = result.elements[elementIndex]
        var paint = element.layout; paint.centerX = x
        if element.kind == .button, let builtin = element.defaultControlID { result.setButtonCustomization(paint, for: builtin) }
        else { result.elements[elementIndex].layout = paint; if let index = result.customButtons.firstIndex(where: { $0.id == element.id }) { result.customButtons[index].layout = paint } }
        return result
    }
    private func frame(type: String = "gamepad_profiles", server: String = "ios-mock-mac", revision: UInt64 = 41,
                       active: UUID? = nil, customization: GamepadCustomization? = nil, name: String? = nil) throws -> Data {
        var root = try XCTUnwrap(JSONSerialization.jsonObject(with: source) as? [String:Any])
        var profiles = try XCTUnwrap(root["profiles"] as? [[String:Any]])
        if let customization {
            var profile = document.projectedProfiles[0]; profile.customization = customization
            var object = try XCTUnwrap(JSONSerialization.jsonObject(with: JSONEncoder().encode(profile)) as? [String:Any])
            for key in ["skinReference","skinBaselineCustomization","landscapeSkinBaselineCustomization","portraitSkinBaselineCustomization","landscapeCustomization","portraitCustomization","orientationPreference"] { object.removeValue(forKey: key) }
            object["profileFormatVersion"] = 5; profiles[0] = object
        }
        if let name { profiles[0]["name"] = name }
        root = ["type":type, "timestamp":0, "serverID":server, "authToken":"isolated-ios-test-token",
            "profileFormatVersion":5, "configurationRevision":NSNumber(value:revision), "inputProtocolVersion":3,
            "capabilities":["owned_phone_edit_v1"], "gamepadProfiles":profiles,
            "gamepadProfileID":(active ?? document.activeProfileID).uuidString,
            "defaultGamepadProfileID":document.defaultProfileID?.uuidString as Any]
        return try JSONSerialization.data(withJSONObject:root)
    }
    private func receipt(_ edit: OwnedPhoneEdit, status: String, current: UInt64, committed: UInt64? = nil) throws -> Data {
        var result: [String:Any] = ["schemaVersion":1,"editID":edit.editID.uuidString,"profileID":edit.profileID.uuidString,
            "baseConfigurationRevision":NSNumber(value:edit.baseConfigurationRevision),"configurationRevision":NSNumber(value:current),
            "changed":committed.map { $0 > edit.baseConfigurationRevision } ?? false,"status":status]
        if let committed { result["committedRevision"] = NSNumber(value:committed) }
        else { result["errorCode"] = "owned_phone_revision_conflict" }
        return try JSONSerialization.data(withJSONObject:["type":"owned_phone_edit_result","timestamp":0,"profileFormatVersion":5,
            "configurationRevision":NSNumber(value:current),"ownedPhoneEditResult":result])
    }
    private func decodeIntent(_ bytes: Data) throws -> OwnedPhoneEdit {
        let fields = try JSONDecoder.rawObjectFields(in:bytes,named:["ownedPhoneEdit"])
        return try OwnedPhoneEdit(validating:XCTUnwrap(fields["ownedPhoneEdit"]))
    }
    func testClientPersistsBeforeSendingWaitsForReceiptAndDropsStaleCharacterEditorEvents() async throws {
        let peer = OwnedPhoneMockPeer(); self.peer = peer
        let accepted = try frame(type:"pairing_accepted")
        peer.onMessage = { [weak peer] _, message in if ["hello","pairing_request"].contains(message["type"] as? String ?? "") { peer?.send(accepted) } }
        let port = try await peer.start()
        let client = try ControllerClient(enableOwnedPhoneEdits:true,ownedCacheURL:cacheURL); self.client = client
        client.connect(hostField:"127.0.0.1",port:String(port),pairingCode:"123456")
        try await waitFor("owned pair") { client.isConnected }
        let profile = try XCTUnwrap(client.selectedGamepadProfile)
        let moved = changed(profile.customization,x:0.63)
        client.updateSelectedKeypadLayout(moved,orientation:.landscape,sendsToMac:false)
        XCTAssertEqual(try OwnedPhoneCache.load(from:cacheURL).journal.pending.count,0,"A preview is not a committed gesture")
        client.updateSelectedKeypadLayout(moved,orientation:.landscape,sendsToMac:true)
        try await waitFor("durable edit sent") { peer.intents.count == 1 }
        let pending = try XCTUnwrap(OwnedPhoneCache.load(from:cacheURL).journal.pending.first)
        XCTAssertEqual(try decodeIntent(peer.intents[0]).originalData,pending.edit.originalData)
        peer.send(try frame(revision:42,customization:moved,name:"Echo A"))
        try await waitFor("snapshot echo") { client.selectedGamepadProfileName == "Echo A" }
        XCTAssertEqual(client.pendingOwnedPhoneEditCount,1)
        peer.send(try receipt(pending.edit,status:"applied",current:42,committed:42))
        try await waitFor("explicit receipt") { client.pendingOwnedPhoneEditCount == 0 }
        let editor = IOSKeypadEditRuntime()
        editor.begin(with:client.gamepadCustomization,client:client)
        let other = document.projectedProfiles[1].id
        peer.send(try frame(revision:43,active:other,customization:moved))
        try await waitFor("character B") { client.selectedGamepadProfileID == other }
        editor.apply(changed(moved,x:0.71),orientation:.landscape,isFinal:true,client:client)
        XCTAssertFalse(editor.isEditing); XCTAssertEqual(client.pendingOwnedPhoneEditCount,0)
        XCTAssertEqual(peer.intents.count,1,"The old character editor cannot write into B")
        peer.send(try frame(revision:44,active:profile.id,customization:moved))
        try await waitFor("character A") { client.selectedGamepadProfileID == profile.id }
        XCTAssertEqual(client.gamepadCustomization.elements[0].layout.centerX,0.63)
        XCTAssertEqual(client.defaultGamepadProfileID,document.defaultProfileID)
    }
    func testClientReplaysOriginalRequestAndDefersEarlyReceiptUntilFreshSnapshot() async throws {
        let snapshot = try OwnedPhoneSnapshot(serverID:"ios-mock-mac",configurationRevision:41,documentData:source)
        let profile = snapshot.profiles[0]; let moved = changed(profile.customization,x:0.65)
        let initial = try OwnedPhoneSession(cache:.init(snapshot:snapshot,journal:.init())).recording(profileID:profile.id,customization:moved,nowMillis:100)
        try initial.cache.save(to:cacheURL)
        let original = initial.cache.journal.pending[0].edit
        let peer = OwnedPhoneMockPeer(); self.peer = peer
        let accepted = try frame(type:"pairing_accepted",revision:42,name:"Fresh A")
        let conflict = try receipt(original,status:"conflict",current:42)
        let fresh = try frame(revision:42,name:"Fresh A")
        let committed = try frame(revision:43,customization:moved,name:"Committed A")
        peer.onMessage = { [weak self, weak peer] data, object in
            guard let self, let peer else { return }
            switch object["type"] as? String {
            case "hello","pairing_request": peer.send(accepted)
            case "owned_phone_edit":
                guard let edit = try? self.decodeIntent(data) else { return }
                if edit.editID == original.editID { peer.send(conflict) }
                else if let receipt = try? self.receipt(edit,status:"applied",current:43,committed:43) { peer.send(receipt) }
            case "gamepad_profiles": peer.send(peer.intents.count < 2 ? fresh : committed)
            default: break
            }
        }
        let port = try await peer.start()
        let client = try ControllerClient(enableOwnedPhoneEdits:true,ownedCacheURL:cacheURL); self.client = client
        client.connect(hostField:"127.0.0.1",port:String(port),pairingCode:"123456")
        try await waitFor("retry and confirmed source") { peer.intents.count == 2 && client.pendingOwnedPhoneEditCount == 0 && client.selectedGamepadProfileName == "Committed A" }
        let first = try decodeIntent(peer.intents[0]); let second = try decodeIntent(peer.intents[1])
        XCTAssertEqual(first.originalData,original.originalData)
        XCTAssertNotEqual(first.editID,second.editID); XCTAssertEqual(second.baseConfigurationRevision,42)
        XCTAssertEqual(try OwnedPhoneCache.load(from:cacheURL).snapshot?.configurationRevision,43)
        XCTAssertEqual(client.gamepadCustomization.elements[0].layout.centerX,0.65)
    }
    func testActualHeadlessReceiverCommitsEditsAndProfileSelection() async throws {
        let environment = ProcessInfo.processInfo.environment
        guard let port = environment["THUMBLE_OWNED_RECEIVER_PORT"], let server = environment["THUMBLE_OWNED_RECEIVER_SERVER"],
              let token = environment["THUMBLE_OWNED_RECEIVER_TOKEN"], let secondText = environment["THUMBLE_OWNED_RECEIVER_SECOND_PROFILE"],
              let second = UUID(uuidString:secondText) else { throw XCTSkip("Run through verify-owned-phone-ios.py") }
        let credential: [String:Any] = ["serverID":server,"authToken":token,"macName":"Isolated receiver test",
            "lastURLString":"ws://127.0.0.1:\(port)","updatedAt":100]
        UserDefaults.standard.set(try JSONSerialization.data(withJSONObject:credential),forKey:"PocketPad.iOS.trustedMacCredential.v1")
        XCTAssertTrue(ControllerClient.ownedPhoneEditsEnabled)
        let client = try ControllerClient(enableOwnedPhoneEdits:ControllerClient.ownedPhoneEditsEnabled,ownedCacheURL:cacheURL,allowSmartConnectDuringTests:true); self.client = client
        client.startSmartConnect()
        try await waitFor("real owned receiver paired") { client.isConnected }
        let original = try XCTUnwrap(client.selectedGamepadProfile)
        let originalDefault = client.defaultGamepadProfileID
        let control = try XCTUnwrap(original.customization.elements.first(where: { !$0.layout.isHidden && !$0.layout.isLocationLocked }))
        let x: CGFloat = control.layout.centerX == 0.68 ? 0.66 : 0.68
        client.updateSelectedKeypadLayout(changed(original.customization,x:x,id:control.id),orientation:.landscape,sendsToMac:true)
        try await waitFor("real geometry committed and acknowledged") {
            client.pendingOwnedPhoneEditCount == 0 && (try? OwnedPhoneCache.load(from:self.cacheURL).snapshot?.configurationRevision) == 42
        }
        XCTAssertEqual(client.gamepadCustomization.elements.first(where: { $0.id == control.id })?.layout.centerX,x)
        let preference: GamepadColorSchemePreference = client.gamepadCustomization.colorSchemePreference == .dark ? .light : .dark
        client.setKeypadColorSchemePreference(preference)
        try await waitFor("real appearance committed and acknowledged") {
            client.pendingOwnedPhoneEditCount == 0 && (try? OwnedPhoneCache.load(from:self.cacheURL).snapshot?.configurationRevision) == 43
        }
        XCTAssertEqual(client.gamepadCustomization.colorSchemePreference,preference)
        client.selectGamepadProfile(second)
        try await waitFor("receiver selection B confirmed") { (try? OwnedPhoneCache.load(from:self.cacheURL).snapshot?.activeProfileID) == second }
        client.selectGamepadProfile(original.id)
        try await waitFor("receiver selection A confirmed") {
            (try? OwnedPhoneCache.load(from:self.cacheURL).snapshot?.activeProfileID) == original.id &&
            (try? OwnedPhoneCache.load(from:self.cacheURL).snapshot?.configurationRevision) == 45
        }
        XCTAssertEqual(client.defaultGamepadProfileID,originalDefault)
        XCTAssertEqual(client.gamepadCustomization.elements.first(where: { $0.id == control.id })?.layout.centerX,x)
        XCTAssertTrue(try OwnedPhoneCache.load(from:cacheURL).journal.pending.isEmpty)
    }

    func testLegacyPendingChangesBlockOwnedAdmissionAndCorruptionIsKept() async throws {
        let key = "PocketPad.iOS.pendingSkinSelections.v1"
        let pending = try JSONSerialization.data(withJSONObject:[["profileID":document.projectedProfiles[0].id.uuidString,"updatedAt":100]])
        UserDefaults.standard.set(pending,forKey:key)
        let peer = OwnedPhoneMockPeer(); self.peer = peer
        let accepted = try frame(type:"pairing_accepted")
        var offer: [Int]?
        peer.onMessage = { [weak peer] _, message in
            if ["hello","pairing_request"].contains(message["type"] as? String ?? "") {
                offer = message["supportedProfileFormats"] as? [Int]; peer?.send(accepted)
            }
        }
        let port = try await peer.start()
        let client = try ControllerClient(enableOwnedPhoneEdits:true,ownedCacheURL:cacheURL); self.client = client
        client.connect(hostField:"127.0.0.1",port:String(port),pairingCode:"123456")
        try await waitFor("legacy edits block migration") { if case .failed = client.state { return true }; return false }
        XCTAssertEqual(offer,[4]);XCTAssertEqual(UserDefaults.standard.data(forKey:key),pending)
        XCTAssertFalse(FileManager.default.fileExists(atPath:cacheURL.path))
        client.disconnect()
        for invalid in [Data("corrupt".utf8),Data(#"[{"profileID":"bad","updatedAt":100}]"#.utf8)] {
            UserDefaults.standard.set(invalid,forKey:key)
            XCTAssertThrowsError(try ControllerClient(enableOwnedPhoneEdits:true,ownedCacheURL:cacheURL))
            XCTAssertEqual(UserDefaults.standard.data(forKey:key),invalid)
        }
        UserDefaults.standard.set("wrong storage type",forKey:key)
        XCTAssertThrowsError(try ControllerClient(enableOwnedPhoneEdits:true,ownedCacheURL:cacheURL))
        XCTAssertEqual(UserDefaults.standard.string(forKey:key),"wrong storage type")
    }
    func testOfflineEditCreatedAfterHelloBlocksOwnedSnapshotAndKeepsLegacyRecovery() async throws {
        let peer = OwnedPhoneMockPeer(); self.peer = peer
        var offer: [Int]?
        peer.onMessage = { _, message in
            if ["hello","pairing_request"].contains(message["type"] as? String ?? "") {
                offer = message["supportedProfileFormats"] as? [Int]
            }
        }
        let port = try await peer.start()
        let client = try ControllerClient(enableOwnedPhoneEdits:ControllerClient.ownedPhoneEditsEnabled,ownedCacheURL:cacheURL); self.client = client
        client.connect(hostField:"127.0.0.1",port:String(port),pairingCode:"123456")
        try await waitFor("clean hello sent") { offer != nil }
        XCTAssertEqual(offer,[4,5])
        let originalID = client.selectedGamepadProfileID
        client.updateSelectedKeypadLayout(changed(client.gamepadCustomization,x:0.63),orientation:.landscape,sendsToMac:true)
        let pending = try PendingKeypadLayoutPersistence.load()
        XCTAssertEqual(pending.count,1)
        XCTAssertTrue(client.hasPendingKeypadLayoutEdits)
        peer.send(try frame(type:"pairing_accepted"))
        try await waitFor("new pending edit blocks owned snapshot") { client.lastError?.contains("need reconciliation") == true }
        XCTAssertFalse(client.isConnected)
        XCTAssertEqual(client.selectedGamepadProfileID,originalID)
        XCTAssertEqual(try PendingKeypadLayoutPersistence.load(),pending)
        XCTAssertFalse(FileManager.default.fileExists(atPath:cacheURL.path))
    }
    func testAnotherMacCannotApplyOrAcknowledgeAnOlderMacsPendingEdit() async throws {
        let snapshot = try OwnedPhoneSnapshot(serverID:"mac-a",configurationRevision:41,documentData:source)
        let profile = snapshot.profiles[0]
        let initial = try OwnedPhoneSession(cache:.init(snapshot:snapshot,journal:.init())).recording(profileID:profile.id,
            customization:changed(profile.customization,x:0.69),nowMillis:100)
        try initial.cache.save(to:cacheURL)
        let old = initial.cache.journal.pending[0].edit
        let peer = OwnedPhoneMockPeer();self.peer = peer
        let accepted = try frame(type:"pairing_accepted",server:"mac-b",name:"Mac B")
        peer.onMessage = { [weak peer] _, message in if ["hello","pairing_request"].contains(message["type"] as? String ?? "") { peer?.send(accepted) } }
        let port = try await peer.start()
        let client = try ControllerClient(enableOwnedPhoneEdits:true,ownedCacheURL:cacheURL);self.client = client
        client.connect(hostField:"127.0.0.1",port:String(port),pairingCode:"123456")
        try await waitFor("new Mac paired") { client.isConnected && client.selectedGamepadProfileName == "Mac B" }
        XCTAssertEqual(client.gamepadCustomization.elements[0].layout.centerX,profile.customization.elements[0].layout.centerX)
        XCTAssertEqual(client.pendingOwnedPhoneEditCount,1);XCTAssertTrue(peer.intents.isEmpty)
        peer.send(try receipt(old,status:"applied",current:42,committed:42))
        peer.send(try frame(server:"mac-b",revision:42,name:"Mac B refreshed"))
        try await waitFor("receipt from B processed") { client.selectedGamepadProfileName == "Mac B refreshed" }
        let saved = try OwnedPhoneCache.load(from:cacheURL)
        XCTAssertEqual(saved.snapshot?.serverID,"mac-b")
        XCTAssertEqual(saved.journal.pending.count,1)
        XCTAssertEqual(saved.journal.pending[0].serverID,"mac-a")
        XCTAssertEqual(saved.journal.pending[0].edit.originalData,old.originalData)
        XCTAssertEqual(saved.journal.pending[0].recoveryDocument,source)
        XCTAssertTrue(peer.intents.isEmpty)
    }

}
