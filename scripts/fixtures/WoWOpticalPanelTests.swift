import Foundation
import CoreGraphics
import ImageIO

@main
struct WoWOpticalPanelTests {
    static func main() throws {
        let fixtureURL = URL(fileURLWithPath: CommandLine.arguments[1])
        let fixture = try JSONSerialization.jsonObject(with: Data(contentsOf: fixtureURL)) as! [String: Any]
        let hex = (fixture["frames"] as! [String])[0]
        let chars = Array(hex)
        let bytes = stride(from: 0, to: chars.count, by: 2).map { UInt8(String(chars[$0...($0 + 1)]), radix: 16)! }
        assert(WoWOpticalPanel.validFrame(Data(bytes)))
        var corrupt = bytes; corrupt[100] ^= 1
        assert(!WoWOpticalPanel.validFrame(Data(corrupt)))
        for scale in [0.75, 0.853, 1, 1.1, 1.28, 1.5, 1.6, 2, 2.557, 3] {
            let image = makeImage(bytes, scale: scale)
            guard let result = try WoWOpticalPanel.decode(image) else { fatalError("No panel at scale \(scale)") }
            assert(result.frame == Data(bytes), "Wrong frame at scale \(scale)")
            let source = CGImageSourceCreateWithData(result.atlas as CFData, nil)!
            let atlas = CGImageSourceCreateImageAtIndex(source, 0, nil)!
            assert(atlas.width == 256 && atlas.height == 256)
            var pixel = [UInt8](repeating: 0, count: 4)
            pixel.withUnsafeMutableBytes { buffer in
                let context = CGContext(data: buffer.baseAddress, width: 1, height: 1, bitsPerComponent: 8,
                    bytesPerRow: 4, space: CGColorSpaceCreateDeviceRGB(),
                    bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
                context.draw(atlas.cropping(to: CGRect(x: 0, y: 0, width: 1, height: 1))!,
                    in: CGRect(x: 0, y: 0, width: 1, height: 1))
            }
            assert(pixel[0] == 0 && pixel[1] == 120 && pixel[2] == 240, "Atlas is flipped or misplaced")
        }
        let corruptResult = try WoWOpticalPanel.decode(makeImage(corrupt, scale: 1))
        assert(corruptResult == nil)
        print("WoW optical frame and atlas: 10 scales, corrupt payload rejected")
    }

    static func makeImage(_ bytes: [UInt8], scale: Double) -> CGImage {
        let width = 1024, height = 1100, x0 = 17, y0 = 23
        var pixels = [UInt8](repeating: 0, count: width * height * 4)
        func rect(_ x: Double, _ y: Double, _ w: Double, _ h: Double, _ color: [UInt8]) {
            for py in (y0 + Int((y * scale).rounded(.down)))..<(y0 + Int(((y + h) * scale).rounded(.down))) {
                for px in (x0 + Int((x * scale).rounded(.down)))..<(x0 + Int(((x + w) * scale).rounded(.down))) {
                    let offset = (py * width + px) * 4
                    pixels.replaceSubrange(offset..<(offset + 4), with: color + [255])
                }
            }
        }
        for i in 0..<4 { rect(Double(i * 6), 0, 6, 6, i % 2 == 0 ? [0,255,255] : [255,0,255]) }
        for i in 0..<4 { rect(Double(240 + i * 4), 0, 4, 6, i % 2 == 0 ? [0,255,255] : [255,0,255]) }
        for i in 0..<16 { let n = UInt8(i * 17); rect(Double(24 + i * 6), 0, 6, 6, [n,n,n]) }
        for i in 0..<1024 {
            let n = (i % 2 == 0 ? bytes[i / 2] >> 4 : bytes[i / 2] & 15) * 17
            rect(Double(i % 64 * 3), Double(8 + i / 64 * 3), 3, 3, [n,n,n])
        }
        for i in 0..<64 { rect(Double(i % 8 * 32), Double(64 + i / 8 * 32), 32, 32, [UInt8(i),120,240]) }
        let provider = CGDataProvider(data: Data(pixels) as CFData)!
        return CGImage(width: width, height: height, bitsPerComponent: 8, bitsPerPixel: 32,
                       bytesPerRow: width * 4, space: CGColorSpaceCreateDeviceRGB(),
                       bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedLast.rawValue),
                       provider: provider, decode: nil, shouldInterpolate: false, intent: .defaultIntent)!
    }
}
