<template>
  <div
    class="viewer-layout relative flex flex-col w-full h-full gap-6 px-8 pb-10 pt-8"
  >
    <header data-tauri-drag-region class="relative z-10 flex items-end justify-between pt-2 flex-shrink-0">
      <div class="flex max-w-[520px] flex-col gap-1.5">
        <span class="viewer-hero__badge">Teleport Console</span>
      </div>
      <div class="flex items-center gap-4">
        <button
          @click="handleScanClick"
          :disabled="isScanning"
          class="scan-button rounded-full px-4 py-2 text-xs uppercase tracking-[0.16em] transition-colors hover:bg-white/10 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {{ isScanning ? 'Scanning...' : 'Scan' }}
        </button>
        <button
          v-if="isDevelopment"
          @click="showDebugPanel = !showDebugPanel"
          class="scan-button rounded-full px-4 py-2 text-xs uppercase tracking-[0.16em] transition-colors hover:bg-white/10"
        >
          {{ showDebugPanel ? 'Hide Debug' : 'Debug' }}
        </button>
        <div class="opacity-control flex items-center gap-3 rounded-full px-4 py-2 text-xs uppercase tracking-[0.16em]">
          <span class="opacity-control__label">Opacity</span>
          <input
            class="opacity-control__slider h-1 w-28"
            type="range"
            :min="OPACITY_MIN"
            :max="OPACITY_MAX"
            step="5"
            v-model.number="opacityPercent"
            aria-label="Window opacity"
          />
          <span class="opacity-control__value tabular-nums">{{ opacityPercent }}%</span>
        </div>
      </div>
    </header>

    <!-- Debug Panel -->
    <div v-if="showDebugPanel" class="debug-panel flex-shrink-0">
      <div class="debug-panel__header">坐标调试 (拖动标记点)</div>
      <div class="debug-panel__controls">
        <div class="debug-info" v-if="!draggedMarker">
          点击并拖动任意标记点，松开鼠标后会自动复制坐标
        </div>
        <div v-if="draggedMarker" class="debug-marker-info">
          <div class="debug-marker-name">{{ draggedMarker.name }}</div>
          <div class="debug-marker-coord">地图坐标: x={{ draggedMarker.x.toFixed(1) }}%, y={{ draggedMarker.y.toFixed(1) }}%</div>
          <textarea
            ref="coordOutput"
            class="debug-output"
            readonly
            :value="getMarkerJson(draggedMarker)"
          ></textarea>
          <button @click="copyCoordinates" class="debug-button debug-button--primary">复制JSON</button>
        </div>
      </div>
    </div>

    <MapBoard
      ref="boardRef"
      class="flex-1 min-h-0"
      :map="activeMap"
      :navigation-mode="navigationMode"
      :nav-enum="NAVIGATION_MODE"
      :selected-marker-index="selectedMarkerIndex"
      :debug-mode="showDebugPanel"
      :show-label="isDevelopment"
      :map-scale="mapScale"
      @marker-click="handleMarkerClick"
      @marker-drag-start="handleMarkerDragStart"
      @marker-drag-end="handleMarkerDragEnd"
      @marker-dragged="handleMarkerDragged"
      @reset-transform="resetTransform"
      @board-mounted="initMapInteraction"
    />

  </div>
</template>

<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import MapSelector from './components/MapSelector.vue';
import MapBoard from './components/MapBoard.vue';
import inGamePoints from './config/in_game_points.json';
import { useOpacityControl, OPACITY_MIN, OPACITY_MAX } from './hooks/useOpacityControl';
import { useGamepadNavigation, NAVIGATION_MODE } from './hooks/useGamepadNavigation';

const mapAssetManifest = import.meta.glob('../assets/**/*.{png,jpg,jpeg,webp}', {
  eager: true,
  import: 'default'
});

function resolveMapImagePath(imagePath) {
  if (typeof imagePath !== 'string' || !imagePath.trim()) {
    return '';
  }
  if (/^https?:\/\//i.test(imagePath)) {
    return imagePath;
  }
  const trimmed = imagePath.replace(/^[./\\]+/, '');
  const manifestKeys = [`../${trimmed}`];
  if (!trimmed.startsWith('assets/')) {
    manifestKeys.push(`../assets/${trimmed}`);
  }
  for (const key of manifestKeys) {
    const asset = mapAssetManifest[key];
    if (typeof asset === 'string') {
      return asset;
    }
  }
  try {
    return new URL(imagePath, import.meta.url).href;
  } catch (error) {
    console.warn('Failed to resolve map image path', imagePath, error);
    return imagePath;
  }
}

