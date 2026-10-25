<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { 发送请求 } from '../接口/请求'
import type { 用户 } from '../领域/类型'

type 好友 = {
  用户编号: string
  登录名: string
  昵称: string
  建立时间: string
}

type 好友申请 = {
  申请编号: string
  申请人编号: string
  登录名: string
  昵称: string
  申请附言: string
  创建时间: string
}

type 用户搜索结果 = {
  用户编号: string
  登录名: string
  昵称: string
  好友状态: '已是好友' | '待你处理' | '已发送申请' | '可添加'
}

const props = defineProps<{ 当前用户: 用户 | null }>()
const router = useRouter()
const 好友列表 = ref<好友[]>([])
const 收到的申请列表 = ref<好友申请[]>([])
const 搜索结果 = ref<用户搜索结果 | null>(null)
const 登录名查询 = ref('')
const 申请附言 = ref('')
const 正在加载 = ref(false)
const 正在搜索 = ref(false)
const 正在处理编号 = ref('')
const 错误信息 = ref('')
const 提示信息 = ref('')

function 获取令牌() {
  return localStorage.getItem('renwu_token') || ''
}

async function 加载联系人数据() {
  if (!props.当前用户) {
    好友列表.value = []
    收到的申请列表.value = []
    return
  }
  正在加载.value = true
  错误信息.value = ''
  try {
    const 令牌 = 获取令牌()
    const [好友, 申请] = await Promise.all([
      发送请求<好友[]>('/my/friends', {}, 令牌),
      发送请求<好友申请[]>('/my/friend-requests', {}, 令牌),
    ])
    好友列表.value = 好友
    收到的申请列表.value = 申请
  } catch (原因) {
    错误信息.value = 原因 instanceof Error ? 原因.message : '加载联系人失败'
  } finally {
    正在加载.value = false
  }
}

async function 搜索用户() {
  const 登录名 = 登录名查询.value.trim()
  if (!登录名) {
    错误信息.value = '请输入要搜索的完整登录名'
    return
  }
  正在搜索.value = true
  错误信息.value = ''
  提示信息.value = ''
  搜索结果.value = null
  try {
    const 查询 = new URLSearchParams({ login_name: 登录名 })
    const 结果 = await 发送请求<用户搜索结果[]>(
      `/my/friends/search?${查询.toString()}`,
      {},
      获取令牌(),
    )
    搜索结果.value = 结果[0] ?? null
    if (!搜索结果.value) 提示信息.value = '没有找到该登录名对应的用户'
  } catch (原因) {
    错误信息.value = 原因 instanceof Error ? 原因.message : '搜索用户失败'
  } finally {
    正在搜索.value = false
  }
}

async function 发送好友申请(用户: 用户搜索结果) {
  正在处理编号.value = 用户.用户编号
  错误信息.value = ''
  提示信息.value = ''
  try {
    await 发送请求('/my/friend-requests', {
      method: 'POST',
      body: JSON.stringify({
        target_user_id: 用户.用户编号,
        message: 申请附言.value.trim(),
      }),
    }, 获取令牌())
    申请附言.value = ''
    await 搜索用户()
    提示信息.value = '好友申请已发送'
    await 加载联系人数据()
  } catch (原因) {
    错误信息.value = 原因 instanceof Error ? 原因.message : '发送好友申请失败'
  } finally {
    正在处理编号.value = ''
  }
}

async function 处理好友申请(申请: 好友申请, 操作: 'accept' | 'reject') {
  正在处理编号.value = 申请.申请编号
  错误信息.value = ''
  提示信息.value = ''
  try {
    await 发送请求(`/my/friend-requests/${申请.申请编号}/${操作}`, {
      method: 'POST',
    }, 获取令牌())
    提示信息.value = 操作 === 'accept' ? `已添加 ${申请.昵称} 为好友` : '已拒绝好友申请'
    await 加载联系人数据()
  } catch (原因) {
    错误信息.value = 原因 instanceof Error ? 原因.message : '处理好友申请失败'
  } finally {
    正在处理编号.value = ''
  }
}

async function 删除好友(好友项: 好友) {
  if (!window.confirm(`确定删除好友 ${好友项.昵称} 吗？历史聊天记录会保留。`)) return
  正在处理编号.value = 好友项.用户编号
  错误信息.value = ''
  try {
    await 发送请求(`/my/friends/${好友项.用户编号}`, { method: 'DELETE' }, 获取令牌())
    await 加载联系人数据()
  } catch (原因) {
    错误信息.value = 原因 instanceof Error ? 原因.message : '删除好友失败'
  } finally {
    正在处理编号.value = ''
  }
}

