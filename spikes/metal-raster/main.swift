// Metal rasteriser spike: scene generator, CPU reference, GPU strategies A/B, bench + fidelity.
import Foundation
import Metal

// MARK: generator (seeded xorshift, same generator as rust/crates/vexel-render/tests/common/raster_scene.rs)
struct Rng {
    var s: UInt64
    mutating func next() -> UInt64 {
        s ^= s >> 12; s ^= s << 25; s ^= s >> 27
        return s &* 0x2545_F491_4F6C_DD1D
    }
    mutating func unit() -> Double { Double(next() >> 11) / Double(UInt64(1) << 53) }
    mutating func range(_ lo: Double, _ hi: Double) -> Double { lo + (hi - lo) * unit() }
    mutating func below(_ n: Int) -> Int { Int(next() % UInt64(n)) }
    mutating func color() -> [Double] {
        let a = below(4) == 0 ? 255 : 40 + below(216)
        return [Double(below(256)), Double(below(256)), Double(below(256)), Double(a)]
    }
}

enum Profile: String, CaseIterable { case fill, gradient, stroke, nest }
typealias Pt = (Double, Double)
let SF = 28
let LOC: [Double] = [0.125, 0.375, 0.625, 0.875]
let STOPS: [[Double]] = [[0, 250, 30, 20, 255], [0.35, 240, 220, 40, 200], [0.7, 20, 160, 230, 255], [1, 90, 20, 160, 90]]

struct Scene {
    var count = 0
    var sf: [Double] = []   // SF doubles per shape
    var su: [UInt32] = []   // edgeStart, edgeCount, flags(1=evenodd,2=gradient), packed rgba
    var edges: [Double] = [] // x0,y0,x1,y1

    mutating func add(polys: [[Pt]], evenodd: Bool, color: [Double], gradient: Bool, gp: [Double], opacity: Double) {
        let start = edges.count / 4
        var lo = (Double.infinity, Double.infinity), hi = (-Double.infinity, -Double.infinity)
        for poly in polys where poly.count > 1 {
            for i in 0..<poly.count {
                let a = poly[i], b = poly[(i + 1) % poly.count]
                edges += [a.0, a.1, b.0, b.1]
                lo = (min(lo.0, a.0), min(lo.1, a.1)); hi = (max(hi.0, a.0), max(hi.1, a.1))
            }
        }
        var f = [Double](repeating: 0, count: SF)
        f[0] = lo.0; f[1] = lo.1; f[2] = hi.0; f[3] = hi.1
        if gradient {
            for i in 0..<4 { f[4 + i] = gp[i] }
            for (i, st) in STOPS.enumerated() {
                f[8 + i * 5] = st[0]; f[9 + i * 5] = st[1]; f[10 + i * 5] = st[2]; f[11 + i * 5] = st[3]
                f[12 + i * 5] = (st[4] * opacity).rounded()
            }
        }
        sf += f
        let a = UInt32((color[3] * opacity).rounded())
        let packed = UInt32(color[0]) | UInt32(color[1]) << 8 | UInt32(color[2]) << 16 | a << 24
        su += [UInt32(start), UInt32(edges.count / 4 - start), (evenodd ? 1 : 0) | (gradient ? 2 : 0), packed]
        count += 1
    }
}

