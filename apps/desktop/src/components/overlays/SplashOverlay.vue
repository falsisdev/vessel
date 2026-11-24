<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'

const emit = defineEmits<{
  done: []
}>()

const overlayRef = ref<HTMLElement | null>(null)
const wavesRef = ref<SVGGElement | null>(null)
const sideRef = ref<SVGGElement | null>(null)
const bowRef = ref<SVGGElement | null>(null)
const glowRef = ref<SVGCircleElement | null>(null)

let rafId = 0

const clamp01 = (v: number): number => Math.max(0, Math.min(1, v))
const lerp = (a: number, b: number, t: number): number => a + (b - a) * t
const easeOutCubic = (t: number): number => 1 - Math.pow(1 - t, 3)
const easeInOutCubic = (t: number): number =>
  t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2

onMounted(() => {
  const prefersReduced =
    typeof window !== 'undefined' &&
    window.matchMedia?.('(prefers-reduced-motion: reduce)').matches

  const duration = prefersReduced ? 550 : 2850
  const start = performance.now()

  const frame = (now: number) => {
    const p = clamp01((now - start) / duration)

    const overlay = overlayRef.value
    const waves = wavesRef.value
    const side = sideRef.value
    const bow = bowRef.value
    const glow = glowRef.value

    if (prefersReduced) {
      if (waves) waves.setAttribute('opacity', '0')
      if (side) side.setAttribute('opacity', '0')
      if (bow) {
        const s = lerp(0.95, 1.06, easeOutCubic(p))
        bow.setAttribute('transform', `scale(${s.toFixed(3)})`)
        bow.setAttribute('opacity', '1')
      }
      if (glow) glow.setAttribute('opacity', '0.7')
      if (overlay) {
        const reveal = easeInOutCubic(clamp01((p - 0.55) / 0.45))
        overlay.style.opacity = (1 - reveal).toFixed(3)
        if (reveal > 0.15) overlay.style.pointerEvents = 'none'
      }
      if (p < 1) {
        rafId = requestAnimationFrame(frame)
      } else {
        emit('done')
      }
      return
    }

    // Phase 1 (0.00 -> 0.34): ship sails smoothly to center on waves
    const sailP = easeOutCubic(clamp01(p / 0.34))
    // Phase 2 (0.31 -> 0.49): bow 3D turn
    const turnP = easeInOutCubic(clamp01((p - 0.31) / 0.18))
    // Phase 3 (0.49 -> 0.80): settled centered hold (~0.9s) so the Vessel logo is clearly visible
    const settleP = easeOutCubic(clamp01((p - 0.43) / 0.13))
    // Phase 4 (0.80 -> 1.00): overlay dissolves directly into the live app as logo zooms subtly
    const revealP = easeInOutCubic(clamp01((p - 0.80) / 0.20))

    if (waves) {
      const waveShift = -26 * p
      const waveY = Math.sin(p * Math.PI * 3.5) * 1.4
      const waveOpacity = clamp01(1 - clamp01((p - 0.35) / 0.22))
      waves.setAttribute('transform', `translate(${waveShift.toFixed(2)}, ${waveY.toFixed(2)})`)
      waves.setAttribute('opacity', waveOpacity.toFixed(3))
    }

    if (side) {
      const sideX = lerp(-92, 0, sailP)
      const sideY = Math.sin(p * Math.PI * 5.5) * 1.7 * (1 - turnP)
      const sideScaleX = Math.max(0.04, 1 - turnP)
      const sideOpacity = p < 0.08 ? p / 0.08 : clamp01(1 - clamp01((turnP - 0.3) / 0.4))
      side.setAttribute(
        'transform',
        `translate(${sideX.toFixed(2)}, ${sideY.toFixed(2)}) scale(${sideScaleX.toFixed(3)}, 1)`,
      )
      side.setAttribute('opacity', sideOpacity.toFixed(3))
    }

    if (bow) {
      const bowScaleX = clamp01((turnP - 0.32) / 0.68)
      const settleScale = lerp(0.92, 1.0, settleP)
      const revealZoom = lerp(1.0, 1.14, revealP)
      const finalScaleX = bowScaleX * settleScale * revealZoom
      const finalScaleY = settleScale * revealZoom
      bow.setAttribute('transform', `scale(${finalScaleX.toFixed(3)}, ${finalScaleY.toFixed(3)})`)
      bow.setAttribute('opacity', bowScaleX.toFixed(3))
    }

    if (glow) {
      const glowIn = clamp01((p - 0.44) / 0.18)
      glow.setAttribute('opacity', (glowIn * 0.95).toFixed(3))
    }

    if (overlay) {
      overlay.style.opacity = (1 - revealP).toFixed(3)
      if (revealP > 0.15) {
        overlay.style.pointerEvents = 'none'
      }
    }

    if (p < 1) {
      rafId = requestAnimationFrame(frame)
    } else {
      emit('done')
    }
  }

  rafId = requestAnimationFrame(frame)
})