// Simple coordinate system: y and z in JSON are already map percentages (0-100)
// No complex conversion needed - just use the values directly
function convertGamePointsToMarkers(points) {
  if (!Array.isArray(points) || points.length === 0) {
    return [];
  }

  return points.map(point => {
    return {
      id: point.name,
      label: point.name,
      x: parseFloat(point.y.toFixed(1)), // y in JSON = x on map (0-100%)
      y: parseFloat(point.z.toFixed(1)), // z in JSON = y on map (0-100%)
      originalPoint: point
    };
  });
}

// Debug state
const isDevelopment = import.meta.env.DEV;
const showDebugPanel = ref(false);
const draggedMarker = ref(null);
const coordOutput = ref(null);
const originalMarkerPositions = ref(new Map()); // Track original positions: markerId -> {x, y}

// Create single map configuration from in_game_points.json
const maps = ref([
  {
    id: 'world-map',
    label: '世界地图',
    subtitle: '',
    description: '完整的游戏世界地图，显示所有保存点位置',
    image: resolveMapImagePath('/assets/WorldMinimap.png'),
    markers: convertGamePointsToMarkers(inGamePoints),
    theme: {
      '--map-image-filter': 'none'
    }
  }
]);

if (maps.value.length === 0 || maps.value[0].markers.length === 0) {
  throw new Error('Missing map configuration or markers');
}

const SCALE_MIN = 0.6;
const SCALE_MAX = 8.0;
const ALLOWED_THEME_KEYS = new Set(['--map-image-filter']);

const activeIndex = ref(0);
const statusMessage = ref('');
const navigationMode = ref(NAVIGATION_MODE.MARKER);
const selectedMarkerIndex = ref(-1);
const focusedMapIndex = ref(0);
const boardRef = ref(null);
const isScanning = ref(false);

let statusTimer = null;
function isTauriRuntimeAvailable() {
  if (typeof window === 'undefined') {
    return false;
  }
  const globalInvoke = window.__TAURI__?.core?.invoke;
  const legacyInvoke = window.__TAURI_IPC__;
  const internalInvoke = window.__TAURI_INTERNALS__?.invoke;
  return Boolean(globalInvoke || legacyInvoke || internalInvoke);
}

const { opacityPercent, initialize: initializeOpacityControl, handleOpacityChange } =
  useOpacityControl({ isTauriRuntimeAvailable });

const activeMap = computed(() => maps.value[activeIndex.value] ?? { markers: [], image: '' });
const markerCount = computed(() => {
  const markers = activeMap.value?.markers;
  return Array.isArray(markers) ? markers.length : 0;
});

watch(
  () => activeMap.value,
  (map) => {
    applyTheme(map?.theme);
  },
  { immediate: true, flush: 'post' }
);

watch(markerCount, () => {
  alignMarkerIndex();
}, { immediate: true });

watch(
  () => navigationMode.value,
  (mode) => {
    if (mode === NAVIGATION_MODE.MARKER) {
      alignMarkerIndex();
    }
  }
);

watch(
  () => activeIndex.value,
  () => {
    if (navigationMode.value === NAVIGATION_MODE.MARKER) {
      alignMarkerIndex();
    } else {
      selectedMarkerIndex.value = markerCount.value > 0 ? 0 : -1;
    }
  }
);

watch(
  opacityPercent,
  (value, oldValue) => {
    if (value !== oldValue) {
      handleOpacityChange(value);
    }
  }
);

// Watch debug panel toggle
watch(showDebugPanel, async (newValue, oldValue) => {
  if (newValue === true && oldValue === false) {
    // Debug mode opened - save original positions
    originalMarkerPositions.value.clear();
    activeMap.value.markers.forEach(marker => {
      originalMarkerPositions.value.set(marker.id, { x: marker.x, y: marker.y });
    });
  } else if (newValue === false && oldValue === true) {
    // Debug mode closed - check if any positions changed
    await saveChangedMarkerPositions();
  }
});

