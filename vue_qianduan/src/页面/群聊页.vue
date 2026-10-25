<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { 发送请求 } from '../接口/请求'
import { 生成唯一编号 } from '../领域/编号'
import { 订阅实时事件, type 实时事件 } from '../实时/消息中心'
import type { 好友聊天会话, 群成员, 群消息, 用户 } from '../领域/类型'

const props = defineProps<{
  群组编号?: string
  任务编号?: string
  当前用户: 用户 | null
}>()
const router = useRouter()
const 当前群组编号 = ref('')
const 群组 = ref<{
  群组编号: string
  关联任务编号: string | null
  群组名称: string
  群组公告: string
  群主编号: string
  最后消息序号: number
  成员列表: 群成员[]
} | null>(null)
const 消息列表 = ref<群消息[]>([])
const 好友列表 = ref<好友聊天会话[]>([])
const 选择好友编号 = ref('')
const 消息正文 = ref('')
const 正在加载 = ref(false)
const 正在发送 = ref(false)
const 正在邀请 = ref(false)
const 错误信息 = ref('')
const 邀请反馈 = ref('')
const 消息区域 = ref<HTMLElement | null>(null)

function 获取令牌() {
  return localStorage.getItem('renwu_token') || ''
}

async function 加载群聊() {
  if (!props.当前用户 || (!props.群组编号 && !props.任务编号)) {
    群组.value = null
    消息列表.value = []
    return
  }
  const 目标编号 = props.群组编号 || props.任务编号 || ''
  正在加载.value = true
  错误信息.value = ''
  try {
    const 群组编号 = props.群组编号 || (await 发送请求<{ 群组编号: string }>(
      `/tasks/${encodeURIComponent(目标编号)}/group-chat`,
      {},
      获取令牌(),
    )).群组编号
    if (目标编号 !== (props.群组编号 || props.任务编号)) return
    当前群组编号.value = 群组编号
    const 群组结果 = await 发送请求<NonNullable<typeof 群组.value>>(
      `/groups/${encodeURIComponent(群组编号)}`,
      {},
      获取令牌(),
    )
    const [消息结果, 好友结果] = await Promise.all([
      发送请求<群消息[]>(
        `/groups/${encodeURIComponent(群组编号)}/messages?limit=100`,
        {},
        获取令牌(),
      ),
      群组结果.关联任务编号
        ? Promise.resolve([])
        : 发送请求<Array<{ 用户编号: string; 昵称: string }>>('/my/friends', {}, 获取令牌()),
    ])
    if (目标编号 !== (props.群组编号 || props.任务编号)) return
    群组.value = 群组结果
    消息列表.value = 消息结果
    好友列表.value = 好友结果.map((好友) => ({
      会话编号: '',
      联系人编号: 好友.用户编号,
      联系人昵称: 好友.昵称,
      最近消息: null,
      最近消息时间: null,
    }))
    await 滚动到底部()
  } catch (原因) {
    if (目标编号 === (props.群组编号 || props.任务编号)) {
      错误信息.value = 原因 instanceof Error ? 原因.message : '加载群聊失败'
    }
  } finally {
    if (目标编号 === (props.群组编号 || props.任务编号)) 正在加载.value = false
  }
}

async function 滚动到底部() {
  await nextTick()
  const 区域 = 消息区域.value
  if (区域) 区域.scrollTop = 区域.scrollHeight
}

function 添加消息(消息: 群消息) {
  if (
    消息.群组编号 !== 当前群组编号.value ||
    消息列表.value.some((已有消息) => 已有消息.消息编号 === 消息.消息编号)
  ) return
  消息列表.value = [...消息列表.value, 消息].sort((左侧, 右侧) =>
    左侧.群内序号 - 右侧.群内序号,
  )
  if (群组.value) {
    群组.value.最后消息序号 = Math.max(群组.value.最后消息序号, 消息.群内序号)
  }
  void 滚动到底部()
}

