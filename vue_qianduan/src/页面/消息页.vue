<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { 发送请求 } from '../接口/请求'
import { 订阅实时事件 } from '../实时/消息中心'
import type { 消息会话, 用户 } from '../领域/类型'

type 好友申请 = {
  申请编号: string
  申请人编号: string
  登录名: string
  昵称: string
  申请附言: string
  创建时间: string
}

type 消息列表项 =
  | { 类型: '会话'; 键: string; 时间: string; 会话: 消息会话 }
  | { 类型: '好友申请'; 键: string; 时间: string; 申请: 好友申请 }

const 属性 = defineProps<{
  当前用户: 用户 | null
  消息会话列表: 消息会话[]
  正在加载: boolean
  格式化日期: (日期: string) => string
}>()

const emit = defineEmits<{
  刷新: []
  打开会话: [会话: 消息会话]
  前往个人中心: []
}>()

type 消息分类 = '全部' | '好友验证' | 消息会话['会话类型']
const 当前分类 = ref<消息分类>('全部')
const 搜索词 = ref('')
const 好友申请列表 = ref<好友申请[]>([])
const 正在加载申请 = ref(false)
const 正在处理申请编号 = ref('')
const 申请错误 = ref('')
const 申请反馈 = ref('')
let 申请加载序号 = 0

const 分类列表 = computed(() => [
  { 名称: '全部' as const, 数量: 属性.消息会话列表.length + 好友申请列表.value.length },
  { 名称: '好友验证' as const, 数量: 好友申请列表.value.length },
  {
    名称: '好友' as const,
    数量: 属性.消息会话列表.filter((会话) => 会话.会话类型 === '好友').length,
  },
  {
    名称: '群聊' as const,
    数量: 属性.消息会话列表.filter((会话) => 会话.会话类型 === '群聊').length,
  },
  {
    名称: '委托' as const,
    数量: 属性.消息会话列表.filter((会话) => 会话.会话类型 === '委托').length,
  },
])

const 消息列表 = computed<消息列表项[]>(() => [
  ...属性.消息会话列表.map((会话) => ({
    类型: '会话' as const,
    键: `${会话.会话类型}:${会话.会话编号}`,
    时间: 会话.最近消息时间,
    会话,
  })),
  ...好友申请列表.value.map((申请) => ({
    类型: '好友申请' as const,
    键: `好友申请:${申请.申请编号}`,
    时间: 申请.创建时间,
    申请,
  })),
].sort((左侧, 右侧) => new Date(右侧.时间).getTime() - new Date(左侧.时间).getTime()))

const 筛选后消息 = computed(() => {
  const 查询词 = 搜索词.value.trim().toLocaleLowerCase()
  return 消息列表.value.filter((消息) => {
    if (消息.类型 === '好友申请') {
      const 分类匹配 = 当前分类.value === '全部' || 当前分类.value === '好友验证'
      const 搜索匹配 = !查询词 || [
        消息.申请.昵称,
        消息.申请.登录名,
        消息.申请.申请附言,
      ].some((内容) => 内容.toLocaleLowerCase().includes(查询词))
      return 分类匹配 && 搜索匹配
    }
    const 会话 = 消息.会话
    const 分类匹配 = 当前分类.value === '全部' || 会话.会话类型 === 当前分类.value
    const 搜索匹配 = !查询词 || [
      会话.会话名称,
      会话.联系人昵称 ?? '',
      会话.会话说明,
      会话.最近消息 ?? '',
    ].some((内容) => 内容.toLocaleLowerCase().includes(查询词))
    return 分类匹配 && 搜索匹配
  })
})

async function 加载好友申请() {
  const 用户编号 = 属性.当前用户?.用户编号
  const 加载序号 = ++申请加载序号
  if (!用户编号) {
    好友申请列表.value = []
    申请错误.value = ''
    正在加载申请.value = false
    return
  }

  正在加载申请.value = true
  try {
    const 申请 = await 发送请求<好友申请[]>(
      '/my/friend-requests',
      {},
      localStorage.getItem('renwu_token') || '',
    )
    if (加载序号 === 申请加载序号) {
      好友申请列表.value = 申请
      申请错误.value = ''
    }
  } catch (原因) {
    if (加载序号 === 申请加载序号) {
      申请错误.value = 原因 instanceof Error ? 原因.message : '加载好友申请失败'
    }
  } finally {
    if (加载序号 === 申请加载序号) 正在加载申请.value = false
  }
}

async function 处理好友申请(申请: 好友申请, 操作: 'accept' | 'reject') {
  正在处理申请编号.value = 申请.申请编号
  申请错误.value = ''
  申请反馈.value = ''
  try {
    await 发送请求(
      `/my/friend-requests/${encodeURIComponent(申请.申请编号)}/${操作}`,
      { method: 'POST' },
      localStorage.getItem('renwu_token') || '',
    )
    申请反馈.value = 操作 === 'accept' ? `已添加 ${申请.昵称} 为好友` : '已拒绝好友申请'
    await 加载好友申请()
    if (操作 === 'accept') emit('刷新')
  } catch (原因) {
    申请错误.value = 原因 instanceof Error ? 原因.message : '处理好友申请失败'
  } finally {
    正在处理申请编号.value = ''
  }
}

function 刷新消息() {
  emit('刷新')
  void 加载好友申请()
}

watch(() => 属性.当前用户?.用户编号, () => void 加载好友申请(), { immediate: true })

const 取消好友申请订阅 = 订阅实时事件((事件) => {
  if (事件.事件类型.startsWith('friend.request_')) void 加载好友申请()
})

onBeforeUnmount(取消好友申请订阅)
</script>

