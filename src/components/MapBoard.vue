<template>
  <section class="map-board relative z-10">
    <!-- 操作提示 -->
    <div class="map-board__hint">
      <span class="hint-text">滚轮缩放</span>
      <span class="hint-divider">•</span>
      <span class="hint-text">右键/中键平移</span>
      <span class="hint-divider">•</span>
      <span class="hint-text">双击重置</span>
      <span class="hint-divider">•</span>
      <span class="hint-text">点击传送</span>
    </div>

    <!-- 地图画布 -->
    <div class="map-canvas" ref="canvas">
      <div class="map-layer" ref="mapLayer">
        <img
          :src="imageSrc"
          alt="map"
          class="map-image"
          draggable="false"
          ref="image"
        />
        <button
          v-for="(marker, markerIndex) in markers"
          :key="marker.id"
          :class="['map-marker',
                   { 'is-selected': navigationMode === navEnum.MARKER && markerIndex === selectedMarkerIndex },
                   { 'is-draggable': debugMode }]"
          :style="{ left: marker.x + '%', top: marker.y + '%' }"
          @click="handleMarkerInteraction($event, markerIndex, marker)"
          @mousedown="debugMode ? startDrag($event, markerIndex, marker) : null"
        >
          <span class="map-marker__core"></span>
          <span class="map-marker__label">{{ marker.label }}</span>
        </button>
      </div>

      <!-- 角落装饰 -->
      <div class="map-canvas__corner map-canvas__corner--tl"></div>
      <div class="map-canvas__corner map-canvas__corner--tr"></div>
      <div class="map-canvas__corner map-canvas__corner--bl"></div>
      <div class="map-canvas__corner map-canvas__corner--br"></div>
    </div>
  </section>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';

const props = defineProps({
  map: {
    type: Object,
    default: () => ({ markers: [], image: '' })
  },
  navigationMode: {
    type: String,
    default: ''
  },
  selectedMarkerIndex: {
    type: Number,
    default: -1
  },
  navEnum: {
    type: Object,
    required: true
  },
  debugMode: {
    type: Boolean,
    default: false
  }
});

const emit = defineEmits(['marker-click', 'marker-drag-start', 'marker-drag-end', 'marker-dragged', 'reset-transform', 'board-mounted']);

const canvas = ref(null);
const mapLayer = ref(null);
const image = ref(null);

const markers = computed(() =>
  Array.isArray(props.map?.markers) ? props.map.markers : []
);
const imageSrc = computed(() => props.map?.image ?? '');

const dragging = ref(false);
const dragMarkerIndex = ref(-1);

function handleMarkerInteraction(event, markerIndex, marker) {
  if (props.debugMode) {
    return; // In debug mode, dragging is handled by mousedown
  }
  event.preventDefault();
  event.stopPropagation();
  emit('marker-click', markerIndex, marker);
}

function startDrag(event, index, marker) {
  if (!props.debugMode) return;

  event.preventDefault();
  event.stopPropagation();

  dragging.value = true;
  dragMarkerIndex.value = index;

  emit('marker-drag-start');

  const currentMarker = markers.value[index];

  const onMouseMove = (e) => {
    if (!dragging.value) return;

    // Recalculate imageRect each move for zoom/pan
    const imageRect = image.value.getBoundingClientRect();
    const x = ((e.clientX - imageRect.left) / imageRect.width) * 100;
    const y = ((e.clientY - imageRect.top) / imageRect.height) * 100;

    const clampedX = Math.max(0, Math.min(100, x));
    const clampedY = Math.max(0, Math.min(100, y));

    // Update marker position directly (Vue 3 reactive)
    currentMarker.x = parseFloat(clampedX.toFixed(1));
    currentMarker.y = parseFloat(clampedY.toFixed(1));
  };

  const onMouseUp = () => {
    if (dragging.value) {
      emit('marker-dragged', dragMarkerIndex.value, currentMarker.x, currentMarker.y);
      emit('marker-drag-end');
    }

    dragging.value = false;
    dragMarkerIndex.value = -1;

    document.removeEventListener('mousemove', onMouseMove);
    document.removeEventListener('mouseup', onMouseUp);
  };

  document.addEventListener('mousemove', onMouseMove);
  document.addEventListener('mouseup', onMouseUp);
}

onMounted(() => {
  // Emit mounted event to parent
  emit('board-mounted');
});

onBeforeUnmount(() => {
  // Cleanup is handled in App.vue
});

defineExpose({ canvas, mapLayer, image });
</script>

<style scoped>
.map-board {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.map-board__hint {
  position: absolute;
  top: 20px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 100;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 16px;
  background: rgba(10, 15, 30, 0.8);
  border: 1px solid var(--accent-primary);
  border-radius: 4px;
  font-size: 14px;
  color: var(--text-secondary);
}

.hint-divider {
  color: var(--accent-primary);
  opacity: 0.5;
}

.map-canvas {
  position: relative;
  flex: 1;
  overflow: hidden;
  background: radial-gradient(
    ellipse at center,
    rgba(20, 30, 60, 0.3) 0%,
    rgba(10, 15, 30, 0.6) 100%
  );
}

.map-layer {
  position: relative;
  width: 100%;
  height: 100%;
  transform-origin: 0 0;
}

.map-image {
  position: absolute;
  width: 100%;
  height: 100%;
  object-fit: contain;
  user-select: none;
  pointer-events: none;
}

.map-marker {
  position: absolute;
  transform: translate(-50%, -50%);
  cursor: pointer;
  z-index: 10;
  background: transparent;
  border: none;
  padding: 0;
  outline: none;
  transition: filter 0.2s ease;
}

.map-marker:hover {
  filter: brightness(1.3);
}

.map-marker.is-draggable {
  cursor: move !important;
}

.map-marker.is-draggable .map-marker__core {
  border-color: #ffaa00;
  box-shadow: 0 0 10px rgba(255, 170, 0, 0.6);
}

.map-marker__core {
  display: block;
  width: 16px;
  height: 16px;
  border: 2px solid var(--accent-primary);
  border-radius: 50%;
  background: rgba(0, 255, 255, 0.2);
  box-shadow: 0 0 10px rgba(0, 255, 255, 0.5);
  transition: all 0.2s ease;
}

.map-marker.is-selected .map-marker__core {
  border-color: #ffffff;
  background: rgba(255, 255, 255, 0.3);
  box-shadow: 0 0 15px rgba(255, 255, 255, 0.8);
}

.map-marker__label {
  position: absolute;
  top: 100%;
  left: 50%;
  transform: translateX(-50%);
  margin-top: 4px;
  padding: 2px 6px;
  background: rgba(0, 0, 0, 0.7);
  border-radius: 3px;
  font-size: 12px;
  color: var(--text-primary);
  white-space: nowrap;
  pointer-events: none;
}

.map-canvas__corner {
  position: absolute;
  width: 20px;
  height: 20px;
  border: 2px solid var(--accent-primary);
  opacity: 0.3;
  pointer-events: none;
}

.map-canvas__corner--tl {
  top: 10px;
  left: 10px;
  border-right: none;
  border-bottom: none;
}

.map-canvas__corner--tr {
  top: 10px;
  right: 10px;
  border-left: none;
  border-bottom: none;
}

.map-canvas__corner--bl {
  bottom: 10px;
  left: 10px;
  border-right: none;
  border-top: none;
}

.map-canvas__corner--br {
  bottom: 10px;
  right: 10px;
  border-left: none;
  border-top: none;
}
</style>