const { handleGamepadEvent } = useGamepadNavigation({
  navigationMode,
  markerCount,
  cycleMap,
  cycleMarker,
  triggerMarkerSelection: () => triggerMarkerSelection(),
  enterMapSelection,
  enterMarkerSelection,
  showCopyStatus
});

// Simple coordinate system: map x/y are directly stored as JSON y/z (0-100%)
function getMarkerJson(marker) {
  const original = marker.originalPoint;

  return JSON.stringify({
    i: original.i,
    name: original.name,
    x: original.x,
    y: marker.x, // Map x% -> JSON y field
    z: marker.y  // Map y% -> JSON z field
  }, null, 2);
}

async function copyCoordinates() {
  if (!draggedMarker.value) return;

  const json = getMarkerJson(draggedMarker.value);

  try {
    await navigator.clipboard.writeText(json);
    showCopyStatus('已复制到剪贴板');
  } catch (err) {
    // Fallback: select the text
    if (coordOutput.value) {
      coordOutput.value.select();
      document.execCommand('copy');
      showCopyStatus('已复制到剪贴板');
    }
  }
}

async function saveChangedMarkerPositions() {
  if (!isTauriRuntimeAvailable()) {
    console.warn('Tauri runtime not available');
    return;
  }

  // Check if any positions changed
  let hasChanges = false;
  const updatedPoints = [];

  for (const marker of activeMap.value.markers) {
    const original = originalMarkerPositions.value.get(marker.id);
    if (!original) continue;

    const xChanged = Math.abs(marker.x - original.x) > 0.01;
    const yChanged = Math.abs(marker.y - original.y) > 0.01;

    if (xChanged || yChanged) {
      hasChanges = true;
    }

    // Simple coordinate system: map x/y directly to JSON y/z
    updatedPoints.push({
      i: marker.originalPoint.i,
      name: marker.originalPoint.name,
      x: marker.originalPoint.x,
      y: marker.x, // Map x% -> JSON y field
      z: marker.y  // Map y% -> JSON z field
    });
  }

  if (!hasChanges) {
    console.log('No marker positions changed');
    return;
  }

  // Sort by index
  updatedPoints.sort((a, b) => a.i - b.i);

  const jsonStr = JSON.stringify(updatedPoints, null, 2);

  try {
    await invoke('update_in_game_points', { pointsJson: jsonStr });
    console.log('Successfully saved updated marker positions');
    showCopyStatus('已保存坐标更改');

    // Update in-memory data
    activeMap.value.markers.forEach(marker => {
      const updated = updatedPoints.find(p => p.name === marker.id);
      if (updated) {
        marker.originalPoint.y = updated.y;
        marker.originalPoint.z = updated.z;
      }
    });
  } catch (error) {
    console.error('Failed to save marker positions:', error);
    showCopyStatus('保存失败: ' + error);
  }
}

function handleMarkerDragStart() {
  isDraggingMarker = true;
}

function handleMarkerDragEnd() {
  isDraggingMarker = false;
}

function handleMarkerDragged(markerIndex, newX, newY) {
  const marker = activeMap.value.markers[markerIndex];
  if (!marker) return;

  // Update marker position
  marker.x = newX;
  marker.y = newY;

  // Set as dragged marker
  draggedMarker.value = { ...marker };

  // Auto copy
  nextTick(() => {
    copyCoordinates();
  });
}

const handleVisibilityEvent = (event) => {
  handleVisibilityChange(Boolean(event?.detail));
};

const handleGamepadDomEvent = (event) => {
  if (!event) {
    return;
  }
  handleGamepadEvent(event.detail ?? {});
};

// Track window position with debounce
let savePositionTimer = null;
const SAVE_POSITION_DEBOUNCE = 500; // 500ms debounce

async function saveWindowPosition() {
  if (!isTauriRuntimeAvailable()) {
    return;
  }

  try {
    await invoke('save_current_window_position');
    console.log('Window position saved');
  } catch (error) {
    console.error('Failed to save window position:', error);
  }
}

function handleWindowMove() {
  // Debounce saving to avoid excessive writes
  if (savePositionTimer) {
    clearTimeout(savePositionTimer);
  }
  savePositionTimer = setTimeout(() => {
    saveWindowPosition();
  }, SAVE_POSITION_DEBOUNCE);
}

// Track window resize with debounce
let saveResizeTimer = null;
const SAVE_RESIZE_DEBOUNCE = 1000; // 1 second debounce