final class Gen {
    var rng: Rng
    var scene = Scene()
    let size: Double
    let profile: Profile
    init(seed: UInt64, size: Int, profile: Profile) {
        rng = Rng(s: seed ^ 0x9E37_79B9_7F4A_7C15); self.size = Double(size); self.profile = profile
    }
    func xform(_ cx: Double, _ cy: Double) -> (Double, Double, Double, Double, Double, Double) {
        let angle = rng.range(0, 6.28)
        let (s, c) = (sin(angle), cos(angle))
        let (sx, sy) = (rng.range(0.7, 1.4), rng.range(0.7, 1.4))
        return (c * sx, s * sx, -s * sy, c * sy, cx, cy)
    }
    func leaf(opacity: Double) {
        let radius = rng.range(3, min(max(size / 10, 6), 40))
        let cx = rng.range(-radius, size + radius), cy = rng.range(-radius, size + radius)
        let m = xform(cx, cy)
        func map(_ p: Pt) -> Pt { (m.0 * p.0 + m.2 * p.1 + m.4, m.1 * p.0 + m.3 * p.1 + m.5) }
        let isStroke = profile == .stroke
        let open = isStroke && rng.below(5) == 0
        var local: [Pt] = []
        if open {
            for _ in 0..<(3 + rng.below(6)) { local.append((rng.range(-radius, radius), rng.range(-radius, radius))) }
        } else {
            let n = 3 + rng.below(10)
            for i in 0..<n {
                let ang = (Double(i) + rng.range(0, 0.8)) / Double(n) * 6.2831853
                let r = radius * rng.range(0.4, 1.0)
                local.append((r * cos(ang), r * sin(ang)))
            }
        }
        let evenodd = rng.below(3) == 0
        let pts = local.map(map)
        let color = rng.color()
        if isStroke {
            let width = exp(rng.range(0, log(40.0)))  // 1..40 px, log-uniform
            scene.add(polys: strokePolys(pts, closed: !open, hw: width / 2), evenodd: false, color: color,
                      gradient: false, gp: [], opacity: opacity)
        } else if profile == .gradient {
            var lo = (Double.infinity, Double.infinity), hi = (-Double.infinity, -Double.infinity)
            for p in pts { lo = (min(lo.0, p.0), min(lo.1, p.1)); hi = (max(hi.0, p.0), max(hi.1, p.1)) }
            let w = max(hi.0 - lo.0, 1), cyy = (lo.1 + hi.1) / 2
            let gp = [lo.0, lo.1, hi.0, cyy + rng.range(-w, w) / 3]
            scene.add(polys: [pts], evenodd: evenodd, color: color, gradient: true, gp: gp, opacity: opacity)
        } else {
            scene.add(polys: [pts], evenodd: evenodd, color: color, gradient: false, gp: [], opacity: opacity)
        }
    }
    func group(depth: Int, opacityIn: Double) -> Int {
        let opacity = [1.0, 0.8, 0.5, 0.25][rng.below(4)] * opacityIn
        var made = 0
        for _ in 0..<(3 + rng.below(8)) {
            if depth < 3 && rng.below(5) == 0 { made += group(depth: depth + 1, opacityIn: opacity) }
            else { leaf(opacity: opacity); made += 1 }
        }
        return made
    }
    func run(count: Int) -> Scene {
        var made = 0
        while made < count {
            if profile == .nest && rng.below(8) == 0 { made += group(depth: 0, opacityIn: 1) }
            else { leaf(opacity: 1); made += 1 }
        }
        return scene
    }
}

/// Stroke -> polygons (CPU expansion): one quad per segment plus a round-join/cap disc per vertex.
/// All pieces share one orientation so the non-zero union is the stroke.
func strokePolys(_ pts: [Pt], closed: Bool, hw: Double) -> [[Pt]] {
    var out: [[Pt]] = []
    let n = pts.count
    let segs = closed ? n : n - 1
    for i in 0..<segs {
        let a = pts[i], b = pts[(i + 1) % n]
        let dx = b.0 - a.0, dy = b.1 - a.1
        let len = (dx * dx + dy * dy).squareRoot()
        if len < 1e-9 { continue }
        let nx = -dy / len * hw, ny = dx / len * hw
        out.append([(a.0 + nx, a.1 + ny), (b.0 + nx, b.1 + ny), (b.0 - nx, b.1 - ny), (a.0 - nx, a.1 - ny)])
    }
    let k = hw > 6 ? 16 : 8
    for p in pts {
        out.append((0..<k).map { i in
            let t = -Double(i) / Double(k) * 2 * Double.pi
            return (p.0 + hw * cos(t), p.1 + hw * sin(t))
        })
    }
    return out
}

func now() -> Double { Double(DispatchTime.now().uptimeNanoseconds) / 1e6 }
func p50(_ v: [Double]) -> Double { v.sorted()[v.count / 2] }

