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
      active: true,
      shape: generateIrregularShape(r),
    });
  }
  return flakes;
}

// ===== WebGL2 渲染器（调用独立显卡加速） =====
class SnowGLRenderer {
  private gl: WebGL2RenderingContext;
  private program: WebGLProgram;
  private vao: WebGLVertexArrayObject;
  private vertBuffer: WebGLBuffer;
  private vertCapacity: number;
  private vertStride = 4; // floats/vertex: localX, localY, worldX, worldY
  private vertData: Float32Array;
  private resUniformLoc: WebGLUniformLocation;

  constructor(canvas: HTMLCanvasElement, maxFlakes: number) {
    const gl = canvas.getContext('webgl2', {
      alpha: true,
      powerPreference: 'high-performance',
    })!;
    this.gl = gl;

    // —— 顶点着色器 ——
    const vs = gl.createShader(gl.VERTEX_SHADER)!;
    gl.shaderSource(vs, `#version 300 es
      in vec2 a_local;
      in vec2 a_world;
      uniform vec2 u_resolution;
      void main() {
        vec2 pos = a_local + a_world;
        vec2 ndc = pos / u_resolution * 2.0;
        ndc -= 1.0;
        ndc.y = -ndc.y;
        gl_Position = vec4(ndc, 0.0, 1.0);
      }
    `);
    gl.compileShader(vs);

    // —— 片段着色器 ——
    const fs = gl.createShader(gl.FRAGMENT_SHADER)!;
    gl.shaderSource(fs, `#version 300 es
      precision highp float;
      out vec4 fragColor;
      void main() { fragColor = vec4(1.0); }
    `);
    gl.compileShader(fs);

    const prog = gl.createProgram()!;
    gl.attachShader(prog, vs);
    gl.attachShader(prog, fs);
    gl.linkProgram(prog);
    gl.useProgram(prog);
    this.program = prog;
    this.resUniformLoc = gl.getUniformLocation(prog, 'u_resolution')!;

    // —— VAO & VBO ——
    const vao = gl.createVertexArray()!;
    gl.bindVertexArray(vao);
    this.vao = vao;

    // 预分配缓存：maxFlakes × 10 三角形 × 3 顶点 × 4 float
    const vertsPerFlake = 10 * 3;
    this.vertCapacity = maxFlakes * vertsPerFlake * this.vertStride;
    this.vertData = new Float32Array(this.vertCapacity);

    const buf = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, this.vertData.byteLength, gl.DYNAMIC_DRAW);
    this.vertBuffer = buf;

    const fs2 = 4;
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, this.vertStride * fs2, 0);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(1, 2, gl.FLOAT, false, this.vertStride * fs2, 2 * fs2);
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

  clear() {
    this.gl.clear(this.gl.COLOR_BUFFER_BIT);
  }

  render(flakes: Snowflake[]) {
    const gl = this.gl;
    const data = this.vertData;
    let idx = 0;

    for (const f of flakes) {
      if (!f.active) continue;
      const pts = f.shape;
      const n = pts.length;
      const bx = f.x;
      const by = f.y;
      // 三角形扇：v0,v1,v2 → v0,v2,v3 → ... → v0,v_{n-2},v_{n-1}
      const ax = pts[0].x, ay = pts[0].y;
      for (let i = 1; i < n - 1; i++) {
        const b0 = idx * 4;
        data[b0] = ax; data[b0 + 1] = ay;
        data[b0 + 2] = bx; data[b0 + 3] = by;
        const b1 = (idx + 1) * 4;
        data[b1] = pts[i].x; data[b1 + 1] = pts[i].y;
        data[b1 + 2] = bx; data[b1 + 3] = by;
        const b2 = (idx + 2) * 4;
        data[b2] = pts[i + 1].x; data[b2 + 1] = pts[i + 1].y;
        data[b2 + 2] = bx; data[b2 + 3] = by;
        idx += 3;
      }
    }

    gl.bindBuffer(gl.ARRAY_BUFFER, this.vertBuffer);
    gl.bufferSubData(gl.ARRAY_BUFFER, 0, data, 0, idx * 4);
    gl.bindVertexArray(this.vao);
    this.clear();
    gl.drawArrays(gl.TRIANGLES, 0, idx);
    gl.bindVertexArray(null);
  }

  resize(canvas: HTMLCanvasElement, w: number, h: number, dpr: number) {
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    canvas.style.width = `${w}px`;
    canvas.style.height = `${h}px`;
    this.setViewport(canvas.width, canvas.height, w, h);
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

  // ===== 风向状态 =====
  let windDir = Math.random() > 0.5 ? 1 : -1;
  let windStrength = 0.5 + Math.random() * 0.5;
  let windTimer = 0;
  let windDuration = 0;
  function pickWind() {
    windDir = Math.random() > 0.5 ? 1 : -1;
    windStrength = 0.5 + Math.random() * 0.5;
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
    const globalWind = windDir * windStrength * WIND_AMPLITUDE;
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