function handleWindowResize() {
  // Reset map transform to default when window is resized
  resetTransform();

  // Debounce saving to avoid excessive writes during resize
  if (saveResizeTimer) {
    clearTimeout(saveResizeTimer);
  }
  saveResizeTimer = setTimeout(() => {
    saveWindowPosition();
  }, SAVE_RESIZE_DEBOUNCE);
}

onMounted(() => {
  resetToDefaultSelection();
  initializeOpacityControl();

  if (typeof window !== 'undefined') {
    window.addEventListener('viewer-visibility', handleVisibilityEvent);
    window.addEventListener('gamepad-event', handleGamepadDomEvent);

    // Listen for window move and resize events (Tauri-specific)
    if (isTauriRuntimeAvailable()) {
      // Save position on window blur (user moved window and clicked elsewhere)
      window.addEventListener('blur', saveWindowPosition);
      // Save position and size on window resize
      window.addEventListener('resize', handleWindowResize);
    }
  }
});

onBeforeUnmount(() => {
  if (statusTimer) {
    clearTimeout(statusTimer);
  }
  if (savePositionTimer) {
    clearTimeout(savePositionTimer);
  }
  if (saveResizeTimer) {
    clearTimeout(saveResizeTimer);
  }

  // Save position one last time before unmount
  saveWindowPosition();

  const { canvas } = getBoardElements();
  if (canvas?._cleanupMapInteraction) {
    canvas._cleanupMapInteraction();
  }
  if (typeof window !== 'undefined') {
    window.removeEventListener('viewer-visibility', handleVisibilityEvent);
    window.removeEventListener('gamepad-event', handleGamepadDomEvent);

    if (isTauriRuntimeAvailable()) {
      window.removeEventListener('blur', saveWindowPosition);
      window.removeEventListener('resize', handleWindowResize);
    }
  }
});

function getBoardElements() {
  const board = boardRef.value;
  return {
    canvas: board?.canvas ?? null,
    mapLayer: board?.mapLayer ?? null
  };
}

const mapScale = ref(1);
let mapTransform = {
  scale: 1,
  translateX: 0,
  translateY: 0
};

let isPanning = false;
let panStart = { x: 0, y: 0 };
let isDraggingMarker = false;

function initMapInteraction() {
  const { canvas, mapLayer } = getBoardElements();
  if (!canvas || !mapLayer) {
    console.warn('Canvas or map layer not found', { canvas, mapLayer });
    return;
  }

  // Wheel zoom
  const handleWheel = (e) => {
    if (isDraggingMarker) return; // Don't zoom while dragging marker

    e.preventDefault();
    const delta = e.deltaY;
    const zoomFactor = delta > 0 ? 0.9 : 1.1;

    const rect = canvas.getBoundingClientRect();
    const mouseX = e.clientX - rect.left;
    const mouseY = e.clientY - rect.top;

    const newScale = Math.max(SCALE_MIN, Math.min(SCALE_MAX, mapTransform.scale * zoomFactor));

    // Zoom towards mouse position
    const scaleRatio = newScale / mapTransform.scale;
    mapTransform.translateX = mouseX - (mouseX - mapTransform.translateX) * scaleRatio;
    mapTransform.translateY = mouseY - (mouseY - mapTransform.translateY) * scaleRatio;
    mapTransform.scale = newScale;
    mapScale.value = newScale; // Update reactive state

    applyTransform(mapLayer);
  };

  // Pan with right-click or middle-click
  const handleMouseDown = (e) => {
    // Don't pan if clicking on a marker
    if (e.target.closest('.map-marker')) {
      return;
    }
    if (e.button === 1 || e.button === 2) { // Middle or right click
      e.preventDefault();
      isPanning = true;
      panStart = { x: e.clientX, y: e.clientY };
      canvas.style.cursor = 'grabbing';
    }
  };

  const handleMouseMove = (e) => {
    if (!isPanning || isDraggingMarker) return; // Don't pan while dragging marker

    const dx = e.clientX - panStart.x;
    const dy = e.clientY - panStart.y;

    mapTransform.translateX += dx;
    mapTransform.translateY += dy;

    panStart = { x: e.clientX, y: e.clientY };

    applyTransform(mapLayer);
  };

  const handleMouseUp = () => {
    if (isPanning) {
      isPanning = false;
      const { canvas } = getBoardElements();
      if (canvas) {
        canvas.style.cursor = '';
      }
    }
  };

  // Double-click to reset
  const handleDblClick = () => {
    resetTransform();
  };

  // Prevent context menu
  const handleContextMenu = (e) => {
    e.preventDefault();
  };

  canvas.addEventListener('wheel', handleWheel, { passive: false });
  canvas.addEventListener('mousedown', handleMouseDown);
  canvas.addEventListener('mousemove', handleMouseMove);
  canvas.addEventListener('mouseup', handleMouseUp);
  canvas.addEventListener('mouseleave', handleMouseUp);
  canvas.addEventListener('dblclick', handleDblClick);
  canvas.addEventListener('contextmenu', handleContextMenu);

  // Store cleanup function
  canvas._cleanupMapInteraction = () => {
    canvas.removeEventListener('wheel', handleWheel);
    canvas.removeEventListener('mousedown', handleMouseDown);
    canvas.removeEventListener('mousemove', handleMouseMove);
    canvas.removeEventListener('mouseup', handleMouseUp);
    canvas.removeEventListener('mouseleave', handleMouseUp);
    canvas.removeEventListener('dblclick', handleDblClick);
    canvas.removeEventListener('contextmenu', handleContextMenu);
  };
}

