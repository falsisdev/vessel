package dev.falsis.vessel.ui.components

import android.annotation.SuppressLint
import android.graphics.Color as AndroidColor
import android.view.ViewGroup
import android.webkit.JavascriptInterface
import android.webkit.WebChromeClient
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.viewinterop.AndroidView

private val YOUTUBE_ID_REGEX = Regex(
    """(?:youtube(?:-nocookie)?\.com/(?:watch\?(?:.*&)?v=|embed/|shorts/|live/)|youtu\.be/)([A-Za-z0-9_-]{6,20})""",
    RegexOption.IGNORE_CASE,
)

fun extractYouTubeVideoId(url: String?): String? {
    val raw = url?.trim()?.takeIf { it.isNotEmpty() } ?: return null
    return YOUTUBE_ID_REGEX.find(raw)?.groupValues?.getOrNull(1)?.takeIf { it.isNotBlank() }
}

private class TrailerJsBridge(
    private val onState: (Int) -> Unit,
    private val onFailure: () -> Unit,
) {
    @JavascriptInterface
    fun onPlayerState(state: Int) {
        onState(state)
    }

    @JavascriptInterface
    fun onPlayerError() {
        onFailure()
    }
}

private fun buildYouTubeAmbientHtml(videoId: String): String = """
<!DOCTYPE html>
<html>
<head>
<meta name="viewport" content="width=device-width,initial-scale=1,maximum-scale=1,user-scalable=no">
<style>
  html, body {
    margin: 0;
    padding: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: transparent;
  }
  .stage {
    position: fixed;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
  }
  #yt-player, .stage iframe {
    position: absolute;
    top: 50%;
    left: 50%;
    width: max(100vw, 177.78vh);
    height: max(100vh, 56.25vw);
    transform: translate(-50%, -50%) scale(1.04);
    border: 0;
    pointer-events: none;
  }
</style>
</head>
<body>
  <div class="stage"><div id="yt-player"></div></div>
  <script src="https://www.youtube.com/iframe_api"></script>
  <script>
    var ytPlayer = null;
    function notifyState(s) {
      try { if (window.VesselTrailer) window.VesselTrailer.onPlayerState(s); } catch (e) {}
    }
    function notifyErr() {
      try { if (window.VesselTrailer) window.VesselTrailer.onPlayerError(); } catch (e) {}
    }
    window.onYouTubeIframeAPIReady = function() {
      try {
        ytPlayer = new YT.Player('yt-player', {
          videoId: '$videoId',
          playerVars: {
            autoplay: 1,
            mute: 1,
            controls: 0,
            disablekb: 1,
            fs: 0,
            loop: 1,
            playlist: '$videoId',
            modestbranding: 1,
            playsinline: 1,
            rel: 0,
            iv_load_policy: 3,
            origin: 'https://www.youtube-nocookie.com'
          },
          events: {
            onReady: function(ev) {
              try {
                ev.target.mute();
                ev.target.setVolume(0);
                ev.target.playVideo();
              } catch (e) {}
            },
            onStateChange: function(ev) {
              notifyState(ev.data);
              if (ev.data === 0) {
                try { ev.target.seekTo(0); ev.target.playVideo(); } catch (e) {}
              }
            },
            onError: function() { notifyErr(); }
          }
        });
      } catch (e) { notifyErr(); }
    };
  </script>
</body>
</html>
""".trimIndent()

/**
 * Silent, looping ambient trailer layer positioned behind gradients/HUD on Hero and Details.
 * Stays transparent until YouTube playback starts so the backdrop image remains visible as fallback,
 * and disables all focus/touch interaction so Android TV D-Pad navigation is never hijacked.
 */
@SuppressLint("SetJavaScriptEnabled")
@Composable
fun TrailerBackgroundLayer(
    trailerUrl: String?,
    enabled: Boolean,
    active: Boolean,
    modifier: Modifier = Modifier,
) {
    val videoId = remember(trailerUrl) { extractYouTubeVideoId(trailerUrl) }
    var playing by remember(videoId, enabled, active) { mutableStateOf(false) }
    var failed by remember(videoId, enabled, active) { mutableStateOf(false) }

    if (!enabled || !active || videoId == null || failed) {
        return
    }

    var webViewRef by remember(videoId) { mutableStateOf<WebView?>(null) }

    DisposableEffect(videoId) {
        onDispose {
            webViewRef?.let { wv ->
                runCatching {
                    wv.onPause()
                    wv.loadUrl("about:blank")
                    wv.destroy()
                }
            }
            webViewRef = null
        }
    }

    Box(modifier = modifier.fillMaxSize()) {
        AnimatedVisibility(
            visible = playing,
            enter = fadeIn(tween(650)),
            exit = fadeOut(tween(300)),
            modifier = Modifier.fillMaxSize(),
        ) {
            Box(modifier = Modifier.fillMaxSize())
        }
        AndroidView(
            modifier = Modifier.fillMaxSize(),
            factory = { ctx ->
                WebView(ctx).apply {
                    isFocusable = false
                    isFocusableInTouchMode = false
                    isClickable = false
                    isLongClickable = false
                    descendantFocusability = ViewGroup.FOCUS_BLOCK_DESCENDANTS
                    setBackgroundColor(AndroidColor.TRANSPARENT)
                    alpha = 0f
                    settings.javaScriptEnabled = true
                    settings.domStorageEnabled = true
                    settings.mediaPlaybackRequiresUserGesture = false
                    settings.loadsImagesAutomatically = true
                    settings.useWideViewPort = true
                    settings.loadWithOverviewMode = true
                    settings.cacheMode = WebSettings.LOAD_DEFAULT
                    webChromeClient = WebChromeClient()
                    webViewClient = object : WebViewClient() {
                        override fun onReceivedError(
                            view: WebView?,
                            request: WebResourceRequest?,
                            error: WebResourceError?,
                        ) {
                            if (request?.isForMainFrame == true) {
                                post { failed = true }
                            }
                        }
                    }
                    addJavascriptInterface(
                        TrailerJsBridge(
                            onState = { state ->
                                post {
                                    if (state == 1) {
                                        playing = true
                                        animate().alpha(1f).setDuration(550L).start()
                                    }
                                }
                            },
                            onFailure = { post { failed = true } },
                        ),
                        "VesselTrailer",
                    )
                    loadDataWithBaseURL(
                        "https://www.youtube-nocookie.com",
                        buildYouTubeAmbientHtml(videoId),
                        "text/html",
                        "utf-8",
                        null,
                    )
                    webViewRef = this
                }
            },
            update = { wv ->
                wv.isFocusable = false
                wv.isFocusableInTouchMode = false
                if (active) {
                    wv.onResume()
                } else {
                    wv.onPause()
                }
            },
        )
    }
}
