// Draws Go:Voices' icon: the GO:KEYS round button, "GO:V" on its face.
// usage, from apple/: swift scripts/icon.swift GoVoices/Resources/Fonts/Quicksand-Variable.ttf GoVoices/Resources/Assets.xcassets/AppIcon.appiconset
import AppKit
import CoreText

let args = CommandLine.arguments
let fontURL = URL(fileURLWithPath: args[1]) as CFURL
let out = URL(fileURLWithPath: args[2])

func rgb(_ hex: UInt32, _ alpha: CGFloat = 1) -> CGColor {
    CGColor(srgbRed: CGFloat((hex >> 16) & 0xFF) / 255, green: CGFloat((hex >> 8) & 0xFF) / 255,
            blue: CGFloat(hex & 0xFF) / 255, alpha: alpha)
}

let red = rgb(0xE0242B)
let background = rgb(0x272727) // the panels, as the app
let face = rgb(0x272727)
let text = rgb(0xFFFFFF)

func font(size: CGFloat) -> CTFont {
    let descriptors = CTFontManagerCreateFontDescriptorsFromURL(fontURL) as! [CTFontDescriptor]
    let wght = 0x7767_6874 // 'wght'
    let bold = CTFontDescriptorCreateCopyWithAttributes(
        descriptors[0], [kCTFontVariationAttribute: [wght: 700]] as CFDictionary)
    return CTFontCreateWithFontDescriptor(bold, size, nil)
}

/// One icon, `pixels` square. `mac`: in the Mac's rounded square, with its
/// margin and shadow; otherwise full bleed, for iOS to mask.
func draw(pixels: Int, mac: Bool) -> CGImage {
    let ctx = CGContext(data: nil, width: pixels, height: pixels, bitsPerComponent: 8, bytesPerRow: 0,
                        space: CGColorSpace(name: CGColorSpace.sRGB)!,
                        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    let scale = CGFloat(pixels) / 1024
    ctx.scaleBy(x: scale, y: scale)

    // The tile: the app's panel grey, the button set into it as there.
    let tile = mac ? CGRect(x: 100, y: 100, width: 824, height: 824) : CGRect(x: 0, y: 0, width: 1024, height: 1024)
    if mac {
        ctx.saveGState()
        ctx.setShadow(offset: CGSize(width: 0, height: -10), blur: 20, color: rgb(0x000000, 0.35))
        ctx.addPath(CGPath(roundedRect: tile, cornerWidth: 185, cornerHeight: 185, transform: nil))
        ctx.setFillColor(background)
        ctx.fillPath()
        ctx.restoreGState()
    } else {
        ctx.setFillColor(background)
        ctx.fill(tile)
    }

    // The button: a red rim, the face set into it.
    let diameter = tile.width * 0.74
    let button = CGRect(x: tile.midX - diameter / 2, y: tile.midY - diameter / 2, width: diameter, height: diameter)
    ctx.setFillColor(red)
    ctx.fillEllipse(in: button)
    let rim = diameter * 0.05
    ctx.setFillColor(face)
    ctx.fillEllipse(in: button.insetBy(dx: rim, dy: rim))

    // GO:V, centred on the face.
    let size = diameter * 0.27
    let attributed = NSAttributedString(string: "GO:V", attributes: [
        kCTFontAttributeName as NSAttributedString.Key: font(size: size),
        kCTForegroundColorAttributeName as NSAttributedString.Key: text,
        kCTKernAttributeName as NSAttributedString.Key: size * 0.02,
    ])
    let line = CTLineCreateWithAttributedString(attributed)
    let bounds = CTLineGetBoundsWithOptions(line, .useGlyphPathBounds)
    ctx.textPosition = CGPoint(x: button.midX - bounds.midX, y: button.midY - bounds.midY)
    CTLineDraw(line, ctx)

    return ctx.makeImage()!
}

func write(_ image: CGImage, _ name: String) {
    let rep = NSBitmapImageRep(cgImage: image)
    try! rep.representation(using: .png, properties: [:])!.write(to: out.appendingPathComponent(name))
}

write(draw(pixels: 1024, mac: false), "ios-1024.png")
for points in [16, 32, 128, 256, 512] {
    for factor in [1, 2] {
        write(draw(pixels: points * factor, mac: true), "mac-\(points)@\(factor)x.png")
    }
}
