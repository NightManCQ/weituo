<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import type { 消息, 任务, 用户 } from '../领域/类型'

const 可视视口高度属性 = '--聊天可视高度'
const 可视视口顶部属性 = '--聊天可视顶部'
let 原可视视口高度 = ''
let 原可视视口顶部 = ''

const props = defineProps<{
  当前用户: 用户 | null
  联系人昵称: string
  当前任务: 任务 | null
  消息列表: 消息[]
  消息表单数据: string
  可以聊天: boolean
  正在处理: boolean
  正在发送聊天消息: boolean
  消息待确认: boolean
  有待确认消息: boolean
  可以发送消息: boolean
}>()

const emit = defineEmits<{
  返回: []
  '更新:消息表单数据': [内容: string]
  发送消息: []
}>()

const 消息滚动区域 = ref<HTMLElement | null>(null)

function 需要显示消息信息(索引: number) {
  if (索引 === 0) return true

  const 当前消息 = props.消息列表[索引]
  const 上一条消息 = props.消息列表[索引 - 1]
  if (!当前消息 || !上一条消息 || 当前消息.发送者编号 !== 上一条消息.发送者编号) {
    return true
  }

  const 当前时间 = new Date(当前消息.创建时间).getTime()
  const 上一条时间 = new Date(上一条消息.创建时间).getTime()
  return !Number.isFinite(当前时间) ||
    !Number.isFinite(上一条时间) ||
    Math.abs(当前时间 - 上一条时间) >= 5 * 60 * 1000
}

function 格式化聊天时间(时间: string) {
  const 日期 = new Date(时间)
  if (Number.isNaN(日期.getTime())) return 时间

  const 时分秒 = [日期.getHours(), 日期.getMinutes(), 日期.getSeconds()]
    .map((数字) => String(数字).padStart(2, '0'))
    .join(':')
  const 今天 = new Date()
  今天.setHours(0, 0, 0, 0)
  const 昨天 = new Date(今天)
  昨天.setDate(昨天.getDate() - 1)
  const 明天 = new Date(今天)
  明天.setDate(明天.getDate() + 1)

  if (日期 >= 今天 && 日期 < 明天) return 时分秒
  if (日期 >= 昨天 && 日期 < 今天) return `昨天 ${时分秒}`
  return `${日期.getFullYear()}年${日期.getMonth() + 1}月${日期.getDate()}日 ${时分秒}`
}

async function 滚动到最新消息() {
  await nextTick()
  const 滚动容器 = 消息滚动区域.value
  if (滚动容器) 滚动容器.scrollTop = 滚动容器.scrollHeight
}

function 同步可视视口() {
  const 可视视口 = window.visualViewport
  document.documentElement.style.setProperty(
    可视视口高度属性,
    `${可视视口?.height ?? window.innerHeight}px`,
  )
  document.documentElement.style.setProperty(
    可视视口顶部属性,
    `${可视视口?.offsetTop ?? 0}px`,
  )
}

onMounted(() => {
  原可视视口高度 = document.documentElement.style.getPropertyValue(可视视口高度属性)
  原可视视口顶部 = document.documentElement.style.getPropertyValue(可视视口顶部属性)
  同步可视视口()
  window.visualViewport?.addEventListener('resize', 同步可视视口)
  window.visualViewport?.addEventListener('scroll', 同步可视视口)
  window.addEventListener('resize', 同步可视视口)
  void 滚动到最新消息()
})

watch(() => props.消息列表, () => void 滚动到最新消息(), { flush: 'post' })

onBeforeUnmount(() => {
  window.visualViewport?.removeEventListener('resize', 同步可视视口)
  window.visualViewport?.removeEventListener('scroll', 同步可视视口)
  window.removeEventListener('resize', 同步可视视口)
  if (原可视视口高度) {
    document.documentElement.style.setProperty(可视视口高度属性, 原可视视口高度)
  } else {
    document.documentElement.style.removeProperty(可视视口高度属性)
  }
  if (原可视视口顶部) {
    document.documentElement.style.setProperty(可视视口顶部属性, 原可视视口顶部)
  } else {
    document.documentElement.style.removeProperty(可视视口顶部属性)
  }
})
</script>

<template>
  <section class="page-panel chat-page">
    <div class="chat-page-heading">
      <button class="button button-quiet chat-back" @click="emit('返回')">← 返回消息列表</button>
      <div v-if="当前任务" class="chat-page-title">
        <div class="section-eyebrow">与 {{ 联系人昵称 }} 聊天 · 委托{{ 当前任务.任务状态 }}</div>
        <h1>{{ 当前任务.标题 }}</h1>
      </div>
    </div>
    <section v-if="当前用户 && 当前任务 && 可以聊天" class="full-chat-layout">
      <div ref="消息滚动区域" class="message-list full-message-list" aria-live="polite">
        <div v-if="!消息列表.length" class="chat-empty">还没有消息，发起一段对话吧。</div>
        <article
          v-for="(消息项, 索引) in 消息列表"
          :key="消息项.消息编号"
          class="message"
          :class="{ mine: 消息项.发送者编号 === 当前用户.用户编号 }"
        >
          <div v-if="需要显示消息信息(索引)" class="message-meta">
            {{ 消息项.发送者编号 === 当前用户.用户编号 ? '我' : 消息项.发送者昵称 }} · {{ 格式化聊天时间(消息项.创建时间) }}
          </div>
          <p>{{ 消息项.消息正文 }}</p>
        </article>
      </div>
      <form class="chat-compose full-chat-compose" @submit.prevent="emit('发送消息')">
        <input
          :value="消息表单数据"
          maxlength="4000"
          :placeholder="有待确认消息 ? '消息等待服务端确认…' : '输入消息…'"
          aria-label="消息内容"
          :disabled="有待确认消息 || 正在发送聊天消息"
          @input="emit('更新:消息表单数据', ($event.target as HTMLInputElement).value)"
        />
        <button
          class="button button-primary"
          :disabled="!可以发送消息 || 正在处理 || 正在发送聊天消息 || 消息待确认"
        >{{ 正在发送聊天消息 ? '发送中…' : (有待确认消息 ? (消息待确认 ? '等待验证…' : '重试发送') : '发送') }}</button>
      </form>
    </section>
    <div v-else class="empty-state">
      <h3>无法打开此聊天</h3><p>请返回消息列表后重新选择会话。</p>
      <button class="button button-outline" @click="emit('返回')">返回消息列表</button>
    </div>
  </section>
</template>
