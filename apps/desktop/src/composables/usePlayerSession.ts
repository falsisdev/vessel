import { ref } from 'vue'
import type { Episode, MediaDetail, StreamCandidate, SubtitleTrack } from '@/types/vessel'

export interface PlayerSession {
  pluginId: string
  nativeId: string
  detail: MediaDetail
  episode: Episode
  streams: StreamCandidate[]
  subtitles: SubtitleTrack[]
  activeStreamIndex: number
  startPositionMs?: number
  channelStreams?: Record<string, StreamCandidate>
}

const activeSession = ref<PlayerSession | null>(null)

export function usePlayerSession() {
  function setSession(session: PlayerSession): void {
    activeSession.value = session
  }

  function updateEpisodeSession(
    episode: Episode,
    streams: StreamCandidate[],
    subtitles: SubtitleTrack[],
    activeStreamIndex = 0,
  ): void {
    if (!activeSession.value) return
    activeSession.value = {
      ...activeSession.value,
      episode,
      streams,
      subtitles,
      activeStreamIndex,
      startPositionMs: 0,
    }
  }

  function setActiveStreamIndex(index: number): void {
    if (!activeSession.value) return
    activeSession.value = {
      ...activeSession.value,
      activeStreamIndex: index,
    }
  }

  function clearSession(): void {
    activeSession.value = null
  }

  return {
    activeSession,
    setSession,
    updateEpisodeSession,
    setActiveStreamIndex,
    clearSession,
  }
}