// MARK: CPU reference (Double, row-crossing algorithm; different from the GPU per-edge loop)
func cpuReference(_ sc: Scene, _ W: Int, _ H: Int) -> [UInt8] {
    var fb = [UInt8](repeating: 0, count: W * H * 4)
    let bandRows = 8
    let bands = (H + bandRows - 1) / bandRows
    fb.withUnsafeMutableBufferPointer { out in
        let base = out.baseAddress!
        sc.sf.withUnsafeBufferPointer { sf in sc.su.withUnsafeBufferPointer { su in sc.edges.withUnsafeBufferPointer { ed in
        DispatchQueue.concurrentPerform(iterations: bands) { b in
            let by0 = b * bandRows, by1 = min(H, by0 + bandRows)
            var cross = [[(Double, Int)]](repeating: [], count: 4)
            for s in 0..<sc.count {
                let f = sf.baseAddress! + s * SF
                let ylo = max(by0, Int(f[1].rounded(.down))), yhi = min(by1 - 1, Int(f[3].rounded(.down)))
                let xlo = max(0, Int(f[0].rounded(.down))), xhi = min(W - 1, Int(f[2].rounded(.down)))
                if ylo > yhi || xlo > xhi { continue }
                let start = Int(su[s * 4]), cnt = Int(su[s * 4 + 1]), flags = su[s * 4 + 2], packed = su[s * 4 + 3]
                let evenodd = flags & 1 != 0, grad = flags & 2 != 0
                let solid = [Int(packed & 255), Int((packed >> 8) & 255), Int((packed >> 16) & 255), Int(packed >> 24)]
                for y in ylo...yhi {
                    for sy in 0..<4 {
                        cross[sy].removeAll(keepingCapacity: true)
                        let py = Double(y) + LOC[sy]
                        for e in 0..<cnt {
                            let q = ed.baseAddress! + (start + e) * 4
                            let up = q[1] <= py && q[3] > py, dn = q[3] <= py && q[1] > py
                            if !up && !dn { continue }
                            cross[sy].append((q[0] + (py - q[1]) * (q[2] - q[0]) / (q[3] - q[1]), up ? 1 : -1))
                        }
                    }
                    if cross.allSatisfy({ $0.isEmpty }) { continue }
                    for x in xlo...xhi {
                        var alphaSum = 0, pm = [0, 0, 0]
                        for sy in 0..<4 where !cross[sy].isEmpty {
                            for sx in 0..<4 {
                                let px = Double(x) + LOC[sx]
                                var w = 0
                                for c in cross[sy] where c.0 > px { w += c.1 }
                                if !(evenodd ? (w & 1 != 0) : (w != 0)) { continue }
                                var col = solid
                                if grad { col = gradColor(f, px, Double(y) + LOC[sy]) }
                                alphaSum += col[3]
                                for c in 0..<3 { pm[c] += col[c] * col[3] }
                            }
                        }
                        if alphaSum == 0 { continue }
                        let src = [(pm[0] + alphaSum / 2) / alphaSum, (pm[1] + alphaSum / 2) / alphaSum,
                                   (pm[2] + alphaSum / 2) / alphaSum, (alphaSum + 8) / 16]
                        if src[3] == 0 { continue }
                        let o = (y * W + x) * 4
                        let da = Int(base[o + 3]), inv = 255 - src[3]
                        let oan = src[3] * 255 + da * inv
                        for c in 0..<3 {
                            let num = src[c] * src[3] * 255 + Int(base[o + c]) * da * inv
                            base[o + c] = UInt8((num + oan / 2) / oan)
                        }
                        base[o + 3] = UInt8((oan + 127) / 255)
                    }
                }
            }
        }
        } } }
    }
    return fb
}

func gradColor(_ f: UnsafePointer<Double>, _ px: Double, _ py: Double) -> [Int] {
    let dx = f[6] - f[4], dy = f[7] - f[5]
    let l2 = dx * dx + dy * dy
    var t = l2 > 0 ? ((px - f[4]) * dx + (py - f[5]) * dy) / l2 : 0
    t = min(max(t, 0), 1)
    var i = 0
    while i < 2 && t > f[8 + (i + 1) * 5] { i += 1 }
    let o0 = f[8 + i * 5], o1 = f[8 + (i + 1) * 5]
    let u = o1 > o0 ? min(max((t - o0) / (o1 - o0), 0), 1) : 0
    return (0..<4).map { c in
        let a = f[9 + i * 5 + c], b = f[9 + (i + 1) * 5 + c]
        return Int((a + (b - a) * u + 0.5).rounded(.down))
    }
}

