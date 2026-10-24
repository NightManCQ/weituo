<script setup lang="ts">
import type { 聊天会话, 用户 } from '../领域/类型'

defineProps<{
  当前用户: 用户 | null
  聊天会话列表: 聊天会话[]
  正在加载: boolean
  格式化日期: (日期: string) => string
}>()

const emit = defineEmits<{
  刷新: []
  打开会话: [会话: 聊天会话]
  前往个人中心: []
}>()
</script>

<template>
  <section class="page-panel">
    <div class="welcome-row">
      <div class="page-heading">
        <div class="section-eyebrow">委托沟通</div><h1>消息</h1>
        <p class="muted">查看你正在交流的用户，点击会话继续聊天。</p>
      </div>
      <button v-if="当前用户" class="button button-quiet" :disabled="正在加载" @click="emit('刷新')">
        {{ 正在加载 ? '加载中…' : '刷新 ↻' }}
      </button>
    </div>
    <template v-if="当前用户">
      <div v-if="聊天会话列表.length" class="conversation-list conversation-list-page">
        <button
          v-for="会话 in 聊天会话列表"
          :key="会话.任务编号"
          class="conversation-item conversation-row"
          @click="emit('打开会话', 会话)"
        >
          <span class="conversation-avatar">{{ 会话.联系人昵称.slice(0, 1) }}</span>
          <span class="conversation-copy">
            <strong>{{ 会话.联系人昵称 }} <small>· {{ 会话.任务标题 }}</small></strong>
            <small>{{ 会话.最近消息 }}</small>
          </span>
          <span class="conversation-meta">
            <small>{{ 格式化日期(会话.最近消息时间) }}</small>
            <span class="status-badge">{{ 会话.任务状态 }}</span>
          </span>
        </button>
      </div>
      <div v-else class="empty-state">
        <div class="empty-icon">◌</div>
        <h3>{{ 正在加载 ? '正在加载消息' : '还没有消息会话' }}</h3>
        <p>当你与委托参与者发送消息后，会话会显示在这里。</p>
      </div>
    </template>
    <div v-else class="empty-state access-prompt">
      <div class="empty-icon">◉</div><h3>登录后查看消息</h3><p>登录后即可查看委托沟通记录。</p>
      <button class="button button-primary" @click="emit('前往个人中心')">前往登录</button>
    </div>
  </section>
</template>
