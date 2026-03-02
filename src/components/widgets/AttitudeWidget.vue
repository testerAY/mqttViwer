<script setup lang="ts">
import { computed, toRef } from 'vue';
import type { WidgetConfig } from '../../types/dashboard';
import type { MqttMessage } from '../../types/mqtt';
import { useWidgetData } from '../../composables/useWidgetData';
import { extractValue } from '../../utils/jsonExtractor';

const props = defineProps<{
  config: WidgetConfig;
  message: MqttMessage | undefined;
}>();

const { lastMessage } = useWidgetData(toRef(props, 'config'));

const rollKey  = computed(() => props.config.settings?.rollKey  || 'roll');
const pitchKey = computed(() => props.config.settings?.pitchKey || 'pitch');
const yawKey   = computed(() => props.config.settings?.yawKey   || 'yaw');

const parsedPayload = computed(() => {
  const payload = lastMessage.value?.payload;
  if (!payload) return null;
  try { return JSON.parse(payload); } catch { return null; }
});

const roll  = computed(() => {
  const v = parsedPayload.value ? extractValue(parsedPayload.value, rollKey.value) : null;
  return v != null ? Math.max(-90, Math.min(90, Number(v))) : 0;
});
const pitch = computed(() => {
  const v = parsedPayload.value ? extractValue(parsedPayload.value, pitchKey.value) : null;
  return v != null ? Math.max(-90, Math.min(90, Number(v))) : 0;
});
const yaw   = computed(() => {
  const v = parsedPayload.value ? extractValue(parsedPayload.value, yawKey.value) : null;
  return v != null ? ((Number(v) % 360) + 360) % 360 : 0;
});

const hasData = computed(() => parsedPayload.value !== null);

// px per degree for pitch lines; 60° fills ~90px radius
const PITCH_SCALE = 1.5;

// Horizon group transform: roll then pitch-offset
const horizonTransform = computed(
  () => `rotate(${-roll.value}, 100, 100) translate(0, ${pitch.value * PITCH_SCALE})`
);

// Bank pointer triangle rotates with roll, sits at top of circle
const bankPointerTransform = computed(
  () => `rotate(${roll.value}, 100, 100)`
);

// Pitch lines: lines at every 5°, longer at multiples of 10°
const pitchLines = computed(() => {
  const lines = [];
  for (let deg = -30; deg <= 30; deg += 5) {
    if (deg === 0) continue;
    const y = 100 - deg * PITCH_SCALE;
    const isMajor = deg % 10 === 0;
    const halfLen = isMajor ? 20 : 10;
    lines.push({ deg, y, x1: 100 - halfLen, x2: 100 + halfLen, isMajor });
  }
  return lines;
});

// Roll arc tick marks (fixed, on top of circle border)
const rollTicks = computed(() => {
  const angles = [-60, -45, -30, -20, -10, 0, 10, 20, 30, 45, 60];
  return angles.map(a => {
    const rad = (a - 90) * Math.PI / 180;
    const r1 = 86;
    const r2 = a % 30 === 0 ? 80 : 83;
    return {
      x1: 100 + r1 * Math.cos(rad),
      y1: 100 + r1 * Math.sin(rad),
      x2: 100 + r2 * Math.cos(rad),
      y2: 100 + r2 * Math.sin(rad),
      isMajor: a % 30 === 0,
    };
  });
});

const fmt = (n: number, dec = 1) => n.toFixed(dec);
</script>

<template>
  <div class="flex-1 flex flex-col items-center justify-center p-2 w-full h-full gap-1 min-h-0">
    <!-- SVG instrument -->
    <div class="flex-1 flex items-center justify-center w-full min-h-0">
      <svg
        viewBox="0 0 200 200"
        class="w-full h-full"
        style="max-width: 220px; max-height: 220px;"
      >
        <defs>
          <clipPath id="att-clip">
            <circle cx="100" cy="100" r="88" />
          </clipPath>
        </defs>

        <!-- ── Rotating horizon group ── -->
        <g :transform="horizonTransform" clip-path="url(#att-clip)">
          <!-- Sky -->
          <rect x="-200" y="-300" width="600" height="400" fill="#1a6fa8" />
          <!-- Ground -->
          <rect x="-200" y="100" width="600" height="400" fill="#7a4f28" />
          <!-- Horizon line -->
          <line x1="-200" y1="100" x2="400" y2="100" stroke="white" stroke-width="1.5" />
          <!-- Pitch reference lines -->
          <g v-for="l in pitchLines" :key="l.deg">
            <line
              :x1="l.x1" :y1="l.y" :x2="l.x2" :y2="l.y"
              stroke="white"
              :stroke-width="l.isMajor ? 1.2 : 0.8"
              opacity="0.85"
            />
            <text
              v-if="l.isMajor"
              :x="l.x2 + 3" :y="l.y + 3.5"
              fill="white" font-size="6" opacity="0.8"
            >{{ Math.abs(l.deg) }}</text>
          </g>
        </g>

        <!-- ── Roll arc tick marks (fixed) ── -->
        <g v-for="t in rollTicks" :key="`t${t.x1}`">
          <line
            :x1="t.x1" :y1="t.y1" :x2="t.x2" :y2="t.y2"
            stroke="white"
            :stroke-width="t.isMajor ? 1.5 : 1"
            opacity="0.9"
          />
        </g>

        <!-- ── Bank pointer (rotates with roll) ── -->
        <g :transform="bankPointerTransform">
          <polygon points="100,13 96,20 104,20" fill="white" opacity="0.9" />
        </g>

        <!-- ── Circle border ── -->
        <circle cx="100" cy="100" r="88" fill="none" stroke="#555" stroke-width="2" />

        <!-- ── Fixed aircraft symbol ── -->
        <g v-if="hasData">
          <!-- Left wing -->
          <rect x="60" y="98" width="28" height="4" rx="1" fill="#ffd700" />
          <!-- Right wing -->
          <rect x="112" y="98" width="28" height="4" rx="1" fill="#ffd700" />
          <!-- Center dot -->
          <circle cx="100" cy="100" r="3.5" fill="#ffd700" />
          <!-- Short tail fin indicator -->
          <rect x="98" y="106" width="4" height="8" rx="1" fill="#ffd700" />
        </g>

        <!-- No data overlay -->
        <g v-if="!hasData">
          <rect x="0" y="0" width="200" height="200" fill="black" opacity="0.5" clip-path="url(#att-clip)" />
          <text x="100" y="104" text-anchor="middle" fill="white" font-size="10" opacity="0.7">Waiting...</text>
        </g>
      </svg>
    </div>

    <!-- Digital readout -->
    <div class="flex gap-4 text-xs font-mono w-full justify-center pb-1">
      <div class="flex flex-col items-center">
        <span class="opacity-50 text-[10px]">Roll</span>
        <span class="font-bold" :class="Math.abs(roll) > 45 ? 'text-warning' : ''">
          {{ fmt(roll) }}°
        </span>
      </div>
      <div class="flex flex-col items-center">
        <span class="opacity-50 text-[10px]">Pitch</span>
        <span class="font-bold" :class="Math.abs(pitch) > 30 ? 'text-warning' : ''">
          {{ fmt(pitch) }}°
        </span>
      </div>
      <div class="flex flex-col items-center">
        <span class="opacity-50 text-[10px]">Yaw</span>
        <span class="font-bold">{{ fmt(yaw, 0) }}°</span>
      </div>
    </div>
  </div>
</template>