async function 发送消息() {
  const 正文 = 消息正文.value.trim()
  if (!正文 || !props.当前用户 || 正在发送.value) return
  正在发送.value = true
  错误信息.value = ''
  try {
    const 消息 = await 发送请求<群消息>(
      `/groups/${encodeURIComponent(当前群组编号.value)}/messages`,
      {
        method: 'POST',
        body: JSON.stringify({
          client_message_id: 生成唯一编号(),
          content: 正文,
        }),
      },
      获取令牌(),
    )
    添加消息(消息)
    消息正文.value = ''
  } catch (原因) {
    错误信息.value = 原因 instanceof Error ? 原因.message : '发送群消息失败'
  } finally {
    正在发送.value = false
  }
}

async function 邀请好友() {
  if (!选择好友编号.value || 正在邀请.value) return
  正在邀请.value = true
  邀请反馈.value = ''
  错误信息.value = ''
  try {
    await 发送请求(
      `/groups/${encodeURIComponent(当前群组编号.value)}/invitations`,
      {
        method: 'POST',
        body: JSON.stringify({ user_id: 选择好友编号.value }),
      },
      获取令牌(),
    )
    const 好友 = 好友列表.value.find((项目) => 项目.联系人编号 === 选择好友编号.value)
    邀请反馈.value = `已邀请 ${好友?.联系人昵称 ?? '好友'}`
    选择好友编号.value = ''
  } catch (原因) {
    错误信息.value = 原因 instanceof Error ? 原因.message : '邀请好友失败'
  } finally {
    正在邀请.value = false
  }
}

async function 移除成员(成员: 群成员) {
  if (!window.confirm(`确定将 ${成员.昵称} 移出群聊吗？`)) return
  错误信息.value = ''
  try {
    await 发送请求(
      `/groups/${encodeURIComponent(当前群组编号.value)}/members/${encodeURIComponent(成员.用户编号)}`,
      { method: 'DELETE' },
      获取令牌(),
    )
    await 加载群聊()
  } catch (原因) {
    错误信息.value = 原因 instanceof Error ? 原因.message : '移除群成员失败'
  }
}

async function 退出群组() {
  if (!window.confirm('确定退出这个群聊吗？')) return
  try {
    await 发送请求(
      `/groups/${encodeURIComponent(当前群组编号.value)}/leave`,
      { method: 'DELETE' },
      获取令牌(),
    )
    await router.replace({ name: 'groups' })
  } catch (原因) {
    错误信息.value = 原因 instanceof Error ? 原因.message : '退出群聊失败'
  }
}

function 处理实时事件(事件: 实时事件) {
  if (事件.事件类型 !== 'group.message_created' || !事件.数据) return
  const 数据 = 事件.数据
  const 群组编号 = 数据.群组编号 ?? 事件.原始事件.群组编号
  if (群组编号 !== 当前群组编号.value) return
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
  添加消息({
    消息编号,
    群组编号: 当前群组编号.value,
    群内序号,
    发送者编号,
    发送者昵称,
    客户端消息编号,
    消息正文: 正文,
    创建时间,
  })
}

const 取消实时订阅 = 订阅实时事件(处理实时事件)

watch(() => [props.群组编号, props.任务编号, props.当前用户?.用户编号] as const, () => {
  群组.value = null
  当前群组编号.value = ''
  消息列表.value = []
  void 加载群聊()
}, { immediate: true })

onBeforeUnmount(取消实时订阅)
</script>

