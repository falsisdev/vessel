const YOUTUBE_ID_REGEX =
  /(?:youtube(?:-nocookie)?\.com\/(?:watch\?(?:.*&)?v=|embed\/|shorts\/|live\/)|youtu\.be\/)([A-Za-z0-9_-]{6,20})/i

export function extractYouTubeVideoId(url?: string | null): string | null {
  const raw = url?.trim()
  if (!raw) return null
  return YOUTUBE_ID_REGEX.exec(raw)?.[1] ?? null
}

export function youtubeAmbientEmbedUrl(videoId: string): string {
  const id = encodeURIComponent(videoId)
  return `https://www.youtube-nocookie.com/embed/${id}?autoplay=1&mute=1&controls=0&disablekb=1&fs=0&loop=1&playlist=${id}&modestbranding=1&playsinline=1&rel=0&iv_load_policy=3`
}