function applyTransform(mapLayer) {
  if (!mapLayer) return;
  mapLayer.style.transform = `translate(${mapTransform.translateX}px, ${mapTransform.translateY}px) scale(${mapTransform.scale})`;
}

function resetTransform() {
  mapTransform = {
    scale: 1,
    translateX: 0,
    translateY: 0
  };
  mapScale.value = 1; // Update reactive state
  const { mapLayer } = getBoardElements();
  applyTransform(mapLayer);
}

function handleMapSelect(index) {
  setActiveMap(index, { force: true });
  navigationMode.value = NAVIGATION_MODE.MARKER;
}

function setFocusedMap(index) {
  if (!maps.value[index]) {
    return;
  }
  focusedMapIndex.value = index;
}

function setActiveMap(index, options = {}) {
  const { force = false, syncFocus = true } = options;
  if (!maps.value[index]) {
    return;
  }
  if (!force && activeIndex.value === index) {
    if (syncFocus) {
      focusedMapIndex.value = index;
    }
    return;
  }
  activeIndex.value = index;
  if (syncFocus) {
    focusedMapIndex.value = index;
  }
  selectedMarkerIndex.value = -1;
  alignMarkerIndex();
}

function alignMarkerIndex() {
  if (markerCount.value <= 0) {
    selectedMarkerIndex.value = -1;
    return;
  }
  if (selectedMarkerIndex.value >= markerCount.value) {
    selectedMarkerIndex.value = -1;
  }
}

function cycleMap(step) {
  if (!maps.value.length) {
    return;
  }
  const currentIndex = navigationMode.value === NAVIGATION_MODE.MAP ? focusedMapIndex.value : activeIndex.value;
  const next = (currentIndex + step + maps.value.length) % maps.value.length;
  if (navigationMode.value === NAVIGATION_MODE.MAP) {
    setFocusedMap(next);
    return;
  }
  setActiveMap(next);
}

function cycleMarker(step) {
  const total = markerCount.value;
  if (total <= 0) {
    showCopyStatus('No markers available for this map');
    return;
  }
  const current = selectedMarkerIndex.value < 0 ? 0 : selectedMarkerIndex.value;
  const next = (current + step + total) % total;
  selectedMarkerIndex.value = next;
}

function enterMapSelection() {
  navigationMode.value = NAVIGATION_MODE.MAP;
  setFocusedMap(activeIndex.value);
  alignMarkerIndex();
}

function enterMarkerSelection(options = {}) {
  const { reset = false } = options;
  if (!maps.value.length) {
    return;
  }
  if (focusedMapIndex.value !== activeIndex.value) {
    setActiveMap(focusedMapIndex.value, { force: true });
  }
  if (markerCount.value <= 0) {
    showCopyStatus('No markers available for this map');
    return;
  }
  navigationMode.value = NAVIGATION_MODE.MARKER;
  if (reset || selectedMarkerIndex.value < 0) {
    selectedMarkerIndex.value = 0;
  }
  alignMarkerIndex();
  setFocusedMap(activeIndex.value);
}