<template>
  <section class="page-panel group-chat-page">
    <header class="group-chat-heading">
      <button
        class="button button-quiet"
        @click="router.push({ name: 群组?.关联任务编号 ? 'messages' : 'groups' })"
      >← {{ 群组?.关联任务编号 ? '消息列表' : '群聊列表' }}</button>
      <div v-if="群组" class="group-chat-title">
        <div class="section-eyebrow">
          {{ 群组.关联任务编号 ? '委托沟通 · ' : '' }}{{ 群组.成员列表.length }} 位成员
        </div>
        <h1>{{ 群组.群组名称 }}</h1>
        <p v-if="群组.群组公告" class="muted">{{ 群组.群组公告 }}</p>
      </div>
      <button
        v-if="群组 && !群组.关联任务编号 && 当前用户?.用户编号 !== 群组.群主编号"
        class="button button-quiet"
        @click="退出群组"
      >退出群聊</button>
    </header>

    <div v-if="错误信息" class="group-feedback error-notice" role="alert">{{ 错误信息 }}</div>
    <div v-if="邀请反馈" class="group-feedback info-notice" role="status">{{ 邀请反馈 }}</div>

    <div v-if="群组" class="group-chat-layout">
      <section class="group-chat-main">
        <div ref="消息区域" class="message-list group-message-list" aria-live="polite">
          <div v-if="正在加载" class="chat-empty">正在加载群消息…</div>
          <div v-else-if="!消息列表.length" class="chat-empty">还没有消息，向群里打个招呼吧。</div>
          <article
            v-for="消息 in 消息列表"
            :key="消息.消息编号"
            class="message"
            :class="{ mine: 消息.发送者编号 === 当前用户?.用户编号 }"
          >
            <div class="message-meta">
              {{ 消息.发送者编号 === 当前用户?.用户编号 ? '我' : 消息.发送者昵称 }}
              · {{ new Date(消息.创建时间).toLocaleString() }}
            </div>
            <p>{{ 消息.消息正文 }}</p>
          </article>
        </div>
        <form class="chat-compose full-chat-compose" @submit.prevent="发送消息">
          <input
            v-model="消息正文"
            maxlength="4000"
            placeholder="输入群消息…"
            aria-label="群消息内容"
            :disabled="正在发送"
          />
          <button class="button button-primary" :disabled="正在发送 || !消息正文.trim()">
            {{ 正在发送 ? '发送中…' : '发送' }}
          </button>
        </form>
      </section>

      <aside class="group-members-panel">
        <h2>{{ 群组.关联任务编号 ? '委托参与者' : '群成员' }}</h2>
        <form
          v-if="!群组.关联任务编号 && 群组.成员列表.some((成员) =>
            成员.用户编号 === 当前用户?.用户编号 && ['群主', '管理员'].includes(成员.成员角色))"
          class="group-invite-form"
          @submit.prevent="邀请好友"
        >
          <select v-model="选择好友编号" aria-label="选择要邀请的好友">
            <option value="">选择好友邀请</option>
            <option
              v-for="好友 in 好友列表.filter((候选) =>
                !群组?.成员列表.some((成员) => 成员.用户编号 === 候选.联系人编号))"
              :key="好友.联系人编号"
              :value="好友.联系人编号"
            >{{ 好友.联系人昵称 }}</option>
          </select>
          <button class="button button-outline button-small" :disabled="正在邀请 || !选择好友编号">
            {{ 正在邀请 ? '邀请中…' : '邀请' }}
          </button>
        </form>
        <p v-if="!好友列表.length" class="muted">添加好友后才能邀请入群。</p>
        <ul class="group-member-list">
          <li v-for="成员 in 群组.成员列表" :key="成员.用户编号">
            <span class="group-member-copy">
              <strong>{{ 成员.昵称 }}</strong>
              <small>{{ 成员.成员角色 }}</small>
            </span>
            <button
              v-if="!群组.关联任务编号 && 成员.用户编号 !== 当前用户?.用户编号 &&
                群组.成员列表.some((当前成员) =>
                  当前成员.用户编号 === 当前用户?.用户编号 &&
                  ['群主', '管理员'].includes(当前成员.成员角色)) &&
                成员.成员角色 !== '群主'"
              class="button button-quiet button-small"
              @click="移除成员(成员)"
            >移除</button>
          </li>
        </ul>
      </aside>
    </div>

    <div v-else-if="正在加载" class="empty-state">正在加载群组…</div>
    <div v-else class="empty-state">
      <h3>无法打开这个群聊</h3>
      <p>群组不存在，或你不是有效成员。</p>
      <button class="button button-outline" @click="router.push({ name: 'groups' })">返回群聊列表</button>
    </div>
  </section>
</template>
