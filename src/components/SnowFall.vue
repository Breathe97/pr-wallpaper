<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";

const canvasRef = ref<HTMLCanvasElement | null>(null);
let animationId = 0;

// ===== 可调参数 =====
/** 雪花总数 */
const MAX_SNOW = 100;
/** 小雪强度（0~1，越低雪花越少） */
const LIGHT_INTENSITY = 0.12;
/** 大雪强度（0~1） */
const HEAVY_INTENSITY = 0.7;
/** 从小雪变为大雪的最小/最大延迟时间（毫秒） */
const TRANSITION_DELAY_MIN = 5000;
const TRANSITION_DELAY_MAX = 20000;
/** 风速变化幅度（像素/帧） */
const WIND_AMPLITUDE = 0.8;
/** 风速变化速度，越小变化越慢越平滑 */
const WIND_SPEED = 0.001;
/** 速度系数：速度 = 半径 × 该值 */
const SPEED_FACTOR = 0.3;            
// ===================

interface Snowflake {
  x: number;
  y: number;
  r: number;        // 半径
  speed: number;    // 下落速度
  windOffset: number; // 个人风偏移（让雪花横向飘移略有差异）
  opacity: number;  // 透明度
  active: boolean;  // 是否活跃（用于控制疏密）
}

function initSnowflakes(width: number, height: number): Snowflake[] {
  const flakes: Snowflake[] = [];
  for (let i = 0; i < MAX_SNOW; i++) {
    const r = Math.random() * 4 + 1;
    flakes.push({
      x: Math.random() * width,
      y: -(Math.random() * height + 50), // 全部从顶部之外开始
      r,
      speed: r * SPEED_FACTOR,            // 越大越快，越小越慢
      windOffset: Math.random() * 0.6 - 0.3,
      opacity: Math.random() * 0.5 + 0.3,
      active: true,
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
    if (!f.active) continue;
    ctx.beginPath();
    ctx.arc(f.x, f.y, f.r, 0, Math.PI * 2);
    ctx.fillStyle = `rgba(255, 255, 255, ${f.opacity})`;
    ctx.fill();
  }
}

function updateSnowflakes(
  flakes: Snowflake[],
  width: number,
  height: number,
  intensity: number,
  globalWind: number
) {
  for (const f of flakes) {
    if (!f.active) continue;

    f.y += f.speed;
    f.x += globalWind + f.windOffset + Math.sin(f.y * 0.01) * 0.3;

    // 飘出底部后根据当前强度决定是否重新出现
    if (f.y - f.r > height) {
      if (Math.random() < intensity) {
        f.y = -f.r;
        f.x = Math.random() * width;
      } else {
        f.active = false; // 强度低时暂时休眠
      }
    }
    // 飘出左右边界后从另一侧进入
    if (f.x + f.r < 0) {
      f.x = width + f.r;
    } else if (f.x - f.r > width) {
      f.x = -f.r;
    }
  }

  // 唤醒部分休眠的雪花（强度越高唤醒越多）
  for (const f of flakes) {
    if (!f.active && Math.random() < intensity * 0.02) {
      f.active = true;
      f.y = -f.r;
      f.x = Math.random() * width;
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
  let elapsed = 0;
  let isHeavy = false;
  // 随机决定何时从小雪切换到大雪
  const transitionAt = TRANSITION_DELAY_MIN + Math.random() * (TRANSITION_DELAY_MAX - TRANSITION_DELAY_MIN);
  // 随机相位偏移，让每次启动的风向都不同
  const phase1 = Math.random() * Math.PI * 2;
  const phase2 = Math.random() * Math.PI * 2;
  const phase3 = Math.random() * Math.PI * 2;

  function animate() {
    elapsed++;
    const w = window.innerWidth;
    const h = window.innerHeight;

    // 到达切换时间后变为大雪
    if (!isHeavy && elapsed * 16 >= transitionAt) {
      isHeavy = true;
    }

    // 用多个正弦波叠加模拟随机自然风（左右方向变化）
    const globalWind =
      Math.sin(elapsed * WIND_SPEED * 1.0 + phase1) * WIND_AMPLITUDE * 0.5 +
      Math.sin(elapsed * WIND_SPEED * 2.7 + phase2) * WIND_AMPLITUDE * 0.3 +
      Math.sin(elapsed * WIND_SPEED * 5.3 + phase3) * WIND_AMPLITUDE * 0.2;

    const intensity = isHeavy ? HEAVY_INTENSITY : LIGHT_INTENSITY;
    updateSnowflakes(flakes, w, h, intensity, globalWind);
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