// MARK: GPU
struct Params { var width: UInt32; var height: UInt32; var tilesX: UInt32; var tilesY: UInt32 }
struct ShapeDispatch { var shape: UInt32; var x0: UInt32; var y0: UInt32; var pad: UInt32 = 0 }

final class GPU {
    let dev = MTLCreateSystemDefaultDevice()!
    lazy var queue = dev.makeCommandQueue()!
    var tilePSO: MTLComputePipelineState!, shapePSO: MTLComputePipelineState!
    init() {
        let exe = URL(fileURLWithPath: CommandLine.arguments[0]).deletingLastPathComponent()
        let lib = try! dev.makeLibrary(URL: exe.appendingPathComponent("raster.metallib"))
        tilePSO = try! dev.makeComputePipelineState(function: lib.makeFunction(name: "tile_kernel")!)
        shapePSO = try! dev.makeComputePipelineState(function: lib.makeFunction(name: "shape_kernel")!)
    }
}

struct Timing { var upload = 0.0, bin = 0.0, gpu = 0.0, e2e = 0.0 }

final class Job {
    let g: GPU, sc: Scene, W: Int, H: Int
    let sfF: [Float], edF: [Float]
    var sfB: MTLBuffer, suB: MTLBuffer, edB: MTLBuffer, fbB: MTLBuffer, tsB: MTLBuffer, tlB: MTLBuffer
    let tilesX: Int, tilesY: Int
    var out: [UInt8]
    var tlCap: Int
    init(_ g: GPU, _ sc: Scene, _ W: Int, _ H: Int) {
        self.g = g; self.sc = sc; self.W = W; self.H = H
        sfF = sc.sf.map { Float($0) }; edF = sc.edges.map { Float($0) }
        let d = g.dev
        sfB = d.makeBuffer(length: max(sfF.count * 4, 16), options: .storageModeShared)!
        suB = d.makeBuffer(length: max(sc.su.count * 4, 16), options: .storageModeShared)!
        edB = d.makeBuffer(length: max(edF.count * 4, 16), options: .storageModeShared)!
        fbB = d.makeBuffer(length: W * H * 4, options: .storageModeShared)!
        tilesX = (W + 15) / 16; tilesY = (H + 15) / 16
        tsB = d.makeBuffer(length: (tilesX * tilesY + 1) * 4, options: .storageModeShared)!
        tlCap = 1 << 20
        tlB = d.makeBuffer(length: tlCap * 4, options: .storageModeShared)!
        out = [UInt8](repeating: 0, count: W * H * 4)
    }
    func upload() {
        sfF.withUnsafeBytes { memcpy(sfB.contents(), $0.baseAddress!, $0.count) }
        sc.su.withUnsafeBytes { memcpy(suB.contents(), $0.baseAddress!, $0.count) }
        edF.withUnsafeBytes { memcpy(edB.contents(), $0.baseAddress!, $0.count) }
    }
    func pixelBox(_ s: Int) -> (Int, Int, Int, Int)? {  // inclusive pixel bbox clipped to the canvas
        let b = s * SF
        let x0 = max(0, Int(sfF[b].rounded(.down))), y0 = max(0, Int(sfF[b + 1].rounded(.down)))
        let x1 = min(W - 1, Int(sfF[b + 2].rounded(.down))), y1 = min(H - 1, Int(sfF[b + 3].rounded(.down)))
        return x0 > x1 || y0 > y1 ? nil : (x0, y0, x1, y1)
    }
    /// CPU binning: per 16x16 tile, shape indices in z-order.
    func bin() {
        var counts = [UInt32](repeating: 0, count: tilesX * tilesY + 1)
        var boxes = [(Int, Int, Int, Int)?](repeating: nil, count: sc.count)
        for s in 0..<sc.count {
            guard let b = pixelBox(s) else { continue }
            boxes[s] = b
            for ty in (b.1 / 16)...(b.3 / 16) { for tx in (b.0 / 16)...(b.2 / 16) { counts[ty * tilesX + tx + 1] += 1 } }
        }
        for i in 1..<counts.count { counts[i] += counts[i - 1] }
        let total = Int(counts.last!)
        if total > tlCap { tlCap = total * 2; tlB = g.dev.makeBuffer(length: tlCap * 4, options: .storageModeShared)! }
        var fill = Array(counts.dropLast())
        let tl = tlB.contents().bindMemory(to: UInt32.self, capacity: tlCap)
        for s in 0..<sc.count {
            guard let b = boxes[s] else { continue }
            for ty in (b.1 / 16)...(b.3 / 16) { for tx in (b.0 / 16)...(b.2 / 16) {
                let t = ty * tilesX + tx
                tl[Int(fill[t])] = UInt32(s); fill[t] += 1
            } }
        }
        counts.withUnsafeBytes { memcpy(tsB.contents(), $0.baseAddress!, $0.count) }
    }
    func finish(_ cb: MTLCommandBuffer, _ t0: Double, _ t: inout Timing) {
        cb.commit(); cb.waitUntilCompleted()
        t.gpu = (cb.gpuEndTime - cb.gpuStartTime) * 1000
        out.withUnsafeMutableBytes { memcpy($0.baseAddress!, fbB.contents(), W * H * 4) }
        t.e2e = now() - t0
    }
    func runA() -> Timing {
        var t = Timing(); let t0 = now()
        upload(); t.upload = now() - t0
        let t1 = now(); bin(); t.bin = now() - t1
        var P = Params(width: UInt32(W), height: UInt32(H), tilesX: UInt32(tilesX), tilesY: UInt32(tilesY))
        let cb = g.queue.makeCommandBuffer()!, enc = cb.makeComputeCommandEncoder()!
        enc.setComputePipelineState(g.tilePSO)
        enc.setBuffer(sfB, offset: 0, index: 0); enc.setBuffer(suB, offset: 0, index: 1); enc.setBuffer(edB, offset: 0, index: 2)
        enc.setBuffer(tsB, offset: 0, index: 3); enc.setBuffer(tlB, offset: 0, index: 4); enc.setBuffer(fbB, offset: 0, index: 5)
        enc.setBytes(&P, length: MemoryLayout<Params>.size, index: 6)
        enc.dispatchThreadgroups(MTLSize(width: tilesX, height: tilesY, depth: 1), threadsPerThreadgroup: MTLSize(width: 16, height: 16, depth: 1))
        enc.endEncoding()
        finish(cb, t0, &t); return t
    }
    func runB() -> Timing {
        var t = Timing(); let t0 = now()
        upload(); t.upload = now() - t0
        var P = Params(width: UInt32(W), height: UInt32(H), tilesX: UInt32(tilesX), tilesY: UInt32(tilesY))
        let cb = g.queue.makeCommandBuffer()!
        let blit = cb.makeBlitCommandEncoder()!
        blit.fill(buffer: fbB, range: 0..<(W * H * 4), value: 0); blit.endEncoding()
        let enc = cb.makeComputeCommandEncoder()!
        enc.setComputePipelineState(g.shapePSO)
        enc.setBuffer(sfB, offset: 0, index: 0); enc.setBuffer(suB, offset: 0, index: 1); enc.setBuffer(edB, offset: 0, index: 2)
        enc.setBuffer(fbB, offset: 0, index: 5)
        enc.setBytes(&P, length: MemoryLayout<Params>.size, index: 6)
        for s in 0..<sc.count {
            guard let b = pixelBox(s) else { continue }
            var D = ShapeDispatch(shape: UInt32(s), x0: UInt32(b.0), y0: UInt32(b.1))
            enc.setBytes(&D, length: MemoryLayout<ShapeDispatch>.size, index: 7)
            enc.dispatchThreads(MTLSize(width: b.2 - b.0 + 1, height: b.3 - b.1 + 1, depth: 1), threadsPerThreadgroup: MTLSize(width: 8, height: 8, depth: 1))
        }
        enc.endEncoding()
        finish(cb, t0, &t); return t
    }
}