const hasScanned = ref(false);

async function teleportToMarker(marker = {}) {
  if (!isTauriRuntimeAvailable()) {
    return;
  }

  const savepointName = marker.id;
  if (!savepointName) {
    console.warn('Marker has no ID');
    return;
  }

  // Auto-scan if not scanned yet
  if (!hasScanned.value) {
    showCopyStatus('首次传送，正在扫描保存点...');
    try {
      await invoke('scan_save_points');
      hasScanned.value = true;
      console.log('Auto-scan completed');
    } catch (error) {
      console.error('Auto-scan failed:', error);
      showCopyStatus('扫描失败，请手动点击 Scan');
      return;
    }
  }

  try {
    const response = await invoke('teleport_to_savepoint', { savepointName });
    console.log('Teleport response:', response);

    if (response && response.startsWith('ERR')) {
      console.error('Teleport failed:', response);

      // If savepoint not found, suggest rescan
      if (response.includes('not found')) {
        showCopyStatus('保存点未找到，请点击 Scan 重新扫描');
        hasScanned.value = false;
      } else {
        showCopyStatus('传送失败: ' + response);
      }
    }
  } catch (error) {
    console.error('Teleport failed:', error);
    showCopyStatus('传送失败');
  }
}

function handleMarkerClick(markerIndex, marker) {
  if (!marker) {
    return;
  }
  selectedMarkerIndex.value = markerIndex;
  navigationMode.value = NAVIGATION_MODE.MARKER;
  teleportToMarker(marker);
  const label = typeof marker.label === 'string' && marker.label ? marker.label : null;
  if (label) {
    showCopyStatus(`传送至「${label}」`);
  } else {
    showCopyStatus('传送中...');
  }
}

function triggerMarkerSelection(index = selectedMarkerIndex.value) {
  const markers = activeMap.value?.markers ?? [];
  if (!markers.length) {
    showCopyStatus('No markers available for this map');
    return;
  }
  const targetIndex = typeof index === 'number' ? index : selectedMarkerIndex.value;
  if (targetIndex < 0 || targetIndex >= markers.length) {
    return;
  }
  handleMarkerClick(targetIndex, markers[targetIndex]);
}
function showCopyStatus(message) {
  statusMessage.value = message;
  if (statusTimer) {
    clearTimeout(statusTimer);
  }
  statusTimer = window.setTimeout(() => {
    statusMessage.value = '';
  }, 2200);
}

function applyTheme(theme = {}) {
  const root = document.documentElement;
  ALLOWED_THEME_KEYS.forEach((key) => {
    root.style.removeProperty(key);
    const value = theme?.[key];
    if (typeof value === 'string' && value && value !== 'none') {
      root.style.setProperty(key, value);
    }
  });
}

function resetToDefaultSelection() {
  navigationMode.value = NAVIGATION_MODE.MARKER;
  setActiveMap(0, { force: true });
  alignMarkerIndex();
  nextTick(() => {
    resetTransform();
  });
}

function handleVisibilityChange(isVisible) {
  if (!isVisible) {
    navigationMode.value = NAVIGATION_MODE.MAP;
    setFocusedMap(activeIndex.value);
    // Save window position when hiding
    saveWindowPosition();
    return;
  }
  navigationMode.value = NAVIGATION_MODE.MARKER;
  alignMarkerIndex();
  setFocusedMap(activeIndex.value);
  // Don't reset transform - preserve user's zoom and pan
}

async function handleScanClick() {
  if (!isTauriRuntimeAvailable()) {
    console.warn('Tauri runtime not available');
    return;
  }

  isScanning.value = true;
  try {
    const response = await invoke('scan_save_points');
    console.log('Scan response:', response);

    // Parse and log the JSON response
    try {
      const data = JSON.parse(response);
      console.log('Save points found:', data.save_points);
      console.log('Total count:', data?.save_points?.length || 0);
      hasScanned.value = true;
      showCopyStatus(`已扫描到 ${data?.save_points?.length || 0} 个保存点`);
    } catch (parseError) {
      console.warn('Failed to parse scan response:', parseError);
    }
  } catch (error) {
    console.error('Scan failed:', error);
    showCopyStatus('扫描失败');
  } finally {
    isScanning.value = false;
  }
}
</script>































