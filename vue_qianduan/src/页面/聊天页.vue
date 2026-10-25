<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { 发送请求 } from '../接口/请求'
import { 生成唯一编号 } from '../领域/编号'
import { 订阅实时事件 } from '../实时/消息中心'
import type { 消息, 任务, 用户 } from '../领域/类型'

type 私聊消息 = {
  消息编号: string
  会话编号: string
  发送者编号: string
  发送者昵称: string
  客户端消息编号: string
  消息正文: string
  创建时间: string
}

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
  好友私聊?: { 用户编号: string; 联系人昵称: string }
}>()

const emit = defineEmits<{
  返回: []
  '更新:消息表单数据': [内容: string]
  发送消息: []
}>()

const 消息滚动区域 = ref<HTMLElement | null>(null)
const 好友消息滚动区域 = ref<HTMLElement | null>(null)
const 好友私聊消息列表 = ref<私聊消息[]>([])
const 好友消息表单数据 = ref('')
const 好友消息错误 = ref('')
const 正在加载好友消息 = ref(false)
const 正在发送好友消息 = ref(false)
const 好友消息待确认 = ref<{ 客户端消息编号: string; 正文: string } | null>(null)

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

async function 滚动到最新好友消息() {
  await nextTick()
  const 滚动容器 = 好友消息滚动区域.value
  if (滚动容器) 滚动容器.scrollTop = 滚动容器.scrollHeight
}

async function 加载好友消息(好友编号: string) {
  if (!props.当前用户 || !好友编号 || 正在加载好友消息.value) return
  正在加载好友消息.value = true
  try {
    const 消息 = await 发送请求<私聊消息[]>(
      `/my/friends/${encodeURIComponent(好友编号)}/messages?limit=200`,
      {},
      localStorage.getItem('renwu_token') || '',
    )
    if (props.好友私聊?.用户编号 === 好友编号) {
      const 滚动容器 = 好友消息滚动区域.value
      const 原本接近底部 = !滚动容器 ||
        滚动容器.scrollHeight - 滚动容器.scrollTop - 滚动容器.clientHeight < 80
      const 首次加载 = 好友私聊消息列表.value.length === 0
      好友私聊消息列表.value = 消息
      好友消息错误.value = ''
      if (原本接近底部 || 首次加载) await 滚动到最新好友消息()
    }
  } catch (原因) {
    if (props.好友私聊?.用户编号 === 好友编号) {
      好友消息错误.value = 原因 instanceof Error ? 原因.message : '加载私聊消息失败'
    }
  } finally {
    正在加载好友消息.value = false
  }
}

