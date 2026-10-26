<script setup lang="ts">
import { RouterLink } from 'vue-router'
import type { 任务, 任务操作, 用户 } from '../领域/类型'

type 任务操作项 = { key: 任务操作; label: string }

defineProps<{
  当前用户: 用户 | null
  任务列表: 任务[]
  格式化日期: (日期: string) => string
  获取任务操作: (任务项: 任务) => 任务操作项[]
}>()

const emit = defineEmits<{
  刷新: []
  发布需求: []
  执行操作: [任务项: 任务, 操作: 任务操作]
  查看消息: [任务项: 任务]
  登录: []
}>()

</script>

<template>
  <section class="main-content">
    <div class="welcome-row">
      <div>
        <div class="section-eyebrow">正在发生</div>
        <h1>委托广场 <span class="count-pill">{{ 任务列表.length }}</span></h1>
        <p class="muted">{{ 当前用户 ? `你好，${当前用户.昵称}，看看有哪些委托适合你。` : '发现委托，找到一起完成它的人。' }}</p>
      </div>
      <div class="square-actions">
        <RouterLink
          v-if="当前用户"
          class="button button-outline"
          :to="{ name: 'tasks' }"
        >我的委托</RouterLink>
        <button class="button button-primary" @click="emit('发布需求')">＋ 发布委托</button>
      </div>
    </div>
    <div class="list-heading">
      <div><div class="section-eyebrow">浏览委托</div><h2>待接单委托</h2></div>
      <div class="list-controls">
        <button class="button button-quiet" @click="emit('刷新')">刷新 ↻</button>
      </div>
    </div>
    <div v-if="任务列表.length" class="task-grid">
      <article v-for="任务项 in 任务列表" :key="任务项.任务编号" class="task-card">
        <RouterLink class="commission-card-content" :to="{ name: 'commission-detail', params: { id: 任务项.任务编号 } }">
          <div class="task-card-top">
            <span class="status-badge" :class="`status-${任务项.任务状态}`"><i></i>{{ 任务项.任务状态 }}</span>
            <span class="task-date">{{ 格式化日期(任务项.创建时间) }}</span>
          </div>
          <h3>{{ 任务项.标题 }}</h3>
          <p class="task-description">{{ 任务项.描述 || '暂无补充描述' }}</p>
          <div class="task-meta">
            <span>发布者 {{ 当前用户 && 任务项.发布者编号 === 当前用户.用户编号 ? '我' : 任务项.发布者编号.slice(0, 8) }}</span>
            <span v-if="任务项.截止时间">截止 {{ 格式化日期(任务项.截止时间) }}</span>
            <span class="task-reward">
              {{ 任务项.报酬积分 !== null ? `${任务项.报酬积分} 积分报酬` : '历史委托无积分报酬' }}
            </span>
          </div>
        </RouterLink>
        <div class="task-card-footer">
          <span class="task-version">#{{ 任务项.任务编号.slice(0, 8) }}</span>
          <div class="action-row">
            <button
              v-for="操作 in 获取任务操作(任务项)"
              :key="操作.key"
              class="button button-small"
              :class="操作.key === 'cancel' ? 'button-quiet' : 'button-primary'"
              @click="emit('执行操作', 任务项, 操作.key)"
            >{{ 操作.label }}</button>
            <button
              v-if="当前用户 && (任务项.发布者编号 === 当前用户.用户编号 || 任务项.接单者编号 === 当前用户.用户编号)"
              class="button button-small button-outline"
              @click="emit('查看消息', 任务项)"
            >查看消息</button>
            <button
              v-else-if="!当前用户 && 任务项.任务状态 === '待接单'"
              class="button button-small button-outline"
              @click="emit('登录')"
            >登录后接单</button>
          </div>
        </div>
      </article>
    </div>
    <div v-else class="empty-state"><div class="empty-icon">✳</div><h3>暂时没有委托</h3><p>稍后刷新看看，或发布一个新委托。</p></div>
  </section>
</template>

<style scoped>
.square-actions{display:flex;align-items:center;gap:8px}
@media(max-width:680px){.square-actions{flex-wrap:wrap;justify-content:flex-end}}
</style>
