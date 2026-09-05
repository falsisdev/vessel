import Foundation
import Combine
import AVFoundation

public class PlayerState: ObservableObject {
    public static let shared = PlayerState()

    @Published public var currentMedia: MediaItem?
    @Published public var currentStream: StreamSource?
    @Published public var isPlaying: Bool = false
    @Published public var isPlayerPresented: Bool = false
    @Published public var currentTime: Double = 0.0
    @Published public var duration: Double = 0.0
    @Published public var isMuted: Bool = false

    public var avPlayer: AVPlayer?

    private init() {}

    public func play(stream: StreamSource, media: MediaItem) {
        self.currentStream = stream
        self.currentMedia = media

        guard let url = URL(string: stream.url) else { return }
        let playerItem = AVPlayerItem(url: url)
        if avPlayer == nil {
            avPlayer = AVPlayer(playerItem: playerItem)
        } else {
            avPlayer?.replaceCurrentItem(with: playerItem)
        }

        avPlayer?.play()
        self.isPlaying = true
        self.isPlayerPresented = true
    }

    public func togglePlayPause() {
        guard let p = avPlayer else { return }
        if isPlaying {
            p.pause()
            isPlaying = false
        } else {
            p.play()
            isPlaying = true
        }
    }

    public func stop() {
        avPlayer?.pause()
        avPlayer = nil
        isPlaying = false
        isPlayerPresented = false
        currentStream = nil
        currentMedia = nil
    }
}
