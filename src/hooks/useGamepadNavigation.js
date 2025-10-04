import { ref } from 'vue';

const NAVIGATION_MODE = Object.freeze({
  MAP: 'map',
  MARKER: 'marker'
});

export function useGamepadNavigation({
  navigationMode,
  markerCount,
  cycleMap,
  cycleMarker,
  triggerMarkerSelection,
  enterMapSelection,
  enterMarkerSelection,
  showCopyStatus
}) {
  const axisLatch = ref(0);

  function handleDirectionalInput(direction) {
    if (!direction) {
      return;
    }
    const step = direction > 0 ? 1 : -1;
    if (navigationMode.value === NAVIGATION_MODE.MAP) {
      cycleMap(step);
    } else {
      if ((markerCount.value ?? 0) <= 0) {
        showCopyStatus?.('No markers available for this map');
        return;
      }
      cycleMarker(step);
    }
  }

  function handleButtonA() {
    if (navigationMode.value === NAVIGATION_MODE.MAP) {
      enterMarkerSelection?.({ reset: true });
    } else if (navigationMode.value === NAVIGATION_MODE.MARKER) {
      triggerMarkerSelection?.();
    }
  }

  function handleButtonB() {
    if (navigationMode.value !== NAVIGATION_MODE.MAP) {
      enterMapSelection?.();
    }
  }

  function handleGamepadEvent(payload) {
    if (!payload || typeof payload !== 'object') {
      return;
    }
    if (payload.type === 'button') {
      if (payload.state !== 'pressed') {
        return;
      }
      switch (payload.button) {
        case 'A':
          handleButtonA();
          break;
        case 'B':
          handleButtonB();
          break;
        case 'DPAD_LEFT':
          handleDirectionalInput(-1);
          break;
        case 'DPAD_RIGHT':
          handleDirectionalInput(1);
          break;
        default:
          break;
      }
      return;
    }
    if (payload.type === 'axis' && payload.axis === 'left_x') {
      const direction = Number(payload.direction ?? 0);
      if (Number.isNaN(direction) || direction === 0) {
        axisLatch.value = 0;
        return;
      }
      const normalized = direction > 0 ? 1 : -1;
      if (axisLatch.value !== normalized) {
        axisLatch.value = normalized;
        handleDirectionalInput(normalized);
      }
    }
  }

  return {
    handleGamepadEvent
  };
}

export { NAVIGATION_MODE };
