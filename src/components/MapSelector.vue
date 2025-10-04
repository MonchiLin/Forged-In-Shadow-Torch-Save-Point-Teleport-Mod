<template>
  <section class="map-selector relative z-10">
    <div class="map-selector__scroll-container">
      <div
        class="map-selector__track flex gap-3"
        role="tablist"
      >
        <button
          v-for="(map, index) in maps"
          :key="map.id"
          type="button"
          class="map-card group relative flex-shrink-0"
          role="tab"
          :aria-selected="index === activeIndex"
          :data-state="index === activeIndex ? 'active' : 'idle'"
          :class="{
            'is-active': index === activeIndex,
            'is-focused': navigationMode === navEnum.MAP && index === focusedIndex
          }"
          @click="handleSelect(index)"
          @keydown.enter.prevent="handleSelect(index)"
          @keydown.space.prevent="handleSelect(index)"
        >
          <!-- 卡片背景和边框 -->
          <div class="map-card__bg"></div>
          <div class="map-card__border"></div>
          <div class="map-card__glow"></div>

          <!-- 卡片内容 -->
          <div class="map-card__content">
            <span class="map-card__label">{{ map.label }}</span>
            <!-- 激活指示器 -->
            <div class="map-card__indicator"></div>
          </div>

          <!-- 扫描线效果 -->
          <div class="map-card__scanline"></div>
        </button>
      </div>
    </div>
  </section>
</template>

<script setup>
const props = defineProps({
  maps: {
    type: Array,
    default: () => []
  },
  activeIndex: {
    type: Number,
    default: 0
  },
  focusedIndex: {
    type: Number,
    default: 0
  },
  navigationMode: {
    type: String,
    default: ''
  },
  navEnum: {
    type: Object,
    required: true
  }
});

const emit = defineEmits(['select']);

function handleSelect(index) {
  if (!props.maps[index]) {
    return;
  }
  emit('select', index);
}
</script>
