<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { listen } from "@tauri-apps/api/event";

const audio = ref<HTMLAudioElement | null>(null);
const isPlaying = ref(true);

onMounted(async () => {
  const aud = new Audio("/music/music.mp3");
  aud.loop = true;
  aud.volume = 0.3;
  audio.value = aud;

  try {
    await aud.play();
  } catch {
    isPlaying.value = false;
  }

  const unlisten = await listen("music-toggle", () => {
    if (!audio.value) return;
    if (isPlaying.value) {
      audio.value.pause();
      isPlaying.value = false;
    } else {
      audio.value.play();
      isPlaying.value = true;
    }
  });

  onUnmounted(() => {
    unlisten();
    audio.value?.pause();
    audio.value = null;
  });
});
</script>

<template>
  <div style="display: none" />
</template>
