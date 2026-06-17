<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { listen } from '@tauri-apps/api/event'

const audio = ref<HTMLAudioElement | null>(null)
const isPlaying = ref(true)
const musicEnabled = ref(true)
const currentTrack = ref('music-1')

const MUSIC_FILES: Record<string, string> = {
  'music-1': '/music/music_1.mp3',
  'music-2': '/music/music_2.mp3',
}

function switchTrack(trackId: string) {
  const src = MUSIC_FILES[trackId]
  if (!src) return
  currentTrack.value = trackId
  if (!audio.value) return
  const wasPlaying = !audio.value.paused
  audio.value.src = src
  audio.value.load()
  if (wasPlaying && musicEnabled.value) {
    audio.value.play()
  }
}

onMounted(async () => {
  const aud = new Audio(MUSIC_FILES[currentTrack.value])
  aud.loop = true
  aud.volume = 0.3
  audio.value = aud

  try {
    await aud.play()
  } catch {
    isPlaying.value = false
  }

  const unlistenOff = await listen('music-off', () => {
    musicEnabled.value = false
    audio.value?.pause()
    isPlaying.value = false
  })

  const unlistenSelect = await listen<string>('music-select', (event) => {
    musicEnabled.value = true
    switchTrack(event.payload)
  })

  const unlistenPlay = await listen('music-play', () => {
    if (!audio.value || !musicEnabled.value) return
    audio.value.currentTime = 0
    audio.value.play()
    isPlaying.value = true
  })

  const unlistenStop = await listen('music-stop', () => {
    if (!audio.value) return
    audio.value.pause()
    isPlaying.value = false
  })

  onUnmounted(() => {
    unlistenOff()
    unlistenSelect()
    unlistenPlay()
    unlistenStop()
    audio.value?.pause()
    audio.value = null
  })
})
</script>

<template>
  <div style="display: none" />
</template>
