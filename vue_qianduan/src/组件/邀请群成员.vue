<script setup lang="ts">
import { computed } from 'vue'
import type { 群成员 } from '../领域/类型'

const props = defineProps<{
  好友列表: Array<{ 用户编号: string; 昵称: string }>
  成员列表: 群成员[]
  正在邀请: boolean
  modelValue: string
}>()

const emit = defineEmits<{
  邀请: [用户编号: string]
  'update:modelValue': [用户编号: string]
}>()

const 可邀请好友 = computed(() => props.好友列表.filter(
  (好友) => !props.成员列表.some((成员) => 成员.用户编号 === 好友.用户编号),
))

function 邀请() {
  if (!props.modelValue || props.正在邀请) return
  emit('邀请', props.modelValue)
}
</script>

<template>
  <section class="group-invite-panel">
    <h2>邀请好友</h2>
    <p v-if="!好友列表.length" class="muted">添加好友后才能邀请入群。</p>
    <p v-else-if="!可邀请好友.length" class="muted">好友已全部在群内。</p>
    <form v-else class="group-invite-form" @submit.prevent="邀请">
      <select
        :value="modelValue"
        aria-label="选择要邀请的好友"
        @change="emit('update:modelValue', ($event.target as HTMLSelectElement).value)"
      >
        <option value="">选择好友邀请</option>
        <option
          v-for="好友 in 可邀请好友"
          :key="好友.用户编号"
          :value="好友.用户编号"
        >{{ 好友.昵称 }}</option>
      </select>
      <button
        class="button button-outline button-small"
        :disabled="正在邀请 || !modelValue"
      >{{ 正在邀请 ? '邀请中…' : '邀请' }}</button>
    </form>
  </section>
</template>
