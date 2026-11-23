import type { SubtitleFormat } from '@/types/vessel'

export interface SubtitleCue {
  startMs: number
  endMs: number
  lines: string[]
}

function parseTimestampToMs(raw: string): number {
  const cleaned = raw.trim().replace(',', '.')
  const parts = cleaned.split(':')
  if (parts.length < 2 || parts.length > 3) return 0

  let hours = 0
  let mins = 0
  let secsRaw = '0'
  if (parts.length === 3) {
    hours = Number.parseInt(parts[0], 10) || 0
    mins = Number.parseInt(parts[1], 10) || 0
    secsRaw = parts[2]
  } else {
    mins = Number.parseInt(parts[0], 10) || 0
    secsRaw = parts[1]
  }
  const secs = Number.parseFloat(secsRaw) || 0
  return Math.max(0, Math.round((hours * 3600 + mins * 60 + secs) * 1000))
}

function stripMarkupTags(text: string): string {
  return text
    .replace(/\{\\[^}]*\}/g, '')
    .replace(/<[^>]+>/g, '')
    .replace(/\\N/g, '\n')
    .replace(/\\n/g, '\n')
    .replace(/\\h/g, ' ')
    .trim()
}

function parseSrtOrVtt(rawText: string): SubtitleCue[] {
  const normalized = rawText.replace(/^\uFEFF/, '').replace(/\r\n?/g, '\n')
  const blocks = normalized.split(/\n{2,}/)
  const cues: SubtitleCue[] = []

  for (const block of blocks) {
    const rawLines = block.split('\n').map((l) => l.trimEnd())
    const timingIdx = rawLines.findIndex((line) => line.includes('-->'))
    if (timingIdx === -1) continue

    const timingLine = rawLines[timingIdx]
    const arrowParts = timingLine.split('-->')
    if (arrowParts.length < 2) continue

    const startMs = parseTimestampToMs(arrowParts[0])
    const endToken = arrowParts[1].trim().split(/\s+/)[0] ?? ''
    const endMs = parseTimestampToMs(endToken)
    if (endMs <= startMs) continue

    const bodyLines = rawLines
      .slice(timingIdx + 1)
      .map(stripMarkupTags)
      .flatMap((line) => line.split('\n'))
      .map((line) => line.trim())
      .filter(Boolean)

    if (bodyLines.length > 0) {
      cues.push({ startMs, endMs, lines: bodyLines })
    }
  }

  return cues.sort((a, b) => a.startMs - b.startMs)
}

function parseAssOrSsa(rawText: string): SubtitleCue[] {
  const normalized = rawText.replace(/^\uFEFF/, '').replace(/\r\n?/g, '\n')
  const cues: SubtitleCue[] = []
  let formatFields: string[] = ['Layer', 'Start', 'End', 'Style', 'Name', 'MarginL', 'MarginR', 'MarginV', 'Effect', 'Text']
  let inEvents = false

  for (const rawLine of normalized.split('\n')) {
    const line = rawLine.trim()
    if (!line) continue
    if (line.startsWith('[')) {
      inEvents = line.toLowerCase() === '[events]'
      continue
    }
    if (!inEvents && !line.startsWith('Dialogue:')) continue

    if (line.toLowerCase().startsWith('format:')) {
      formatFields = line
        .slice(7)
        .split(',')
        .map((f) => f.trim().toLowerCase())
      continue
    }

    if (line.toLowerCase().startsWith('dialogue:')) {
      const payload = line.slice(9).trim()
      const parts = payload.split(',')
      const textIdx = Math.max(formatFields.indexOf('text'), 9)
      if (parts.length <= textIdx) continue

      const startIdx = Math.max(formatFields.indexOf('start'), 1)
      const endIdx = Math.max(formatFields.indexOf('end'), 2)
      const startMs = parseTimestampToMs(parts[startIdx] ?? '')
      const endMs = parseTimestampToMs(parts[endIdx] ?? '')
      if (endMs <= startMs) continue

      const rawTextPart = parts.slice(textIdx).join(',')
      const cleanLines = stripMarkupTags(rawTextPart)
        .split('\n')
        .map((l) => l.trim())
        .filter(Boolean)
      if (cleanLines.length > 0) {
        cues.push({ startMs, endMs, lines: cleanLines })
      }
    }
  }

  return cues.sort((a, b) => a.startMs - b.startMs)
}

export function parseSubtitles(rawText: string, format?: SubtitleFormat): SubtitleCue[] {
  const trimmed = rawText.trimStart()
  if (
    format === 'ass' ||
    format === 'ssa' ||
    trimmed.includes('[Script Info]') ||
    trimmed.includes('Dialogue:')
  ) {
    return parseAssOrSsa(rawText)
  }
  return parseSrtOrVtt(rawText)
}