onUnmounted(() => {
  if (rafId) cancelAnimationFrame(rafId)
})
</script>

<template>
  <div ref="overlayRef" class="vessel-splash-overlay" aria-hidden="true">
    <svg class="splash-stage-svg" viewBox="0 0 360 220" fill="none">
      <defs>
        <radialGradient id="vesselSplashGlow">
          <stop offset="0%" stop-color="var(--vessel-accent, #cba6f7)" stop-opacity="0.24" />
          <stop offset="100%" stop-color="var(--vessel-accent, #cba6f7)" stop-opacity="0" />
        </radialGradient>
      </defs>
      <g transform="translate(180, 110)">
        <circle ref="glowRef" cx="0" cy="0" r="64" fill="url(#vesselSplashGlow)" opacity="0" />
        <g ref="wavesRef">
          <path
            d="M-125 22 Q -90 15 -55 22 T 15 22 T 85 22 T 155 22"
            stroke="var(--vessel-accent-secondary, #89b4fa)"
            stroke-width="2"
            stroke-linecap="round"
            opacity="0.58"
          />
          <path
            d="M-110 31 Q -75 25 -40 31 T 30 31 T 100 31 T 170 31"
            stroke="var(--vessel-accent, #cba6f7)"
            stroke-width="1.5"
            stroke-linecap="round"
            opacity="0.38"
          />
        </g>

        <g ref="sideRef">
          <rect x="-28" y="-18" width="22" height="10" rx="2" fill="var(--vessel-accent, #cba6f7)" />
          <rect x="-3" y="-18" width="18" height="10" rx="2" fill="var(--vessel-accent-secondary, #89b4fa)" />
          <rect x="18" y="-24" width="11" height="16" rx="2" fill="var(--vessel-text-primary, #eef1fa)" />
          <path d="M-42 -5 H40 L31 12 H-33 Z" fill="var(--vessel-text-primary, #eef1fa)" />
        </g>

        <g ref="bowRef" opacity="0">
          <g transform="scale(3.2) translate(-12, -12)">
            <rect x="8.5" y="3.5" width="7" height="3" rx="0.9" fill="var(--vessel-accent, #cba6f7)" />
            <rect x="6" y="7.5" width="5.5" height="3.2" rx="0.9" fill="var(--vessel-accent-secondary, #89b4fa)" />
            <rect
              x="12.5"
              y="7.5"
              width="5.5"
              height="3.2"
              rx="0.9"
              fill="var(--vessel-text-primary, #eef1fa)"
              opacity="0.92"
            />
            <path d="M3.5 12L12 20.8L20.5 12H16.2L12 16.4L7.8 12H3.5Z" fill="var(--vessel-text-primary, #eef1fa)" />
          </g>
        </g>
      </g>
    </svg>
  </div>
</template>

<style scoped>
.vessel-splash-overlay {
  position: fixed;
  inset: 0;
  z-index: 400;
  display: flex;
  align-items: center;
  justify-content: center;
  background:
    radial-gradient(circle at 50% 48%, rgba(203, 166, 247, 0.08) 0%, transparent 45%),
    var(--vessel-bg-base, #090a10);
  will-change: opacity;
}

.splash-stage-svg {
  width: min(56vw, 440px);
  height: auto;
  overflow: visible;
}
</style>
