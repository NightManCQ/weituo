<script setup lang="ts">
import { RouterLink } from 'vue-router'
import type { 任务, 任务操作, 用户 } from '../领域/类型'

type 任务操作项 = { key: 任务操作; label: string }

defineProps<{
  当前用户: 用户 | null
  任务列表: 任务[]
  正在加载: boolean
  格式化日期: (日期: string) => string
  获取任务操作: (任务项: 任务) => 任务操作项[]
}>()

const emit = defineEmits<{
  刷新: []
  执行操作: [任务项: 任务, 操作: 任务操作]
  查看消息: [任务项: 任务]
  前往广场: []
  前往个人中心: []
}>()
</script>

<template>
  <section class="page-panel">
    <div class="welcome-row">
      <div>
        <div class="section-eyebrow">我的协作</div>
        <h1>我的委托 <span class="count-pill">{{ 任务列表.length }}</span></h1>
        <p class="muted">这里展示你已接单的委托及当前进度。</p>
      </div>
      <button v-if="当前用户" class="button button-quiet" :disabled="正在加载" @click="emit('刷新')">
        {{ 正在加载 ? '加载中…' : '刷新 ↻' }}
      </button>
    </div>
    <template v-if="当前用户">
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
              <span>发布者 {{ 任务项.发布者编号.slice(0, 8) }}</span>
              <span v-if="任务项.截止时间">截止 {{ 格式化日期(任务项.截止时间) }}</span>
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
              <button class="button button-small button-outline" @click="emit('查看消息', 任务项)">查看消息</button>
            </div>
          </div>
        </article>
      </div>
      <div v-else class="empty-state">
        <div class="empty-icon">▤</div><h3>还没有接单的委托</h3><p>前往委托广场浏览并接取适合你的委托。</p>
        <button class="button button-primary" @click="emit('前往广场')">浏览委托</button>
      </div>
    </template>
    <div v-else class="empty-state access-prompt">
      <div class="empty-icon">◉</div><h3>登录后查看我的委托</h3><p>登录后即可查看你已接单的委托。</p>
      <button class="button button-primary" @click="emit('前往个人中心')">前往登录</button>
    </div>
  </section>
</template>