<template>
  <section class="im-inbox">
    <header class="im-inbox-header">
      <div>
        <div class="section-eyebrow">消息中心</div>
        <h1>消息</h1>
        <p>好友私聊、群聊和委托沟通都在这里。</p>
      </div>
      <button
        v-if="当前用户"
        class="button button-quiet im-refresh"
        :disabled="正在加载 || 正在加载申请"
        aria-label="刷新会话"
        @click="刷新消息"
      >
        {{ 正在加载 || 正在加载申请 ? '同步中…' : '刷新 ↻' }}
      </button>
    </header>

    <template v-if="当前用户">
      <div v-if="申请错误" class="im-request-feedback im-request-error" role="alert">{{ 申请错误 }}</div>
      <div v-if="申请反馈" class="im-request-feedback im-request-success" role="status">{{ 申请反馈 }}</div>
      <div class="im-toolbar">
        <label class="im-search">
          <span aria-hidden="true">⌕</span>
          <input v-model="搜索词" type="search" placeholder="搜索会话或消息" aria-label="搜索会话或消息">
        </label>
        <div class="im-filters" role="tablist" aria-label="会话分类">
          <button
            v-for="分类 in 分类列表"
            :key="分类.名称"
            type="button"
            role="tab"
            :aria-selected="当前分类 === 分类.名称"
            :class="{ active: 当前分类 === 分类.名称 }"
            @click="当前分类 = 分类.名称"
          >
            {{ 分类.名称 }}<span>{{ 分类.数量 }}</span>
          </button>
        </div>
      </div>

      <div v-if="筛选后消息.length" class="im-conversation-list">
        <template v-for="消息 in 筛选后消息" :key="消息.键">
          <article v-if="消息.类型 === '好友申请'" class="im-conversation im-friend-request">
            <span class="im-avatar im-avatar-request">{{ 消息.申请.昵称.slice(0, 1) }}</span>
            <span class="im-conversation-copy">
              <span class="im-conversation-topline">
                <strong>{{ 消息.申请.昵称 }} 的好友验证</strong>
                <time>{{ 格式化日期(消息.申请.创建时间) }}</time>
              </span>
              <span class="im-conversation-preview">
                <span>@{{ 消息.申请.登录名 }} · {{ 消息.申请.申请附言 || '请求添加你为好友' }}</span>
                <span class="im-conversation-tags"><small class="im-tag">待处理</small></span>
              </span>
            </span>
            <span class="im-request-actions">
              <button
                class="button button-primary button-small"
                :disabled="正在处理申请编号 === 消息.申请.申请编号"
                @click="处理好友申请(消息.申请, 'accept')"
              >同意</button>
              <button
                class="button button-quiet button-small"
                :disabled="正在处理申请编号 === 消息.申请.申请编号"
                @click="处理好友申请(消息.申请, 'reject')"
              >拒绝</button>
            </span>
          </article>
          <button
            v-else
            class="im-conversation"
            type="button"
            @click="emit('打开会话', 消息.会话)"
          >
            <span class="im-avatar" :class="`im-avatar-${消息.会话.会话类型}`">
              {{ 消息.会话.会话类型 === '群聊' ? '群' : 消息.会话.会话名称.slice(0, 1) }}
            </span>
            <span class="im-conversation-copy">
              <span class="im-conversation-topline">
                <strong>{{ 消息.会话.会话名称 }}</strong>
                <time>{{ 格式化日期(消息.会话.最近消息时间) }}</time>
              </span>
              <span class="im-conversation-preview">
                <span>{{ 消息.会话.最近消息 || (消息.会话.会话类型 === '好友' ? '开始聊天吧' : '暂无消息') }}</span>
                <span class="im-conversation-tags">
                  <small v-if="消息.会话.会话类型 === '委托' && 消息.会话.会话状态" class="im-tag">
                    {{ 消息.会话.会话状态 }}
                  </small>
                  <small v-else-if="消息.会话.会话类型 === '群聊'" class="im-tag">
                    {{ 消息.会话.成员数 ?? 0 }} 人
                  </small>
                  <small v-else-if="消息.会话.会话类型 === '好友'" class="im-tag">好友</small>
                </span>
              </span>
            </span>
          </button>
        </template>
      </div>
      <div v-else class="im-empty">
        <span class="im-empty-icon">◌</span>
        <strong>{{ 正在加载 || 正在加载申请 ? '正在同步消息' : 搜索词 ? '没有找到匹配的消息' : '还没有消息' }}</strong>
        <p>
          {{ 搜索词 ? '试试其他名称或消息关键词。' : '好友验证、好友私聊、群聊和委托沟通都会集中显示在这里。' }}
        </p>
      </div>
    </template>
    <div v-else class="im-empty">
      <span class="im-empty-icon">◉</span>
      <strong>登录后查看消息</strong>
      <p>登录后即可集中查看好友、群聊和委托会话。</p>
      <button class="button button-primary" @click="emit('前往个人中心')">前往登录</button>
    </div>
  </section>
</template>

<style scoped>
.im-request-feedback{margin-bottom:12px;padding:10px 12px;border-radius:9px;font-size:12px}
.im-request-error{background:#fff0ec;color:#a84229}
.im-request-success{background:#e7f4ef;color:#126b56}
.im-friend-request{cursor:default}
.im-friend-request:hover{background:#fff}
.im-avatar-request{background:#fff1e5;color:#b87936}
.im-request-actions{display:flex;flex:none;gap:6px}
@media(max-width:680px){
  .im-friend-request{flex-wrap:wrap}
  .im-friend-request .im-conversation-copy{min-width:calc(100% - 56px);flex:1}
  .im-request-actions{width:100%;justify-content:flex-end}
}
</style>
