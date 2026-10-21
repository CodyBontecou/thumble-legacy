import AppKit
import ScreenCaptureKit
import ImageIO
import UniformTypeIdentifiers

// A separate bridge family: no configuration mutation and no arbitrary-window capture.
struct WoWLiveCaptureRequest: Decodable {
    let wowLiveCommand: String
}

struct WoWLiveCaptureReply: Encodable {
    let status: String
    var windowID: UInt32? = nil
    var frameBase64: String? = nil
    var atlasPNGBase64: String? = nil
}

enum WoWLiveCapture {
    @MainActor
    static func perform(_ request: WoWLiveCaptureRequest) async throws -> WoWLiveCaptureReply {
        guard ["status", "request-permission", "capture"].contains(request.wowLiveCommand) else {
            throw CaptureError.invalidCommand
        }
        if request.wowLiveCommand == "request-permission" {
            return WoWLiveCaptureReply(status: CGRequestScreenCaptureAccess() ? "authorized" : "permission-required")
        }
        guard CGPreflightScreenCaptureAccess() else {
            return WoWLiveCaptureReply(status: "permission-required")
        }
        if request.wowLiveCommand == "status" { return WoWLiveCaptureReply(status: "authorized") }
        // This helper is a command-line process with no AppKit startup cycle.
        // Initialize its display connection before SCContentFilter asks WindowServer
        // for display geometry (otherwise CoreGraphics asserts CGS_REQUIRE_INIT).
        _ = NSApplication.shared
        let content = try await SCShareableContent.excludingDesktopWindows(true, onScreenWindowsOnly: true)
        let windows = content.windows.filter { window in
            guard let app = window.owningApplication,
                  app.bundleIdentifier == "com.blizzard.worldofwarcraft",
                  let process = NSRunningApplication(processIdentifier: app.processID),
                  let executable = process.executableURL?.path else { return false }
            return ["/_classic_era_/", "/_forever_/"].contains { executable.contains($0) }
                && window.windowLayer == 0 && window.frame.width >= 320 && window.frame.height >= 320
        }
        // Ambiguous receivers must not alternate between two characters in separate clients.
        guard windows.count == 1, let window = windows.first else {
            return WoWLiveCaptureReply(status: windows.isEmpty ? "game-not-visible" : "multiple-game-windows")
        }
        let filter = SCContentFilter(desktopIndependentWindow: window)
        let configuration = SCStreamConfiguration()
        let scale = Double(filter.pointPixelScale)
        configuration.width = Int((filter.contentRect.width * scale).rounded())
        configuration.height = Int((filter.contentRect.height * scale).rounded())
        guard configuration.width > 0, configuration.height > 0,
              configuration.width <= 8192, configuration.height <= 8192,
              configuration.width * configuration.height <= 16_777_216 else { throw CaptureError.imageTooLarge }
        configuration.showsCursor = false
        let image = try await SCScreenshotManager.captureImage(contentFilter: filter, configuration: configuration)
        guard let panel = try WoWOpticalPanel.decode(image) else {
            return WoWLiveCaptureReply(status: "panel-not-visible", windowID: window.windowID)
        }
        return WoWLiveCaptureReply(status: "frame", windowID: window.windowID,
                                   frameBase64: panel.frame.base64EncodedString(),
                                   atlasPNGBase64: panel.atlas.base64EncodedString())
    }

    enum CaptureError: Error { case invalidCommand, imageTooLarge, invalidImage }
}

// Only the 512-byte telemetry frame and its 256-square icon atlas leave this decoder.
// Whole-window pixels are transient and are never written to disk or returned to the caller.
enum WoWOpticalPanel {
    struct Decoded {
        let frame: Data
        let atlas: Data
    }

