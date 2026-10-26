<script setup lang="ts">
import { RouterLink } from 'vue-router'

const 功能入口 = [
  {
    名称: '委托广场',
    描述: '浏览正在招募的委托，发现可以参与的机会。',
    图标: '▦',
    路由名: 'square',
  },
] as const
</script>

<template>
  <section class="main-content">
    <div class="page-heading">
      <div class="section-eyebrow">探索与发现</div>
      <h1>世界</h1>
      <p class="muted">在这里发现平台上的功能与机会。</p>
    </div>
    <div class="world-list">
      <RouterLink
        v-for="入口 in 功能入口"
        :key="入口.路由名"
        class="world-entry"
        :to="{ name: 入口.路由名 }"
      >
        <span class="world-entry-icon" aria-hidden="true">{{ 入口.图标 }}</span>
        <span class="world-entry-copy">
          <strong>{{ 入口.名称 }}</strong>
          <span>{{ 入口.描述 }}</span>
        </span>
        <span class="world-entry-arrow" aria-hidden="true">›</span>
      </RouterLink>
    </div>
  </section>
</template>

<style scoped>
.world-list{display:grid;gap:12px}
.world-entry{display:flex;align-items:center;gap:14px;padding:18px;background:#fff;border:1px solid #e5ebe8;border-radius:16px;color:inherit;text-decoration:none;box-shadow:0 6px 20px #183b2e08;transition:border-color .18s,transform .18s}
.world-entry:hover{transform:translateY(-2px);border-color:#a9d6c3}
.world-entry-icon{display:grid;place-items:center;flex:none;width:44px;height:44px;border-radius:14px;background:var(--mint);color:var(--green);font-size:21px}
.world-entry-copy{display:grid;gap:5px;min-width:0}
.world-entry-copy strong{font-size:15px}
.world-entry-copy>span{color:#85918d;font-size:12px;line-height:1.5}
.world-entry-arrow{margin-left:auto;color:#85918d;font-size:24px}
@media(max-width:680px){.world-entry{padding:15px;gap:11px}.world-entry-icon{width:40px;height:40px}}
</style>
