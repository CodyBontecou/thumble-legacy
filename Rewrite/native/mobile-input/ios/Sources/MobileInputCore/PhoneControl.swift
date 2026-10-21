import Foundation

public enum PhoneControlError: Error { case rejected }
public struct TrustedHostEndpoint: Sendable {
  public let host: String, certificatePin: String, authorityInstanceID: String
  public let port: UInt16
  public init(host: String, port: UInt16, certificatePin: String, authorityInstanceID: String)
    throws
  {
    guard !host.isEmpty, host.utf8.count <= 253, !host.contains("\0"), port > 0,
      PhoneCredential.pin(certificatePin), EnrollmentInvitation.identity(authorityInstanceID)
    else { throw PhoneControlError.rejected }
    self.host = host
    self.port = port
    self.certificatePin = certificatePin
    self.authorityInstanceID = authorityInstanceID
  }
}
public struct PhoneControlMetadata: Sendable, Equatable {
  public let phoneID: String, credentialEpoch: UInt64, connectionID: String, expiresAt: UInt64
  public let inputSessionAvailable: Bool
}
protocol ControlCredentialSource: Sendable {
  func load(hostPin: String, now: UInt64) async throws -> PhoneCredential?
}
extension PhoneCredentialWorker: ControlCredentialSource {}