    static func decode(_ image: CGImage) throws -> Decoded? {
        let width = image.width, height = image.height
        guard width > 0, height > 0, width <= 8192, height <= 8192,
              width * height <= 16_777_216 else { throw WoWLiveCapture.CaptureError.imageTooLarge }
        var pixels = [UInt8](repeating: 0, count: width * height * 4)
        let rendered = pixels.withUnsafeMutableBytes { bytes -> Bool in
            guard let context = CGContext(data: bytes.baseAddress, width: width, height: height,
                                          bitsPerComponent: 8, bytesPerRow: width * 4,
                                          space: CGColorSpaceCreateDeviceRGB(),
                                          bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
                                            | CGBitmapInfo.byteOrder32Big.rawValue) else { return false }
            context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
            return true
        }
        guard rendered else { throw WoWLiveCapture.CaptureError.invalidImage }
        func rgb(_ x: Int, _ y: Int) -> (Int, Int, Int) {
            let i = (y * width + x) * 4
            return (Int(pixels[i]), Int(pixels[i + 1]), Int(pixels[i + 2]))
        }
        func cyan(_ x: Int, _ y: Int) -> Bool {
            let p = rgb(x, y); return p.0 < 35 && p.1 > 210 && p.2 > 210
        }
        func magenta(_ x: Int, _ y: Int) -> Bool {
            let p = rgb(x, y); return p.0 > 210 && p.1 < 35 && p.2 > 210
        }
        // The panel is anchored at UIParent TOPLEFT. Bound discovery to the top-left
        // quadrant (window decorations and letterboxing are allowed).
        for y in 0..<min(height, 512) {
            var x = 0
            while x < min(width, 512) {
                guard cyan(x, y) else { x += 1; continue }
                let originX = x
                while x < width && cyan(x, y) { x += 1 }
                let markerWidth = x - originX
                guard markerWidth >= 4, markerWidth <= 48 else { continue }
                // Reject the same marker on every row: discover its top edge once.
                guard y == 0 || !cyan(originX + markerWidth / 2, y - 1) else { continue }
                var markerEnd = x
                for marker in 1..<4 {
                    let start = markerEnd
                    while markerEnd < width && (marker % 2 == 0 ? cyan(markerEnd, y) : magenta(markerEnd, y)) {
                        markerEnd += 1
                    }
                    if abs(markerEnd - start - markerWidth) > 1 { break }
                }
                // The short left marker can round by a pixel at fractional UI
                // scales. Measure across the entire 256-unit span instead.
                let approximateUnit = Double(markerEnd - originX) / 24
                let searchStart = originX + Int((230 * approximateUnit).rounded(.down))
                let searchEnd = min(width, originX + Int((270 * approximateUnit).rounded(.up)))
                guard searchStart < searchEnd else { continue }
                var rightEnd: Int?
                var scan = searchStart
                while scan < searchEnd {
                    guard cyan(scan, y) else { scan += 1; continue }
                    var edge = scan
                    var valid = true
                    for marker in 0..<4 {
                        let start = edge
                        while edge < width && (marker % 2 == 0 ? cyan(edge, y) : magenta(edge, y)) { edge += 1 }
                        if abs(Double(edge - start) - 4 * approximateUnit) > 2 { valid = false; break }
                    }
                    if valid { rightEnd = edge; break }
                    scan += 1
                }
                guard let rightEnd else { continue }
                let unit = Double(rightEnd - originX) / 256
                func point(_ dx: Double, _ dy: Double) -> (Int, Int) {
                    (originX + Int((dx * unit).rounded(.down)), y + Int((dy * unit).rounded(.down)))
                }
                let end = point(256, 320)
                guard end.0 <= width, end.1 <= height else { continue }
                let markersOK = (0..<4).allSatisfy { i in
                    let p = point(Double(i * 6 + 3), 3)
                    return i % 2 == 0 ? cyan(p.0, p.1) : magenta(p.0, p.1)
                }
                let rightMarkersOK = (0..<4).allSatisfy { i in
                    let p = point(Double(240 + i * 4 + 2), 3)
                    return i % 2 == 0 ? cyan(p.0, p.1) : magenta(p.0, p.1)
                }
                guard markersOK && rightMarkersOK else { continue }
                var palette: [Int] = []
                for i in 0..<16 {
                    let p = point(Double(24 + i * 6 + 3), 3), color = rgb(p.0, p.1)
                    guard abs(color.0 - color.1) < 8, abs(color.1 - color.2) < 8 else { break }
                    palette.append(color.0)
                }
                guard palette.count == 16, palette[0] < 25, palette[15] > 230,
                      (1..<16).allSatisfy({ palette[$0] - palette[$0 - 1] >= 5 }) else { continue }
                var nibbles: [UInt8] = []
                for i in 0..<1024 {
                    let p = point(Double(i % 64) * 3 + 1.5, 8 + Double(i / 64) * 3 + 1.5)
                    let color = rgb(p.0, p.1)
                    guard abs(color.0 - color.1) < 8, abs(color.1 - color.2) < 8,
                          let nearest = palette.indices.min(by: { abs(palette[$0] - color.0) < abs(palette[$1] - color.0) }),
                          abs(palette[nearest] - color.0) <= 7 else { break }
                    nibbles.append(UInt8(nearest))
                }
                guard nibbles.count == 1024 else { continue }
                let frame = Data(stride(from: 0, to: 1024, by: 2).map { nibbles[$0] << 4 | nibbles[$0 + 1] })
                guard validFrame(frame) else { continue }
                let atlasOrigin = point(0, 64)
                let rect = CGRect(x: atlasOrigin.0, y: atlasOrigin.1,
                                  width: end.0 - atlasOrigin.0, height: end.1 - atlasOrigin.1)
                // Decode crops from the normalized buffer, so image orientation is unambiguous.
                let normalized = pixels.withUnsafeMutableBytes { bytes -> CGImage? in
                    CGContext(data: bytes.baseAddress, width: width, height: height, bitsPerComponent: 8,
                              bytesPerRow: width * 4, space: CGColorSpaceCreateDeviceRGB(),
                              bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
                                | CGBitmapInfo.byteOrder32Big.rawValue)?.makeImage()
                }
                guard let atlas = normalized?.cropping(to: rect) else { continue }
                let encoded = NSMutableData()
                guard let destination = CGImageDestinationCreateWithData(encoded, UTType.png.identifier as CFString, 1, nil) else { continue }
                // Fractional scaling can produce a crop one pixel taller than
                // wide. Normalize the known square atlas to its 256-unit grid.
                guard let atlasContext = CGContext(data: nil, width: 256, height: 256, bitsPerComponent: 8,
                    bytesPerRow: 256 * 4, space: CGColorSpaceCreateDeviceRGB(),
                    bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { continue }
                atlasContext.interpolationQuality = .none
                atlasContext.draw(atlas, in: CGRect(x: 0, y: 0, width: 256, height: 256))
                guard let squareAtlas = atlasContext.makeImage() else { continue }
                CGImageDestinationAddImage(destination, squareAtlas, nil)
                guard CGImageDestinationFinalize(destination), encoded.length <= 2 * 1024 * 1024 else { continue }
                return Decoded(frame: frame, atlas: encoded as Data)
            }
        }
        return nil
    }

    static func validFrame(_ data: Data) -> Bool {
        let bytes = [UInt8](data)
        guard bytes.count == 512, Array(bytes.prefix(5)) == [84, 72, 87, 76, 1] else { return false }
        func u16(_ at: Int) -> Int { Int(bytes[at]) | Int(bytes[at + 1]) << 8 }
        func u32(_ at: Int) -> UInt32 { (0..<4).reduce(0) { $0 | UInt32(bytes[at + $1]) << ($1 * 8) } }
        let page = u16(13), count = u16(15), length = u16(17), total = u16(19)
        guard total > 0, total <= 32768, count == (total + 486) / 487, page < count,
              length == min(487, total - page * 487), bytes[(25 + length)...].allSatisfy({ $0 == 0 }) else { return false }
        var a: UInt32 = 1, b: UInt32 = 0
        for byte in Array(bytes[..<21]) + Array(bytes[25..<(25 + length)]) {
            a = (a + UInt32(byte)) % 65521; b = (b + a) % 65521
        }
        return (b << 16 | a) == u32(21)
    }
}