func median(_ ts: [Timing]) -> Timing {
    Timing(upload: p50(ts.map { $0.upload }), bin: p50(ts.map { $0.bin }), gpu: p50(ts.map { $0.gpu }), e2e: p50(ts.map { $0.e2e }))
}

struct Diff { var maxAbs = 0, mean = 0.0, gt0 = 0.0, gt1 = 0.0, gt2 = 0.0 }
func compare(_ a: [UInt8], _ b: [UInt8]) -> Diff {
    var d = Diff(), sum = 0, n0 = 0, n1 = 0, n2 = 0
    let px = a.count / 4
    for i in 0..<px {
        var m = 0
        for c in 0..<4 { let v = abs(Int(a[i * 4 + c]) - Int(b[i * 4 + c])); sum += v; m = max(m, v) }
        d.maxAbs = max(d.maxAbs, m)
        if m > 0 { n0 += 1 }; if m > 1 { n1 += 1 }; if m > 2 { n2 += 1 }
    }
    d.mean = Double(sum) / Double(a.count)
    d.gt0 = 100 * Double(n0) / Double(px); d.gt1 = 100 * Double(n1) / Double(px); d.gt2 = 100 * Double(n2) / Double(px)
    return d
}

// MARK: main
func arg(_ name: String, _ def: [Int]) -> [Int] {
    if let i = CommandLine.arguments.firstIndex(of: name), i + 1 < CommandLine.arguments.count {
        return CommandLine.arguments[i + 1].split(separator: ",").compactMap { Int($0) }
    }
    return def
}
let cmd = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "bench"
let sizes = arg("--sizes", [900, 1024, 2048]), counts = arg("--n", [500, 2000, 5000, 20000])
let runs = arg("--runs", [7])[0]
let profiles = Profile.allCases

