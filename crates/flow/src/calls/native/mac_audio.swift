// The call's sound on macOS, for herdr-projects' call helper (crates/flow/src/calls/native/mac.rs): the microphone with
// Apple's voice processing (echo cancelling, noise suppression) written to standard output as 16 kHz 16-bit mono PCM,
// and the voice agent's sound read from standard input and played: frames of one kind byte ("a" audio, "f" flush: stop
// what plays now), a little-endian 32-bit length and that many bytes of the same PCM. macOS asks once whether the
// terminal may use the microphone.
import AVFoundation
import Foundation

let engine = AVAudioEngine()
let input = engine.inputNode
do { try input.setVoiceProcessingEnabled(true) } catch { FileHandle.standardError.write("noaec\n".data(using: .utf8)!) }
let pcm16 = AVAudioFormat(commonFormat: .pcmFormatInt16, sampleRate: 16000, channels: 1, interleaved: true)!
let playFormat = AVAudioFormat(commonFormat: .pcmFormatFloat32, sampleRate: 16000, channels: 1, interleaved: false)!
let player = AVAudioPlayerNode()
engine.attach(player)
engine.connect(player, to: engine.mainMixerNode, format: playFormat)
let inFormat = input.outputFormat(forBus: 0)
guard let converter = AVAudioConverter(from: inFormat, to: pcm16) else { FileHandle.standardError.write("no converter\n".data(using: .utf8)!); exit(2) }
let out = FileHandle.standardOutput
input.installTap(onBus: 0, bufferSize: 1024, format: inFormat) { buffer, _ in
    let capacity = AVAudioFrameCount(Double(buffer.frameLength) * 16000 / inFormat.sampleRate + 32)
    guard let converted = AVAudioPCMBuffer(pcmFormat: pcm16, frameCapacity: capacity) else { return }
    var given = false
    var error: NSError?
    converter.convert(to: converted, error: &error) { _, status in
        if given { status.pointee = .noDataNow; return nil }
        given = true
        status.pointee = .haveData
        return buffer
    }
    let bytes = Int(converted.frameLength) * 2
    if bytes > 0, let samples = converted.int16ChannelData?[0] { out.write(Data(bytes: samples, count: bytes)) }
}
do { try engine.start() } catch { FileHandle.standardError.write("engine: \(error)\n".data(using: .utf8)!); exit(3) }
player.play()

let stdin = FileHandle.standardInput
func readExactly(_ n: Int) -> Data? {
    var data = Data()
    while data.count < n {
        let chunk = stdin.readData(ofLength: n - data.count)
        if chunk.isEmpty { return nil }
        data.append(chunk)
    }
    return data
}
while let head = readExactly(5) {
    let bytes = [UInt8](head)
    let length = Int(bytes[1]) | Int(bytes[2]) << 8 | Int(bytes[3]) << 16 | Int(bytes[4]) << 24
    if bytes[0] == UInt8(ascii: "f") { player.stop(); player.play(); continue }
    guard length > 0, let body = readExactly(length) else { if length == 0 { continue } else { break } }
    let frames = length / 2
    guard let buffer = AVAudioPCMBuffer(pcmFormat: playFormat, frameCapacity: AVAudioFrameCount(frames)) else { continue }
    buffer.frameLength = AVAudioFrameCount(frames)
    let floats = buffer.floatChannelData![0]
    body.withUnsafeBytes { raw in
        let samples = raw.bindMemory(to: Int16.self)
        for i in 0..<frames { floats[i] = Float(samples[i]) / 32768 }
    }
    player.scheduleBuffer(buffer, completionHandler: nil)
}
engine.stop()