async function 发送好友私聊消息() {
  const 私聊 = props.好友私聊
  const 正文 = 好友消息表单数据.value.trim()
  if (!私聊 || !props.当前用户 || !正文 || 正在发送好友消息.value) return
  const 待发送消息 = 好友消息待确认.value ?? {
    客户端消息编号: 生成唯一编号(),
    正文,
  }
  好友消息待确认.value = 待发送消息
  正在发送好友消息.value = true
  好友消息错误.value = ''
  try {
    const 消息 = await 发送请求<私聊消息>(
      `/my/friends/${encodeURIComponent(私聊.用户编号)}/messages`,
      {
        method: 'POST',
        body: JSON.stringify({
          client_message_id: 待发送消息.客户端消息编号,
          content: 待发送消息.正文,
        }),
      },
      localStorage.getItem('renwu_token') || '',
    )
    好友私聊消息列表.value = [...好友私聊消息列表.value, 消息]
      .filter((消息项, 索引, 列表) =>
        列表.findIndex((候选) => 候选.消息编号 === 消息项.消息编号) === 索引)
      .slice(-200)
    好友消息表单数据.value = ''
    好友消息待确认.value = null
    await 滚动到最新好友消息()
  } catch (原因) {
    好友消息错误.value = 原因 instanceof Error ? 原因.message : '发送私聊消息失败'
  } finally {
    正在发送好友消息.value = false
  }
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
watch(
  () => [props.好友私聊?.用户编号, props.当前用户?.用户编号] as const,
  ([好友编号, 用户编号]) => {
    好友私聊消息列表.value = []
    好友消息表单数据.value = ''
    好友消息待确认.value = null
    好友消息错误.value = ''
    if (!好友编号 || !用户编号) return
    void 加载好友消息(好友编号)
  },
  { immediate: true },
)

const 取消好友消息订阅 = 订阅实时事件((事件) => {
  if (事件.事件类型 !== 'friend.message_created' || !事件.数据) return
  const 数据 = 事件.数据
  const 当前私聊 = props.好友私聊
  const 当前用户编号 = props.当前用户?.用户编号
  const 联系人编号 = 数据.联系人编号 ?? 数据.contact_user_id
  if (
    !当前私聊 ||
    !当前用户编号 ||
    联系人编号 !== 当前私聊.用户编号
  ) return
  const 消息编号 = 数据.消息编号 ?? 数据.message_id
  const 会话编号 = 数据.会话编号 ?? 数据.conversation_id ?? 事件.会话编号
  const 发送者编号 = 数据.发送者编号 ?? 数据.sender_id
  const 发送者昵称 = 数据.发送者昵称 ?? 数据.sender_nickname
  const 客户端消息编号 = 数据.客户端消息编号 ?? 数据.client_message_id
  const 消息正文 = 数据.消息正文 ?? 数据.content
  const 创建时间 = 数据.创建时间 ?? 数据.created_at
  if (
    typeof 消息编号 !== 'string' ||
    typeof 会话编号 !== 'string' ||
    typeof 发送者编号 !== 'string' ||
    typeof 发送者昵称 !== 'string' ||
    typeof 客户端消息编号 !== 'string' ||
    typeof 消息正文 !== 'string' ||
    typeof 创建时间 !== 'string'
  ) {
    console.error('好友私聊 MQTT 事件缺少消息字段。', 事件.原始事件)
    return
  }
  if (
    发送者编号 !== 当前用户编号 &&
    发送者编号 !== 当前私聊.用户编号
  ) return
  if (好友私聊消息列表.value.some((消息项) => 消息项.消息编号 === 消息编号)) return

  const 滚动容器 = 好友消息滚动区域.value
  const 原本接近底部 = !滚动容器 ||
    滚动容器.scrollHeight - 滚动容器.scrollTop - 滚动容器.clientHeight < 80
  好友私聊消息列表.value = [...好友私聊消息列表.value, {
    消息编号,
    会话编号,
    发送者编号,
    发送者昵称,
    客户端消息编号,
    消息正文,
    创建时间,
  }].sort((左侧, 右侧) => 左侧.创建时间.localeCompare(右侧.创建时间))
  if (原本接近底部) void 滚动到最新好友消息()
})

onBeforeUnmount(() => {
  取消好友消息订阅()
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
      <button class="button button-quiet chat-back" @click="emit('返回')">
        ← {{ 好友私聊 ? '返回联系人' : '返回消息列表' }}
      </button>
      <div v-if="好友私聊" class="chat-page-title">
        <div class="section-eyebrow">好友私聊</div>
        <h1>{{ 好友私聊.联系人昵称 }}</h1>
      </div>
      <div v-if="当前任务" class="chat-page-title">
        <div class="section-eyebrow">与 {{ 联系人昵称 }} 聊天 · 委托{{ 当前任务.任务状态 }}</div>
        <h1>{{ 当前任务.标题 }}</h1>
      </div>
    </div>
    <section v-if="当前用户 && 好友私聊" class="full-chat-layout">
      <div ref="好友消息滚动区域" class="message-list full-message-list" aria-live="polite">
        <div v-if="好友消息错误" class="chat-empty chat-error" role="alert">{{ 好友消息错误 }}</div>
        <div v-else-if="!好友私聊消息列表.length" class="chat-empty">
          {{ 正在加载好友消息 ? '正在加载消息…' : '还没有消息，发起一段对话吧。' }}
        </div>
        <article
          v-for="消息项 in 好友私聊消息列表"
          :key="消息项.消息编号"
          class="message"
          :class="{ mine: 消息项.发送者编号 === 当前用户.用户编号 }"
        >
          <div class="message-meta">
            {{ 消息项.发送者编号 === 当前用户.用户编号 ? '我' : 消息项.发送者昵称 }}
            · {{ 格式化聊天时间(消息项.创建时间) }}
          </div>
          <p>{{ 消息项.消息正文 }}</p>
        </article>
      </div>
      <form class="chat-compose full-chat-compose" @submit.prevent="发送好友私聊消息">
        <input
          v-model="好友消息表单数据"
          maxlength="4000"
          placeholder="输入私聊消息…"
          aria-label="私聊消息内容"
          :disabled="正在发送好友消息 || 好友消息待确认 !== null"
        />
        <button
          class="button button-primary"
          :disabled="!好友消息表单数据.trim() || 正在发送好友消息"
        >{{ 正在发送好友消息 ? '发送中…' : (好友消息待确认 ? '重试发送' : '发送') }}</button>
      </form>
    </section>
    <section v-else-if="当前用户 && 当前任务 && 可以聊天" class="full-chat-layout">
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
