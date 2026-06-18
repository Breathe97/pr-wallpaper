<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { listen } from "@tauri-apps/api/event";

const canvasRef = ref<HTMLCanvasElement | null>(null);

// ===== 可调参数 =====
/** 雪花总数 */
const MAX_SNOW = 30;
/** 小雪强度（0~1，越低雪花越少） */
const LIGHT_INTENSITY = 0.12;
/** 大雪强度（0~1） */
const HEAVY_INTENSITY = 0.7;
/** 风速强度（像素/帧） */
const WIND_AMPLITUDE = 2.5;
/** 风向持续时间最小值（毫秒） */
const WIND_DIR_MS_MIN = 30000;
/** 风向持续时间最大值（毫秒） */
const WIND_DIR_MS_MAX = 180000;
/** 速度系数：速度 = 半径 × 该值 */
const SPEED_FACTOR = 0.3;
// ===================

interface Snowflake {
  x: number;
  y: number;
  r: number;
  speed: number;
  windOffset: number;
  active: boolean;
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
      active: true,
    });
  }
  return flakes;
}

// ===== WebGL2 点渲染器（gl.POINTS — 每雪花 1 顶点，97% 顶点减少） =====
class SnowGLRenderer {
  private gl: WebGL2RenderingContext;
  private program: WebGLProgram;
  private vao: WebGLVertexArrayObject;
  private vertBuffer: WebGLBuffer;
  private vertData: Float32Array;
  private resUniformLoc: WebGLUniformLocation;
  private dprUniformLoc: WebGLUniformLocation;

  constructor(canvas: HTMLCanvasElement, maxFlakes: number) {
    const gl = canvas.getContext('webgl2', {
      alpha: true,
      powerPreference: 'high-performance',
    })!;
    this.gl = gl;

    // —— 顶点着色器：每雪花仅 1 个顶点，gl_PointSize 控制大小 ——
    const vs = gl.createShader(gl.VERTEX_SHADER)!;
    gl.shaderSource(vs, `#version 300 es
      in vec2 a_pos;
      in float a_size;
      uniform vec2 u_resolution;
      uniform float u_dpr;
      void main() {
        vec2 ndc = a_pos / u_resolution * 2.0;
        ndc -= 1.0;
        ndc.y = -ndc.y;
        gl_Position = vec4(ndc, 0.0, 1.0);
        gl_PointSize = a_size * u_dpr;
      }
    `);
    gl.compileShader(vs);

    // —— 片段着色器：gl_PointCoord 裁切为圆形 ——
    const fs = gl.createShader(gl.FRAGMENT_SHADER)!;
    gl.shaderSource(fs, `#version 300 es
      precision highp float;
      out vec4 fragColor;
      void main() {
        float d = length(gl_PointCoord - vec2(0.5));
        if (d > 0.5) discard;
        fragColor = vec4(1.0);
      }
    `);
    gl.compileShader(fs);

    const prog = gl.createProgram()!;
    gl.attachShader(prog, vs);
    gl.attachShader(prog, fs);
    gl.linkProgram(prog);
    gl.useProgram(prog);
    this.program = prog;
    this.resUniformLoc = gl.getUniformLocation(prog, 'u_resolution')!;
    this.dprUniformLoc = gl.getUniformLocation(prog, 'u_dpr')!;

    // —— VAO & VBO ——
    const vao = gl.createVertexArray()!;
    gl.bindVertexArray(vao);
    this.vao = vao;

    // 每雪花 3 float: posX, posY, size (直径)
    this.vertData = new Float32Array(maxFlakes * 3);

    const buf = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, this.vertData.byteLength, gl.DYNAMIC_DRAW);
    this.vertBuffer = buf;

    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 12, 0);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(1, 1, gl.FLOAT, false, 12, 8);
    gl.enableVertexAttribArray(1);

    gl.clearColor(0, 0, 0, 0);
    gl.bindVertexArray(null);
  }

  setViewport(physicalW: number, physicalH: number, cssW: number, cssH: number) {
    const gl = this.gl;
    gl.viewport(0, 0, physicalW, physicalH);
    gl.useProgram(this.program);
    gl.uniform2f(this.resUniformLoc, cssW, cssH);
  }

  setDpr(dpr: number) {
    this.gl.useProgram(this.program);
    this.gl.uniform1f(this.dprUniformLoc, dpr);
  }

  clear() {
    this.gl.clear(this.gl.COLOR_BUFFER_BIT);
  }

  render(flakes: Snowflake[]) {
    const gl = this.gl;
    const data = this.vertData;
    let idx = 0;

    for (const f of flakes) {
      if (!f.active) continue;
      const base = idx * 3;
      data[base] = f.x;
      data[base + 1] = f.y;
      data[base + 2] = f.r * 2; // 直径（CSS 像素）
      idx++;
    }

    if (idx === 0) { this.clear(); return; }

    gl.bindBuffer(gl.ARRAY_BUFFER, this.vertBuffer);
    gl.bufferSubData(gl.ARRAY_BUFFER, 0, data, 0, idx * 3);
    gl.bindVertexArray(this.vao);
    this.clear();
    gl.drawArrays(gl.POINTS, 0, idx);
    gl.bindVertexArray(null);
  }

  resize(canvas: HTMLCanvasElement, w: number, h: number, dpr: number) {
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    canvas.style.width = `${w}px`;
    canvas.style.height = `${h}px`;
    this.setViewport(canvas.width, canvas.height, w, h);
    this.setDpr(dpr);
  }
}
// ==============================================