if cmd == "gen" {  // gen <dir>: write seeded flat binary dumps (header u32 count, edges; then sf f64, su u32, edges f64)
    let dir = CommandLine.arguments[2]
    try? FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
    for p in profiles { for s in sizes { for n in counts {
        let sc = Gen(seed: 3, size: s, profile: p).run(count: n)
        var data = Data()
        var c = UInt32(sc.count), e = UInt32(sc.edges.count / 4)
        data.append(Data(bytes: &c, count: 4)); data.append(Data(bytes: &e, count: 4))
        sc.sf.withUnsafeBytes { data.append(contentsOf: $0) }
        sc.su.withUnsafeBytes { data.append(contentsOf: $0) }
        sc.edges.withUnsafeBytes { data.append(contentsOf: $0) }
        try! data.write(to: URL(fileURLWithPath: "\(dir)/\(p.rawValue)_\(n)_\(s).bin"))
    } } }
    print("wrote dumps to \(dir)")
} else if cmd == "bench" {
    let g = GPU()
    print("device: \(g.dev.name)")
    print("size,profile,N,edges,A_gpu_ms,A_bin_ms,A_upload_ms,A_e2e_ms,B_gpu_ms,B_e2e_ms")
    for s in sizes { for p in profiles { for n in counts {
        let sc = Gen(seed: 3, size: s, profile: p).run(count: n)
        let job = Job(g, sc, s, s)
        _ = job.runA(); _ = job.runB()
        let a = median((0..<runs).map { _ in job.runA() }), b = median((0..<runs).map { _ in job.runB() })
        print(String(format: "%d,%@,%d,%d,%.2f,%.2f,%.2f,%.2f,%.2f,%.2f", s, p.rawValue, n, sc.edges.count / 4, a.gpu, a.bin, a.upload, a.e2e, b.gpu, b.e2e))
        fflush(stdout)
    } } }
} else if cmd == "fidelity" {
    let g = GPU()
    print("size,profile,N,cpu_ref_ms,A_maxabs,A_mean,A_pct_gt0,A_pct_gt1,A_pct_gt2,B_vs_A_identical")
    for s in sizes { for p in profiles { for n in counts {
        let sc = Gen(seed: 3, size: s, profile: p).run(count: n)
        let job = Job(g, sc, s, s)
        let t0 = now(); let ref = cpuReference(sc, s, s); let cpuMs = now() - t0
        _ = job.runA(); let outA = job.out
        _ = job.runB(); let same = outA == job.out
        let d = compare(outA, ref)
        print(String(format: "%d,%@,%d,%.0f,%d,%.5f,%.4f,%.4f,%.4f,%@", s, p.rawValue, n, cpuMs, d.maxAbs, d.mean, d.gt0, d.gt1, d.gt2, same ? "yes" : "no"))
        fflush(stdout)
    } } }
}