function 打开私聊(好友项: Pick<好友, '用户编号' | '昵称'>) {
  void router.push({
    name: 'friend-chat',
    params: { friendId: 好友项.用户编号 },
    query: { nickname: 好友项.昵称 },
  })
}

function 格式化时间(时间: string) {
  const 时间戳 = new Date(时间)
  return Number.isNaN(时间戳.getTime()) ? 时间 : 时间戳.toLocaleDateString()
}

onMounted(() => void 加载联系人数据())
watch(() => props.当前用户?.用户编号, () => void 加载联系人数据())
</script>

<template>
  <section class="page-panel contacts-page">
    <div class="welcome-row">
      <div class="page-heading">
        <div class="section-eyebrow">好友与私聊</div>
        <h1>联系人</h1>
        <p class="muted">管理好友、处理申请，或按登录名查找用户。</p>
      </div>
      <button
        v-if="当前用户"
        class="button button-quiet"
        :disabled="正在加载"
        @click="加载联系人数据"
      >{{ 正在加载 ? '加载中…' : '刷新 ↻' }}</button>
    </div>

    <div v-if="错误信息" class="contacts-feedback contacts-error" role="alert">{{ 错误信息 }}</div>
    <div v-if="提示信息" class="contacts-feedback contacts-success" role="status">{{ 提示信息 }}</div>

    <div v-if="当前用户" class="contacts-columns">
      <div class="contacts-main-column">
        <section class="contacts-card">
          <div class="contacts-card-heading">
            <div><div class="section-eyebrow">查找用户</div><h2>添加好友</h2></div>
          </div>
          <form class="contacts-search" @submit.prevent="搜索用户">
            <label for="friend-login-search">用户登录名</label>
            <div class="contacts-search-row">
              <input
                id="friend-login-search"
                v-model="登录名查询"
                maxlength="64"
                autocomplete="off"
                placeholder="输入完整登录名"
              />
              <button class="button button-primary" :disabled="正在搜索">
                {{ 正在搜索 ? '搜索中…' : '搜索' }}
              </button>
            </div>
          </form>
          <div v-if="搜索结果" class="contact-search-result">
            <div class="contact-avatar">{{ 搜索结果.昵称.slice(0, 1) }}</div>
            <div class="contact-copy">
              <strong>{{ 搜索结果.昵称 }}</strong>
              <small>@{{ 搜索结果.登录名 }}</small>
            </div>
            <form
              v-if="搜索结果.好友状态 === '可添加'"
              class="contact-add-form"
              @submit.prevent="发送好友申请(搜索结果)"
            >
              <input
                v-model="申请附言"
                maxlength="200"
                aria-label="好友申请附言"
                placeholder="申请附言（可选）"
              />
              <button
                class="button button-primary button-small"
                :disabled="正在处理编号 === 搜索结果.用户编号"
              >{{ 正在处理编号 === 搜索结果.用户编号 ? '发送中…' : '添加' }}</button>
            </form>
            <span v-else class="contact-status">{{ 搜索结果.好友状态 }}</span>
          </div>
        </section>

        <section class="contacts-card">
          <div class="contacts-card-heading">
            <div><div class="section-eyebrow">已建立联系</div><h2>我的好友 <span>{{ 好友列表.length }}</span></h2></div>
          </div>
          <div v-if="好友列表.length" class="contacts-list">
            <article v-for="好友项 in 好友列表" :key="好友项.用户编号" class="contact-row">
              <button class="contact-open" @click="打开私聊(好友项)">
                <span class="contact-avatar">{{ 好友项.昵称.slice(0, 1) }}</span>
                <span class="contact-copy">
                  <strong>{{ 好友项.昵称 }}</strong>
                  <small>@{{ 好友项.登录名 }} · 好友于 {{ 格式化时间(好友项.建立时间) }}</small>
                </span>
                <span class="contact-chat-hint">私聊 ›</span>
              </button>
              <button
                class="contact-remove"
                :disabled="正在处理编号 === 好友项.用户编号"
                :aria-label="`删除好友 ${好友项.昵称}`"
                @click="删除好友(好友项)"
              >删除</button>
            </article>
          </div>
          <div v-else class="contacts-empty">
            {{ 正在加载 ? '正在加载好友…' : '还没有好友，搜索登录名来添加。' }}
          </div>
        </section>
      </div>

      <section class="contacts-card requests-card">
        <div class="contacts-card-heading">
          <div><div class="section-eyebrow">等待你的处理</div><h2>好友申请 <span>{{ 收到的申请列表.length }}</span></h2></div>
        </div>
        <div v-if="收到的申请列表.length" class="requests-list">
          <article v-for="申请 in 收到的申请列表" :key="申请.申请编号" class="request-row">
            <div class="contact-row">
              <span class="contact-avatar">{{ 申请.昵称.slice(0, 1) }}</span>
              <div class="contact-copy">
                <strong>{{ 申请.昵称 }}</strong>
                <small>@{{ 申请.登录名 }} · {{ 格式化时间(申请.创建时间) }}</small>
              </div>
            </div>
            <p v-if="申请.申请附言" class="request-message">{{ 申请.申请附言 }}</p>
            <div class="request-actions">
              <button
                class="button button-primary button-small"
                :disabled="正在处理编号 === 申请.申请编号"
                @click="处理好友申请(申请, 'accept')"
              >同意</button>
              <button
                class="button button-quiet button-small"
                :disabled="正在处理编号 === 申请.申请编号"
                @click="处理好友申请(申请, 'reject')"
              >拒绝</button>
            </div>
          </article>
        </div>
        <div v-else class="contacts-empty">
          {{ 正在加载 ? '正在加载申请…' : '暂无待处理的好友申请。' }}
        </div>
      </section>
    </div>

    <div v-else class="empty-state access-prompt">
      <div class="empty-icon">◉</div>
      <h3>登录后使用联系人</h3>
      <p>登录后即可查找用户、添加好友并开始私聊。</p>
      <button class="button button-primary" @click="router.push({ name: 'profile' })">前往登录</button>
    </div>
  </section>
