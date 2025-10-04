import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export const OPACITY_MIN = 40;
export const OPACITY_MAX = 100;
export const OPACITY_DEFAULT = 100;
const OPACITY_STORAGE_KEY = 'teleport-window-opacity';

export function useOpacityControl({ isTauriRuntimeAvailable } = {}) {
  const opacityPercent = ref(OPACITY_DEFAULT);

  function clamp(value, min, max) {
    return Math.min(Math.max(value, min), max);
  }

  async function applyWindowOpacity(percent) {
    const normalized = clamp(Math.round(percent), OPACITY_MIN, OPACITY_MAX) / 100;
    let nativeApplied = false;

    if (isTauriRuntimeAvailable?.()) {
      try {
        nativeApplied = await invoke('set_window_opacity', { opacity: normalized });
      } catch (error) {
        console.warn('Window opacity update failed', error);
      }
    }

    if (typeof document !== 'undefined' && document.body) {
      document.body.classList.toggle('is-opaque', normalized >= 0.99);
      if (nativeApplied) {
        document.body.style.removeProperty('opacity');
      } else {
        document.body.style.opacity = normalized.toFixed(2);
      }
    }
  }

  async function handleOpacityChange(value) {
    const sanitized = clamp(Math.round(value), OPACITY_MIN, OPACITY_MAX);
    if (sanitized !== opacityPercent.value) {
      opacityPercent.value = sanitized;
    }

    if (typeof window !== 'undefined') {
      try {
        window.localStorage?.setItem(OPACITY_STORAGE_KEY, String(sanitized));
      } catch (error) {
        console.warn('Failed to persist window opacity', error);
      }
    }

    await applyWindowOpacity(sanitized);
  }

  async function initialize() {
    if (typeof window !== 'undefined') {
      const stored = Number.parseInt(window.localStorage?.getItem(OPACITY_STORAGE_KEY) ?? '', 10);
      if (!Number.isNaN(stored)) {
        opacityPercent.value = clamp(stored, OPACITY_MIN, OPACITY_MAX);
      }
    }
    await applyWindowOpacity(opacityPercent.value);
  }

  return {
    opacityPercent,
    initialize,
    handleOpacityChange
  };
}
