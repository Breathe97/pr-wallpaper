<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";

const canvasRef = ref<HTMLCanvasElement | null>(null);
let animationId = 0;

interface Snowflake {
  x: number;
  y: number;
  r: number;        // 半径
  speed: number;    // 下落速度
  wind: number;     // 水平飘移速度
  opacity: number;  // 透明度
}

const SNOW_COUNT = 150;

function initSnowflakes(width: number, height: number): Snowflake[] {
  const flakes: Snowflake[] = [];
  for (let i = 0; i < SNOW_COUNT; i++) {
    flakes.push({
      x: Math.random() * width,
      y: Math.random() * height,
      r: Math.random() * 4 + 1,
      speed: Math.random() * 1.5 + 0.5,
      wind: Math.random() * 0.8 - 0.4,
      opacity: Math.random() * 0.5 + 0.3,
    });
  }
  return flakes;
}

function drawSnowfall(
  ctx: CanvasRenderingContext2D,
  flakes: Snowflake[],
  width: number,
  height: number
) {
  ctx.clearRect(0, 0, width, height);

  for (const f of flakes) {
    ctx.beginPath();
    ctx.arc(f.x, f.y, f.r, 0, Math.PI * 2);
    ctx.fillStyle = `rgba(255, 255, 255, ${f.opacity})`;
    ctx.fill();
  }
}

function updateSnowflakes(flakes: Snowflake[], width: number, height: number) {
  for (const f of flakes) {
    f.y += f.speed;
    f.x += f.wind + Math.sin(f.y * 0.01) * 0.3;

    // 飘出底部后从顶部重新出现
    if (f.y - f.r > height) {
      f.y = -f.r;
      f.x = Math.random() * width;
    }
    // 飘出左右边界后从另一侧进入
    if (f.x + f.r < 0) {
      f.x = width + f.r;
    } else if (f.x - f.r > width) {
      f.x = -f.r;
    }
  }
}

function startAnimation(canvas: HTMLCanvasElement) {
  const ctx = canvas.getContext("2d")!;

  const dpr = window.devicePixelRatio || 1;

  function resize() {
    const w = window.innerWidth;
    const h = window.innerHeight;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    canvas.style.width = `${w}px`;
    canvas.style.height = `${h}px`;
    ctx.scale(dpr, dpr);
  }

  resize();
  window.addEventListener("resize", resize);

  const flakes = initSnowflakes(window.innerWidth, window.innerHeight);

  function animate() {
    const w = window.innerWidth;
    const h = window.innerHeight;
    updateSnowflakes(flakes, w, h);
    drawSnowfall(ctx, flakes, w, h);
    animationId = requestAnimationFrame(animate);
  }

  animate();

  // 返回清理函数
  return () => {
    cancelAnimationFrame(animationId);
    window.removeEventListener("resize", resize);
  };
}

let cleanup: (() => void) | undefined;

onMounted(() => {
  const canvas = canvasRef.value;
  if (!canvas) return;
  cleanup = startAnimation(canvas);
});

onUnmounted(() => {
  cleanup?.();
});
</script>

<template>
  <canvas ref="canvasRef" class="snow-canvas" />
</template>

<style scoped>
.snow-canvas {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  pointer-events: none;
  z-index: 9999;
}
</style>