</template>

<style scoped>
.contacts-page{max-width:1080px}
.contacts-columns{display:grid;grid-template-columns:minmax(0,1.35fr) minmax(270px,.8fr);align-items:start;gap:16px}
.contacts-main-column{display:grid;gap:16px}
.contacts-card{min-width:0;padding:20px;border:1px solid var(--line);border-radius:14px;background:#fff}
.contacts-card-heading{display:flex;align-items:center;justify-content:space-between;margin-bottom:17px}
.contacts-card-heading h2{margin:6px 0 0;font-size:17px}
.contacts-card-heading h2 span{margin-left:4px;color:var(--muted);font-size:12px;font-weight:500}
.contacts-search label{display:block;margin-bottom:7px;color:#566460;font-size:11px;font-weight:600}
.contacts-search-row{display:flex;gap:8px}
.contacts-search-row input{min-width:0}
.contact-search-result,.contact-row{display:flex;align-items:center;gap:11px;min-width:0}
.contact-search-result{margin-top:15px;padding:13px;border:1px solid var(--line);border-radius:10px}
.contact-avatar{display:grid;place-items:center;flex:none;width:40px;height:40px;border-radius:50%;background:var(--mint);color:var(--green);font-size:16px;font-weight:700}
.contact-copy{display:grid;gap:5px;min-width:0}
.contact-copy strong,.contact-copy small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.contact-copy strong{font-size:12px}
.contact-copy small{color:var(--muted);font-size:10px}
.contact-add-form{display:flex;flex:1;gap:6px;min-width:0}
.contact-add-form input{min-width:0;padding:9px;font-size:11px}
.contact-status{margin-left:auto;color:var(--muted);font-size:11px;white-space:nowrap}
.contacts-list,.requests-list{display:grid}
.contact-row{padding:12px 0;border-top:1px solid #f0f2f1}
.contact-open{display:flex;flex:1;align-items:center;gap:11px;min-width:0;padding:0;border:0;background:transparent;text-align:left}
.contact-chat-hint{margin-left:auto;color:var(--green);font-size:11px;white-space:nowrap}
.contact-remove{padding:6px 8px;border:0;border-radius:7px;background:transparent;color:#9b6d66;font-size:10px}
.contact-remove:hover:not(:disabled){background:#fff0ee}
.contacts-empty{padding:20px 8px;color:var(--muted);font-size:12px;text-align:center}
.request-row{padding:12px 0;border-top:1px solid #f0f2f1}
.request-message{margin:9px 0 0 51px;color:#687570;font-size:11px;line-height:1.6;white-space:pre-wrap;overflow-wrap:anywhere}
.request-actions{display:flex;justify-content:flex-end;gap:6px;margin-top:10px}
.contacts-feedback{margin-bottom:14px;padding:11px 13px;border-radius:9px;font-size:12px}
.contacts-error{background:#fff0ec;color:#a84229}
.contacts-success{background:#e7f4ef;color:#126b56}
@media(max-width:760px){.contacts-columns{grid-template-columns:1fr}.contacts-card{padding:16px}}
@media(max-width:480px){.contacts-search-result{flex-wrap:wrap}.contact-add-form{flex-basis:100%;margin-left:51px}.contact-status{margin-left:auto}}
</style>
