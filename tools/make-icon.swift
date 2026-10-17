import AppKit
let output = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "assets/icon.png"
let size = 1024
let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
let box = NSBezierPath(roundedRect: NSRect(x: 60, y: 60, width: 904, height: 904), xRadius: 202, yRadius: 202)
NSGradient(starting: NSColor(srgbRed: 0.34, green: 0.43, blue: 0.86, alpha: 1), ending: NSColor(srgbRed: 0.55, green: 0.39, blue: 0.80, alpha: 1))!.draw(in: box, angle: -45)
NSColor.white.setStroke()
let frame = NSBezierPath(roundedRect: NSRect(x: 295, y: 212, width: 434, height: 600), xRadius: 50, yRadius: 50)
frame.lineWidth = 27; frame.stroke()
for y in [412, 612] {
  let line = NSBezierPath(); line.move(to: NSPoint(x: 300, y: y)); line.line(to: NSPoint(x: 724, y: y)); line.lineWidth = 25; line.stroke()
}
NSColor.white.setFill()
for y in [312, 512, 712] { NSBezierPath(ovalIn: NSRect(x: 355, y: y - 15, width: 30, height: 30)).fill() }
for y in [312, 512, 712] {
  let line = NSBezierPath(); line.move(to: NSPoint(x: 444, y: y)); line.line(to: NSPoint(x: 651, y: y)); line.lineWidth = 23; line.lineCapStyle = .round; line.stroke()
}
NSGraphicsContext.restoreGraphicsState()
try bitmap.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: output))
