<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { 发送请求 } from '../接口/请求'
import { 订阅实时事件 } from '../实时/消息中心'
import type { 群邀请, 群组, 用户 } from '../领域/类型'

const props = defineProps<{ 当前用户: 用户 | null }>()
const router = useRouter()
const 群组列表 = ref<群组[]>([])
const 邀请列表 = ref<群邀请[]>([])
const 群组名称 = ref('')
const 正在加载 = ref(false)
const 正在处理 = ref(false)
const 错误信息 = ref('')
const 提示信息 = ref('')

function 获取令牌() {
  return localStorage.getItem('renwu_token') || ''
}

async function 加载群聊() {
  if (!props.当前用户) {
    群组列表.value = []
    邀请列表.value = []
    return
  }
  正在加载.value = true
  错误信息.value = ''
  try {
    const 令牌 = 获取令牌()
    const [群组, 邀请] = await Promise.all([
      发送请求<群组[]>('/my/groups', {}, 令牌),
      发送请求<群邀请[]>('/my/group-invitations', {}, 令牌),
    ])
    群组列表.value = 群组
    邀请列表.value = 邀请
  } catch (原因) {
    错误信息.value = 原因 instanceof Error ? 原因.message : '加载群聊失败'
  } finally {
    正在加载.value = false
  }
}

async function 创建群组() {
  const 名称 = 群组名称.value.trim()
  if (!名称 || 正在处理.value) return
  正在处理.value = true
  错误信息.value = ''
  try {
    const 结果 = await 发送请求<{ 群组编号: string }>('/groups', {
      method: 'POST',
      body: JSON.stringify({ name: 名称 }),
    }, 获取令牌())
    群组名称.value = ''
    await 加载群聊()
    await router.push({ name: 'group-chat', params: { groupId: 结果.群组编号 } })
  } catch (原因) {
    错误信息.value = 原因 instanceof Error ? 原因.message : '创建群组失败'
  } finally {
    正在处理.value = false
  }
}

async function 处理邀请(邀请: 群邀请, 操作: 'accept' | 'reject') {
  正在处理.value = true
  错误信息.value = ''
  提示信息.value = ''
  try {
    await 发送请求(`/group-invitations/${邀请.邀请编号}/${操作}`, {
      method: 'POST',
    }, 获取令牌())
    await 加载群聊()
    if (操作 === 'accept') {
      提示信息.value = `已加入「${邀请.群组名称}」`
      await router.push({ name: 'group-chat', params: { groupId: 邀请.群组编号 } })
    } else {
      提示信息.value = '已拒绝群邀请'
    }
  } catch (原因) {
    错误信息.value = 原因 instanceof Error ? 原因.message : '处理群邀请失败'
  } finally {
    正在处理.value = false
  }
}

function 处理实时事件(事件: { 事件类型: string; 数据?: Record<string, unknown> }) {
  if (事件.事件类型 === 'group.invitation_created') {
    void 加载群聊()
    return
  }
  if (事件.事件类型 !== 'group.message_created' || !事件.数据) return
  const 数据 = 事件.数据
  const 群组编号 = 数据.群组编号
  const 消息正文 = 数据.消息正文
  const 创建时间 = 数据.创建时间
  if (typeof 群组编号 !== 'string' || typeof 消息正文 !== 'string') return
  const 会话 = 群组列表.value.find((项目) => 项目.群组编号 === 群组编号)
  if (!会话) return
  会话.最近消息 = 消息正文
  会话.最近消息时间 = typeof 创建时间 === 'string' ? 创建时间 : new Date().toISOString()
  const 消息序号 = 数据.群内序号
  if (typeof 消息序号 === 'number') {
    会话.最后消息序号 = Math.max(会话.最后消息序号, 消息序号)
  }
  群组列表.value = [...群组列表.value].sort((左侧, 右侧) =>
    (右侧.最近消息时间 ?? '').localeCompare(左侧.最近消息时间 ?? ''),
  )
}

const 取消实时订阅 = 订阅实时事件(处理实时事件)

onMounted(() => void 加载群聊())
watch(() => props.当前用户?.用户编号, () => void 加载群聊())
onBeforeUnmount(取消实时订阅)
</script>

<template>
  <section class="page-panel">
    <div class="welcome-row">
      <div class="page-heading">
        <div class="section-eyebrow">独立群聊</div>
        <h1>群聊</h1>
        <p class="muted">创建群组、处理邀请，与群成员实时交流。</p>
      </div>
      <button class="button button-quiet" :disabled="正在加载" @click="加载群聊">
        {{ 正在加载 ? '加载中…' : '刷新 ↻' }}
      </button>
    </div>

    <div v-if="!当前用户" class="empty-state">
      <h3>登录后使用群聊</h3>
      <p>登录后即可创建群组或接受好友邀请。</p>
    </div>
    <template v-else>
      <p v-if="错误信息" class="group-feedback error-notice" role="alert">{{ 错误信息 }}</p>
      <p v-if="提示信息" class="group-feedback info-notice" role="status">{{ 提示信息 }}</p>

      <form class="group-create-form" @submit.prevent="创建群组">
        <label>
          新群组名称
          <input v-model="群组名称" maxlength="80" placeholder="给群聊起个名字" />
        </label>
        <button class="button button-primary" :disabled="正在处理 || !群组名称.trim()">
          {{ 正在处理 ? '处理中…' : '创建群聊' }}
        </button>
      </form>

      <section v-if="邀请列表.length" class="group-section">
        <div class="list-heading">
          <div><div class="section-eyebrow">待处理</div><h2>群聊邀请</h2></div>
          <span class="count-pill">{{ 邀请列表.length }} 个邀请</span>
        </div>
        <article v-for="邀请 in 邀请列表" :key="邀请.邀请编号" class="group-invitation">
          <div>
            <strong>{{ 邀请.群组名称 }}</strong>
            <small>由 {{ 邀请.邀请人昵称 }} 邀请 · {{ new Date(邀请.创建时间).toLocaleString() }}</small>
          </div>
          <div class="group-actions">
            <button class="button button-primary button-small" :disabled="正在处理" @click="处理邀请(邀请, 'accept')">加入</button>
            <button class="button button-quiet button-small" :disabled="正在处理" @click="处理邀请(邀请, 'reject')">拒绝</button>
          </div>
        </article>
      </section>

      <section class="group-section">
        <div class="list-heading">
          <div><div class="section-eyebrow">我的会话</div><h2>群组列表</h2></div>
          <span class="count-pill">{{ 群组列表.length }} 个群组</span>
        </div>
        <div v-if="群组列表.length" class="conversation-list conversation-list-page">
          <button
            v-for="群组 in 群组列表"
            :key="群组.群组编号"
            class="conversation-item conversation-row"
            @click="router.push({ name: 'group-chat', params: { groupId: 群组.群组编号 } })"
          >
            <span class="conversation-avatar">群</span>
            <span class="conversation-copy">
              <strong>{{ 群组.群组名称 }} <small>· {{ 群组.成员数 }} 人</small></strong>
              <small>{{ 群组.最近消息 || 群组.群组公告 || '还没有消息' }}</small>
            </span>
            <span v-if="群组.最近消息时间" class="conversation-meta">
              <small>{{ new Date(群组.最近消息时间).toLocaleString() }}</small>
            </span>
          </button>
        </div>
        <div v-else class="empty-state">
          <h3>{{ 正在加载 ? '正在加载群组' : '还没有群聊' }}</h3>
          <p>创建一个群组，或接受好友发来的群聊邀请。</p>
        </div>
      </section>
    </template>
  </section>
</template>
