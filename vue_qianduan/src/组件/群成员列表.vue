<script setup lang="ts">
import type { 群成员 } from '../领域/类型'

defineProps<{
  标题: string
  成员列表: 群成员[]
  当前用户编号: string | null
  可移除成员: boolean
}>()

const emit = defineEmits<{
  移除成员: [用户编号: string]
}>()
</script>

<template>
  <section class="group-members-panel">
    <h2>{{ 标题 }} · {{ 成员列表.length }}</h2>
    <p v-if="!成员列表.length" class="muted">暂无成员信息。</p>
    <ul v-else class="group-member-list">
      <li v-for="成员 in 成员列表" :key="成员.用户编号">
        <span class="group-member-copy">
          <strong>{{ 成员.昵称 }}</strong>
          <small>{{ 成员.成员角色 }}</small>
        </span>
        <button
          v-if="可移除成员 && 成员.用户编号 !== 当前用户编号 && 成员.成员角色 !== '群主'"
          class="button button-quiet button-small"
          @click="emit('移除成员', 成员.用户编号)"
        >移除</button>
      </li>
    </ul>
  </section>
</template>
