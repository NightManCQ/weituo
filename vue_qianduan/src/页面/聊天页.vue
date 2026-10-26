<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { 发送请求 } from '../接口/请求'
import { 生成唯一编号 } from '../领域/编号'
import { 订阅实时事件, type 实时事件 } from '../实时/消息中心'
import type { 群成员, 群消息, 用户 } from '../领域/类型'
import GroupMemberList from '../组件/群成员列表.vue'
import GroupInvite from '../组件/邀请群成员.vue'

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
  好友私聊?: { 用户编号: string; 联系人昵称: string }
  群组编号?: string
  任务编号?: string
}>()

const emit = defineEmits<{
  返回: []
}>()

const router = useRouter()
const 好友消息滚动区域 = ref<HTMLElement | null>(null)
const 好友私聊消息列表 = ref<私聊消息[]>([])
const 好友消息表单数据 = ref('')
const 好友消息错误 = ref('')
const 正在加载好友消息 = ref(false)
const 正在发送好友消息 = ref(false)
const 好友消息待确认 = ref<{ 客户端消息编号: string; 正文: string } | null>(null)
const 群消息滚动区域 = ref<HTMLElement | null>(null)
const 群组 = ref<{
  群组编号: string
  关联任务编号: string | null
  群组名称: string
  群组公告: string
  群主编号: string
  最后消息序号: number
  成员列表: 群成员[]
} | null>(null)
const 群消息列表 = ref<群消息[]>([])
const 群聊好友列表 = ref<Array<{ 用户编号: string; 昵称: string }>>([])
const 正在加载群聊 = ref(false)
const 正在发送群消息 = ref(false)
const 正在邀请群成员 = ref(false)
const 群聊错误 = ref('')
const 群聊反馈 = ref('')
const 群菜单已打开 = ref(false)
const 选择邀请用户编号 = ref('')
let 群聊加载序号 = 0

const 当前令牌 = () => localStorage.getItem('renwu_token') || ''

async function 滚动到最新群消息() {
  await nextTick()
  const 区域 = 群消息滚动区域.value
  if (区域) 区域.scrollTop = 区域.scrollHeight
}

async function 加载群聊() {
  const 用户编号 = props.当前用户?.用户编号
  if (!用户编号 || (!props.群组编号 && !props.任务编号)) {
    群组.value = null
    群消息列表.value = []
    正在加载群聊.value = false
    return
  }

  const 加载编号 = ++群聊加载序号
  const 目标编号 = props.群组编号 || props.任务编号 || ''
  正在加载群聊.value = true
  群聊错误.value = ''
  群聊反馈.value = ''
  try {
    const 群组编号 = props.群组编号 || (await 发送请求<{ 群组编号: string }>(
      `/tasks/${encodeURIComponent(目标编号)}/group-chat`,
      {},
      当前令牌(),
    )).群组编号
    const 群组结果 = await 发送请求<NonNullable<typeof 群组.value>>(
      `/groups/${encodeURIComponent(群组编号)}`,
      {},
      当前令牌(),
    )
    const [消息结果, 好友结果] = await Promise.all([
      发送请求<群消息[]>(
        `/groups/${encodeURIComponent(群组编号)}/messages?limit=100`,
        {},
        当前令牌(),
      ),
      群组结果.关联任务编号
        ? Promise.resolve([])
        : 发送请求<Array<{ 用户编号: string; 昵称: string }>>('/my/friends', {}, 当前令牌()),
    ])
    if (加载编号 !== 群聊加载序号) return
    群组.value = 群组结果
    群消息列表.value = 消息结果
    群聊好友列表.value = 好友结果
    await 滚动到最新群消息()
  } catch (原因) {
    if (加载编号 === 群聊加载序号) {
      群聊错误.value = 原因 instanceof Error ? 原因.message : '加载群聊失败'
    }
  } finally {
    if (加载编号 === 群聊加载序号) 正在加载群聊.value = false
  }
}

function 添加群消息(消息: 群消息) {
  if (
    消息.群组编号 !== 群组.value?.群组编号 ||
    群消息列表.value.some((已有消息) => 已有消息.消息编号 === 消息.消息编号)
  ) return
  群消息列表.value = [...群消息列表.value, 消息]
    .sort((左侧, 右侧) => 左侧.群内序号 - 右侧.群内序号)
  if (群组.value) {
    群组.value.最后消息序号 = Math.max(群组.value.最后消息序号, 消息.群内序号)
  }
  void 滚动到最新群消息()
}