/// One pinned native channel. Reconnection requires a fresh client and never restores touches.
@MainActor public final class PhoneControlClient {
  private enum State { case idle, authenticating, authenticated, closed }
  private var state: State = .idle
  private var busy = false
  private var last: UInt64 = 0
  private var deadline: UInt64?
  private var admitted: PhoneControlMetadata?
  private let endpoint: TrustedHostEndpoint
  private let io: any EnrollmentFrameIO
  private let storage: any ControlCredentialSource
  private let now: () -> UInt64
  public convenience init(endpoint: TrustedHostEndpoint, storage: PhoneCredentialWorker) throws {
    try self.init(
      endpoint: endpoint,
      io: ControlTLSClient(
        host: endpoint.host, port: endpoint.port, certificatePin: endpoint.certificatePin),
      storage: storage,
      now: {
        let time = Date().timeIntervalSince1970 * 1000
        return time.isFinite && time >= 0 && time < Double(Int64.max) ? UInt64(time) : UInt64.max
      })
  }
  init(
    endpoint: TrustedHostEndpoint, io: any EnrollmentFrameIO, storage: any ControlCredentialSource,
    now: @escaping () -> UInt64
  ) {
    self.endpoint = endpoint
    self.io = io
    self.storage = storage
    self.now = now
  }
  @discardableResult private func checkpoint() throws -> UInt64 {
    try Task.checkCancellation()
    let time = now()
    guard state != .closed, time >= last, time <= UInt64(Int64.max),
      deadline.map({ time < $0 }) ?? true, admitted.map({ time < $0.expiresAt }) ?? true
    else { throw PhoneControlError.rejected }
    last = time
    return time
  }
  private func send(_ value: [String: Any]) throws {
    try checkpoint()
    guard
      io.offer(
        try ControlTLSFrame(kind: .control, payload: JSONSerialization.data(withJSONObject: value)))
    else { throw PhoneControlError.rejected }
  }
  private func receive() async throws -> [String: EnrollmentJSON.Value] {
    let frame = try await io.receive()
    try checkpoint()
    guard frame.kind == .control else { throw PhoneControlError.rejected }
    return try EnrollmentJSON.parse(frame.payload)
  }
  public func authenticate() async throws -> PhoneControlMetadata {
    guard state == .idle, !busy else { throw PhoneControlError.rejected }
    state = .authenticating
    busy = true
    defer { busy = false }
    return try await withTaskCancellationHandler {
      do {
        try checkpoint()
        try await io.connect()
        try checkpoint()
        try send([
          "version": 1, "command": "hello", "controlVersion": 3, "inputVersion": 1,
          "documentVersion": 1, "capabilitiesVersion": 1,
        ])
        let hello = try await receive()
        guard
          hello == [
            "version": .integer(1), "command": .string("hello"),
            "authorityInstanceId": .string(endpoint.authorityInstanceID),
            "controlVersion": .integer(3), "inputVersion": .integer(1),
            "documentVersion": .integer(1), "capabilitiesVersion": .integer(1),
            "inputEnabled": .boolean(false),
          ]
        else { throw PhoneControlError.rejected }
        let credential = try await storage.load(hostPin: endpoint.certificatePin, now: checkpoint())
        try checkpoint()
        guard let credential, credential.metadata.hostPin == endpoint.certificatePin,
          last < credential.metadata.expiresAt,
          io.offer(try credential.authenticationFrame(authority: endpoint.authorityInstanceID))
        else { throw PhoneControlError.rejected }
        let values = try await receive()
        let receipt = try decode(values, command: "authenticated", credential: credential.metadata)
        admitted = receipt
        deadline = min(receipt.expiresAt, last + 60_000)
        state = .authenticated
        return receipt
      } catch {
        close()
        throw PhoneControlError.rejected
      }
    } onCancel: {
      Task { @MainActor [weak self] in self?.close() }
    }
  }
  private func decode(
    _ value: [String: EnrollmentJSON.Value], command: String,
    credential: PhoneCredentialMetadata? = nil
  ) throws -> PhoneControlMetadata {
    guard
      Set(value.keys)
        == Set([
          "version", "command", "authorityInstanceId", "phoneId", "credentialEpoch", "connectionId",
          "expiresAt", "inputSessionAvailable", "inputEnabled",
        ]),
      value["version"] == .integer(1), value["command"] == .string(command),
      value["authorityInstanceId"] == .string(endpoint.authorityInstanceID),
      value["inputEnabled"] == .boolean(false), value["inputSessionAvailable"] == .boolean(false),
      case .string(let phone)? = value["phoneId"], EnrollmentInvitation.identity(phone),
      case .string(let epochText)? = value["credentialEpoch"], let epoch = UInt64(epochText),
      epoch > 0, epoch <= UInt64(Int64.max), String(epoch) == epochText,
      case .string(let connection)? = value["connectionId"], connection.utf8.count == 32,
      connection.utf8.allSatisfy({ (48...57).contains($0) || (97...102).contains($0) }),
      case .integer(let expires)? = value["expiresAt"], expires > last,
      credential.map({ $0.phoneID == phone && $0.epoch == epoch && expires <= $0.expiresAt })
        ?? true
    else { throw PhoneControlError.rejected }
    let receipt = PhoneControlMetadata(
      phoneID: phone, credentialEpoch: epoch, connectionID: connection, expiresAt: expires,
      inputSessionAvailable: false)
    guard admitted.map({ $0 == receipt }) ?? true else { throw PhoneControlError.rejected }
    return receipt
  }
  public func status() async throws -> PhoneControlMetadata {
    try await operation(disconnect: false)
  }
  public func disconnect() async throws { _ = try await operation(disconnect: true) }
  private func operation(disconnect: Bool) async throws -> PhoneControlMetadata {
    guard state == .authenticated, !busy, let admitted else { throw PhoneControlError.rejected }
    busy = true
    defer { busy = false }
    return try await withTaskCancellationHandler {
      do {
        try send([
          "version": 1, "command": disconnect ? "disconnect" : "control-status",
          "connectionId": admitted.connectionID,
        ])
        let result = try decode(
          try await receive(), command: disconnect ? "disconnected" : "control-status")
        if disconnect { close() }
        return result
      } catch {
        close()
        throw PhoneControlError.rejected
      }
    } onCancel: {
      Task { @MainActor [weak self] in self?.close() }
    }
  }
  public func close() {
    state = .closed
    admitted = nil
    io.close()
  }
}