// ===== 优化: 随机数池（减少 Math.random() 调用频率） =====
const RAND_POOL_SIZE = 1024;
const randPool = new Float64Array(RAND_POOL_SIZE);
let randIdx = 0;

function fastRand(): number {
  randIdx = (randIdx + 1) & (RAND_POOL_SIZE - 1);
  return randPool[randIdx];
}

function refillRandPool() {
  for (let i = 0; i < RAND_POOL_SIZE; i++) {
    randPool[i] = Math.random();
  }
}
refillRandPool();
// ====================================================

// ===== 优化: swayPhase 替代逐雪花 sin 计算 =====
function updateSnowflakes(
  flakes: Snowflake[], width: number, height: number,
  intensity: number, globalWind: number, swayPhase: number
) {
  for (const f of flakes) {
    if (!f.active) continue;
    f.y += f.speed;
    f.x += globalWind + f.windOffset + Math.sin(swayPhase + f.windOffset * Math.PI) * 0.6;
    if (f.y - f.r > height) {
      if (fastRand() < intensity) {
        f.y = -f.r;
        f.x = fastRand() * width;
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
    if (!f.active && fastRand() < intensity * 0.02) {
      f.active = true;
      f.y = -f.r;
      f.x = fastRand() * width;
    }
  }
}
// =================================================

function startAnimation(canvas: HTMLCanvasElement) {
  const dpr = window.devicePixelRatio || 1;
  const renderer = new SnowGLRenderer(canvas, MAX_SNOW);
  let localAnimId = 0;

  function resize() {
    const w = window.innerWidth;
    const h = window.innerHeight;
    renderer.resize(canvas, w, h, dpr);
  }

  resize();
  window.addEventListener("resize", resize);

  const flakes = initSnowflakes(window.innerWidth, window.innerHeight);
  let frame = 0;
  /** 每 N 帧渲染一次，减少 GPU 负载 */
  const RENDER_INTERVAL = 2;

  // ===== 风向状态（指数平滑过渡） =====
  const WIND_SMOOTHING = 0.025;
  let windTimer = 0;
  let windDuration = 0;
  let targetWind = (Math.random() > 0.5 ? 1 : -1) * (0.5 + Math.random() * 0.5) * WIND_AMPLITUDE;
  let currentWind = targetWind;
  function pickWind() {
    const dir = Math.random() > 0.5 ? 1 : -1;
    const strength = 0.5 + Math.random() * 0.5;
    targetWind = dir * strength * WIND_AMPLITUDE;
    windDuration = Math.floor(
      (WIND_DIR_MS_MIN + Math.random() * (WIND_DIR_MS_MAX - WIND_DIR_MS_MIN)) / 16
    );
    windTimer = windDuration;
  }
  pickWind();
  // 随机强度波动相位
  const randomPhase = Math.random() * Math.PI * 2;

  function animate() {
    frame++;
    const w = window.innerWidth;
    const h = window.innerHeight;
    windTimer--;
    if (windTimer <= 0) pickWind();
    currentWind += (targetWind - currentWind) * WIND_SMOOTHING;
    const globalWind = currentWind;
    // 风力越强，雪花密度越大
    const windAbs = Math.abs(globalWind) / WIND_AMPLITUDE;
    // 叠加随机波动 (±0.15)
    const randomMod = Math.sin(frame * 0.005 + randomPhase) * 0.15;
    const intensity = Math.max(0, Math.min(1, LIGHT_INTENSITY + windAbs * (HEAVY_INTENSITY - LIGHT_INTENSITY) + randomMod));
    const swayPhase = frame * 0.01;
    updateSnowflakes(flakes, w, h, intensity, globalWind, swayPhase);
    if (frame % RENDER_INTERVAL === 0) {
      renderer.render(flakes);
    }
    if (frame % 256 === 0) {
      refillRandPool();
    }
    localAnimId = requestAnimationFrame(animate);
  }

  animate();

  return () => {
    cancelAnimationFrame(localAnimId);
    window.removeEventListener("resize", resize);
    renderer.clear();
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
  will-change: transform;
  transform: translateZ(0);
}
</style>