async function 发送群聊消息() {
  const 正文 = 群消息正文.value.trim()
  if (!正文 || !props.当前用户 || !群组.value || 正在发送群消息.value) return
  正在发送群消息.value = true
  群聊错误.value = ''
  try {
    const 消息 = await 发送请求<群消息>(
      `/groups/${encodeURIComponent(群组.value.群组编号)}/messages`,
      {
        method: 'POST',
        body: JSON.stringify({ client_message_id: 生成唯一编号(), content: 正文 }),
      },
      当前令牌(),
    )
    添加群消息(消息)
    群消息正文.value = ''
  } catch (原因) {
    群聊错误.value = 原因 instanceof Error ? 原因.message : '发送群消息失败'
  } finally {
    正在发送群消息.value = false
  }
}

const 群消息正文 = ref('')
const 有权管理群成员 = () => !!群组.value?.成员列表.some(
  (成员) =>
    成员.用户编号 === props.当前用户?.用户编号 &&
    ['群主', '管理员'].includes(成员.成员角色),
)

async function 邀请好友(用户编号: string) {
  if (!群组.value || 群组.value.关联任务编号 || 正在邀请群成员.value) return
  正在邀请群成员.value = true
  群聊错误.value = ''
  群聊反馈.value = ''
  try {
    await 发送请求(
      `/groups/${encodeURIComponent(群组.value.群组编号)}/invitations`,
      { method: 'POST', body: JSON.stringify({ user_id: 用户编号 }) },
      当前令牌(),
    )
    const 好友 = 群聊好友列表.value.find((项目) => 项目.用户编号 === 用户编号)
    群聊反馈.value = `已邀请 ${好友?.昵称 ?? '好友'}`
    选择邀请用户编号.value = ''
  } catch (原因) {
    群聊错误.value = 原因 instanceof Error ? 原因.message : '邀请好友失败'
  } finally {
    正在邀请群成员.value = false
  }
}

async function 移除群成员(用户编号: string) {
  const 成员 = 群组.value?.成员列表.find((项目) => 项目.用户编号 === 用户编号)
  if (!群组.value || !成员 || !window.confirm(`确定将 ${成员.昵称} 移出群聊吗？`)) return
  群聊错误.value = ''
  try {
    await 发送请求(
      `/groups/${encodeURIComponent(群组.value.群组编号)}/members/${encodeURIComponent(用户编号)}`,
      { method: 'DELETE' },
      当前令牌(),
    )
    await 加载群聊()
  } catch (原因) {
    群聊错误.value = 原因 instanceof Error ? 原因.message : '移除群成员失败'
  }
}

async function 退出群聊() {
  if (!群组.value || !window.confirm('确定退出这个群聊吗？')) return
  try {
    await 发送请求(
      `/groups/${encodeURIComponent(群组.value.群组编号)}/leave`,
      { method: 'DELETE' },
      当前令牌(),
    )
    await router.replace({ name: 'contacts' })
  } catch (原因) {
    群聊错误.value = 原因 instanceof Error ? 原因.message : '退出群聊失败'
  }
}

function 返回群聊() {
  if (props.好友私聊) {
    emit('返回')
    return
  }
  void router.push({
    name: 群组.value?.关联任务编号 || props.任务编号 ? 'messages' : 'contacts',
  })
}

function 处理群聊实时事件(事件: 实时事件) {
  if (事件.事件类型 !== 'group.message_created' || !事件.数据) return
  const 数据 = 事件.数据
  const 群组编号 = 数据.群组编号 ?? 事件.原始事件.群组编号
  if (群组编号 !== 群组.value?.群组编号) return
  const 消息编号 = 数据.消息编号
  const 群内序号 = 数据.群内序号
  const 发送者编号 = 数据.发送者编号
  const 发送者昵称 = 数据.发送者昵称
  const 客户端消息编号 = 数据.客户端消息编号
  const 正文 = 数据.消息正文
  const 创建时间 = 数据.创建时间
  if (
    typeof 消息编号 !== 'string' ||
    typeof 群内序号 !== 'number' ||
    typeof 发送者编号 !== 'string' ||
    typeof 发送者昵称 !== 'string' ||
    typeof 客户端消息编号 !== 'string' ||
    typeof 正文 !== 'string' ||
    typeof 创建时间 !== 'string'
  ) {
    console.error('群聊 MQTT 事件缺少消息字段。', 事件.原始事件)
    return
  }
  const 当前群组 = 群组.value
  if (!当前群组) return
  添加群消息({
    消息编号,
    群组编号: 当前群组.群组编号,
    群内序号,
    发送者编号,
    发送者昵称,
    客户端消息编号,
    消息正文: 正文,
    创建时间,
  })
}

const 取消群聊订阅 = 订阅实时事件(处理群聊实时事件)

