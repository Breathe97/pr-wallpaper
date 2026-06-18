<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { listen } from "@tauri-apps/api/event";

const canvasRef = ref<HTMLCanvasElement | null>(null);

// ===== 可调参数 =====
/** 雪花总数 */
const MAX_SNOW = 60;
/** 小雪强度（0~1，越低雪花越少） */
const LIGHT_INTENSITY = 0.12;
/** 大雪强度（0~1） */
const HEAVY_INTENSITY = 0.7;
/** 风速变化幅度（像素/帧） */
const WIND_AMPLITUDE = 0.8;
/** 风速变化速度，越小变化越慢越平滑 */
const WIND_SPEED = 0.001;
/** 速度系数：速度 = 半径 × 该值 */
const SPEED_FACTOR = 0.3;
// ===================

interface Point {
  x: number;
  y: number;
}

interface Snowflake {
  x: number;
  y: number;
  r: number;
  speed: number;
  windOffset: number;
  opacity: number;
  active: boolean;
  /** 不规则形状顶点偏移（相对于圆心，已按半径缩放） */
  shape: Point[];
}

/** 生成不规则形状的顶点（8~12 个点，半径波动 0.6~1.0） */
function generateIrregularShape(baseR: number): Point[] {
  const count = Math.floor(Math.random() * 5) + 8; // 8~12
  const points: Point[] = [];
  for (let i = 0; i < count; i++) {
    const angle = (i / count) * Math.PI * 2;
    const rFactor = 0.6 + Math.random() * 0.4; // 0.6~1.0 随机波动
    const r = baseR * rFactor;
    points.push({ x: Math.cos(angle) * r, y: Math.sin(angle) * r });
  }
  return points;
}

function initSnowflakes(width: number, height: number): Snowflake[] {
  const flakes: Snowflake[] = [];
  for (let i = 0; i < MAX_SNOW; i++) {
    const r = Math.random() * 4 + 1;
    flakes.push({
      x: Math.random() * width,
      y: -(Math.random() * height + 50),
      r,
      speed: r * SPEED_FACTOR,
      windOffset: Math.random() * 0.6 - 0.3,
      opacity: 1,
      active: true,
      shape: generateIrregularShape(r),
    });
  }
  return flakes;
}

function drawSnowfall(ctx: CanvasRenderingContext2D, flakes: Snowflake[], width: number, height: number) {
  ctx.clearRect(0, 0, width, height);
  const path = new Path2D();
  for (const f of flakes) {
    if (!f.active) continue;
    const pts = f.shape;
    path.moveTo(f.x + pts[0].x, f.y + pts[0].y);
    for (let i = 1; i < pts.length; i++) {
      path.lineTo(f.x + pts[i].x, f.y + pts[i].y);
    }
    path.closePath();
  }
  ctx.fillStyle = "#fff";
  ctx.fill(path);
}

function updateSnowflakes(
  flakes: Snowflake[], width: number, height: number, intensity: number, globalWind: number
) {
  for (const f of flakes) {
    if (!f.active) continue;
    f.y += f.speed;
    f.x += globalWind + f.windOffset + Math.sin(f.y * 0.01) * 0.3;
    if (f.y - f.r > height) {
      if (Math.random() < intensity) {
        f.y = -f.r;
        f.x = Math.random() * width;
      } else {
        f.active = false;
      }
    }
    if (f.x + f.r < 0) {
      f.x = width + f.r;
    } else if (f.x - f.r > width) {
      f.x = -f.r;
    }
  }
  for (const f of flakes) {
    if (!f.active && Math.random() < intensity * 0.02) {
      f.active = true;
      f.y = -f.r;
      f.x = Math.random() * width;
    }
  }
}

function startAnimation(canvas: HTMLCanvasElement) {
  const ctx = canvas.getContext("2d", { willReadFrequently: false })!;
  const dpr = window.devicePixelRatio || 1;
  let localAnimId = 0;

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
  let frame = 0;
  /** 每 N 帧渲染一次，减少 GPU 负载 */
  const RENDER_INTERVAL = 2;
  const phase1 = Math.random() * Math.PI * 2;
  const phase2 = Math.random() * Math.PI * 2;
  const phase3 = Math.random() * Math.PI * 2;
  // 随机强度波动相位，使雪花密度变化不可预测
  const randomPhase = Math.random() * Math.PI * 2;

  function animate() {
    frame++;
    const w = window.innerWidth;
    const h = window.innerHeight;
    const globalWind =
      Math.sin(frame * WIND_SPEED * 1.0 + phase1) * WIND_AMPLITUDE * 0.5 +
      Math.sin(frame * WIND_SPEED * 2.7 + phase2) * WIND_AMPLITUDE * 0.3 +
      Math.sin(frame * WIND_SPEED * 5.3 + phase3) * WIND_AMPLITUDE * 0.2;
    // 风力越强，雪花密度越大（风绝对值 0~1 映射到 LIGHT~HEAVY）
    const windStrength = Math.abs(globalWind) / WIND_AMPLITUDE;
    // 叠加随机波动 (±0.15)，让密度变化更有不可预测感
    const randomMod = Math.sin(frame * 0.005 + randomPhase) * 0.15;
    const intensity = Math.max(0, Math.min(1, LIGHT_INTENSITY + windStrength * (HEAVY_INTENSITY - LIGHT_INTENSITY) + randomMod));
    updateSnowflakes(flakes, w, h, intensity, globalWind);
    if (frame % RENDER_INTERVAL === 0) {
      drawSnowfall(ctx, flakes, w, h);
    }
    localAnimId = requestAnimationFrame(animate);
  }

  // 延迟 2 秒启动雪花，等音乐
  const startTimeout = setTimeout(() => {
    animate();
  }, 1000);

  return () => {
    clearTimeout(startTimeout);
    cancelAnimationFrame(localAnimId);
    window.removeEventListener("resize", resize);
  };
}

let cleanup: (() => void) | undefined;
let running = false;
let unlistenStop: (() => void) | undefined;
let unlistenRestart: (() => void) | undefined;

onMounted(async () => {
  const canvas = canvasRef.value;
  if (!canvas) return;

  cleanup = startAnimation(canvas);
  running = true;

  unlistenStop = await listen("snow-stop", () => {
    if (running) {
      cleanup?.();
      cleanup = undefined;
      running = false;
      // 立即清除画布，避免残留
      const ctx = canvasRef.value?.getContext("2d");
      if (ctx && canvasRef.value) {
        ctx.clearRect(0, 0, canvasRef.value.width, canvasRef.value.height);
      }
    }
  });

  unlistenRestart = await listen("snow-restart", () => {
    if (!running && canvasRef.value) {
      cleanup = startAnimation(canvasRef.value);
      running = true;
    }
  });
});

onUnmounted(() => {
  unlistenStop?.();
  unlistenRestart?.();
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
