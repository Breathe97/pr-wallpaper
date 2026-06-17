<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { listen } from "@tauri-apps/api/event";

const audio = ref<HTMLAudioElement | null>(null);
const isPlaying = ref(true);
const musicEnabled = ref(true); // 用户是否勾选了"背景音乐"

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

  const unlistenToggle = await listen("music-toggle", () => {
    if (!audio.value) return;
    musicEnabled.value = !musicEnabled.value;
    if (musicEnabled.value) {
      // 重新勾选 → 从 0 播放
      audio.value.currentTime = 0;
      audio.value.play();
      isPlaying.value = true;
    } else {
      // 取消勾选 → 强制停止
      audio.value.pause();
      isPlaying.value = false;
    }
  });

  const unlistenPlay = await listen("music-play", () => {
    if (!audio.value || !musicEnabled.value) return;
    audio.value.currentTime = 0;
    audio.value.play();
    isPlaying.value = true;
  });

  const unlistenStop = await listen("music-stop", () => {
    if (!audio.value) return;
    audio.value.pause();
    isPlaying.value = false;
  });

  onUnmounted(() => {
    unlistenToggle();
    unlistenPlay();
    unlistenStop();
    audio.value?.pause();
    audio.value = null;
  });
});
</script>

<template>
  <div style="display: none" />
</template>