watch(
  () => [props.群组编号, props.任务编号, props.当前用户?.用户编号] as const,
  () => {
    群聊加载序号 += 1
    群组.value = null
    群消息列表.value = []
    群聊错误.value = ''
    群聊反馈.value = ''
    群消息正文.value = ''
    选择邀请用户编号.value = ''
    群菜单已打开.value = false
    void 加载群聊()
  },
  { immediate: true },
)

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
})

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
  取消群聊订阅()
  群聊加载序号 += 1
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
    <header
      class="chat-page-heading"
      :class="{ 'group-chat-heading': !!(群组编号 || 任务编号) }"
      @keydown.esc="群菜单已打开 = false"
    >
      <button class="button button-quiet chat-back" @click="返回群聊">
        ← {{ 好友私聊 ? '返回联系人' : (群组?.关联任务编号 || 任务编号 ? '返回消息列表' : '返回群聊列表') }}
      </button>
      <div v-if="好友私聊" class="chat-page-title">
        <div class="section-eyebrow">好友私聊</div>
        <h1>{{ 好友私聊.联系人昵称 }}</h1>
      </div>
      <div v-if="群组" class="chat-page-title group-chat-title">
        <div class="section-eyebrow">
          {{ 群组.关联任务编号 ? '委托参与者' : '群聊' }} · {{ 群组.成员列表.length }} 位成员
        </div>
        <h1>{{ 群组.群组名称 }}</h1>
      </div>
      <div v-if="群组" class="group-chat-menu">
        <button
          class="button button-quiet group-menu-toggle"
          type="button"
          aria-label="群聊功能菜单"
          :aria-expanded="群菜单已打开"
          aria-haspopup="true"
          @click="群菜单已打开 = !群菜单已打开"
        >⋯</button>
        <div v-if="群菜单已打开" class="group-menu-panel">
          <GroupInvite
            v-if="!群组.关联任务编号 && 有权管理群成员()"
            v-model="选择邀请用户编号"
            :好友列表="群聊好友列表"
            :成员列表="群组.成员列表"
            :正在邀请="正在邀请群成员"
            @邀请="邀请好友"
          />
          <GroupMemberList
            :标题="群组.关联任务编号 ? '委托参与者' : '群成员'"
            :成员列表="群组.成员列表"
            :当前用户编号="当前用户?.用户编号 ?? null"
            :可移除成员="!群组.关联任务编号 && 有权管理群成员()"
            @移除成员="移除群成员"
          />
          <button
            v-if="!群组.关联任务编号 && 当前用户?.用户编号 !== 群组.群主编号"
            class="button button-quiet group-leave-button"
            type="button"
            @click="退出群聊"
          >退出群聊</button>
        </div>
      </div>
    </header>
    <div v-if="群聊错误" class="group-feedback error-notice" role="alert">{{ 群聊错误 }}</div>
    <div v-if="群聊反馈" class="group-feedback info-notice" role="status">{{ 群聊反馈 }}</div>
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
    <section v-else-if="当前用户 && 群组" class="full-chat-layout group-full-chat-layout">
      <div ref="群消息滚动区域" class="message-list full-message-list" aria-live="polite">
        <div v-if="群聊错误 && !群消息列表.length" class="chat-empty chat-error" role="alert">
          {{ 群聊错误 }}
        </div>
        <div v-else-if="正在加载群聊 && !群消息列表.length" class="chat-empty">
          正在加载群消息…
        </div>
        <div v-else-if="!群消息列表.length" class="chat-empty">
          还没有消息，向群里打个招呼吧。
        </div>
        <article
          v-for="消息 in 群消息列表"
          :key="消息.消息编号"
          class="message"
          :class="{ mine: 消息.发送者编号 === 当前用户.用户编号 }"
        >
          <div class="message-meta">
            {{ 消息.发送者编号 === 当前用户.用户编号 ? '我' : 消息.发送者昵称 }}
            · {{ 格式化聊天时间(消息.创建时间) }}
          </div>
          <p>{{ 消息.消息正文 }}</p>
        </article>
      </div>
      <form class="chat-compose full-chat-compose" @submit.prevent="发送群聊消息">
        <input
          v-model="群消息正文"
          maxlength="4000"
          placeholder="输入群消息…"
          aria-label="群消息内容"
          :disabled="正在发送群消息"
        />
        <button
          class="button button-primary"
          :disabled="正在发送群消息 || !群消息正文.trim()"
        >{{ 正在发送群消息 ? '发送中…' : '发送' }}</button>
      </form>
    </section>
    <div v-else-if="正在加载群聊" class="empty-state">正在加载群组…</div>
    <div v-else class="empty-state">
      <h3>{{ 好友私聊 ? '无法打开此私聊' : '无法打开这个群聊' }}</h3>
      <p>{{ 好友私聊 ? '请返回联系人列表后重新选择好友。' : '群组不存在，或你不是有效成员。' }}</p>
      <button class="button button-outline" @click="返回群聊">
        {{ 好友私聊 ? '返回联系人' : (任务编号 ? '返回消息列表' : '返回群聊列表') }}
      </button>
    </div>
  </section>
</template>
