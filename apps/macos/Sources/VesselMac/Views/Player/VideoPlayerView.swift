import SwiftUI
import AVKit
import AppKit

public struct VideoPlayerView: View {
    @ObservedObject public var playerState = PlayerState.shared

    public init() {}

    public var body: some View {
        ZStack(alignment: .topTrailing) {
            if let avPlayer = playerState.avPlayer {
                AVPlayerViewRepresentable(player: avPlayer)
                    .edgesIgnoringSafeArea(.all)
            } else {
                Color.black.edgesIgnoringSafeArea(.all)
            }

            // Top Bar Overlay
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text(playerState.currentMedia?.title ?? "Playing Media")
                        .font(.system(size: 16, weight: .bold))
                        .foregroundColor(.white)
                    if let s = playerState.currentStream {
                        Text(s.name)
                            .font(.system(size: 12))
                            .foregroundColor(.white.opacity(0.7))
                    }
                }
                .padding(.horizontal, 16)
                .padding(.vertical, 8)
                .background(Color.black.opacity(0.6))
                .clipShape(RoundedRectangle(cornerRadius: 8))

                Spacer()

                // Full Screen Toggle
                Button(action: {
                    NSApplication.shared.keyWindow?.toggleFullScreen(nil)
                }) {
                    Image(systemName: "arrow.up.left.and.arrow.down.right")
                        .font(.system(size: 20, weight: .semibold))
                        .foregroundColor(.white)
                        .padding(8)
                        .background(Color.black.opacity(0.5))
                        .clipShape(Circle())
                }
                .buttonStyle(.plain)

                // Close Player Button
                Button(action: {
                    playerState.stop()
                }) {
                    Image(systemName: "xmark.circle.fill")
                        .font(.system(size: 26))
                        .foregroundColor(.white)
                        .shadow(radius: 4)
                }
                .buttonStyle(.plain)
            }
            .padding(20)
        }
        .frame(minWidth: 800, minHeight: 500)
    }
}

public struct AVPlayerViewRepresentable: NSViewRepresentable {
    public let player: AVPlayer

    public func makeNSView(context: Context) -> AVPlayerView {
        let view = AVPlayerView()
        view.player = player
        view.controlsStyle = .floating
        view.showsFullScreenToggleButton = true
        view.allowsPictureInPicturePlayback = true
        view.showsSharingServiceButton = true
        view.allowsVideoFrameAnalysis = true
        return view
    }

    public func updateNSView(_ nsView: AVPlayerView, context: Context) {
        if nsView.player !== player {
            nsView.player = player
        }
    }
}
