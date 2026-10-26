<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { 发送请求 } from '../接口/请求'
import { 订阅实时事件 } from '../实时/消息中心'
import type { 群邀请, 群组, 用户 } from '../领域/类型'

type 好友 = {
  用户编号: string
  登录名: string
  昵称: string
  建立时间: string
}

type 联系人分类 = '全部' | '好友' | '群聊' | '群邀请'
type 联系人项 =
  | { 类型: '好友'; 编号: string; 名称: string; 搜索内容: string; 时间: string; 好友: 好友 }
  | { 类型: '群聊'; 编号: string; 名称: string; 搜索内容: string; 时间: string; 群组: 群组 }
  | { 类型: '群邀请'; 编号: string; 名称: string; 搜索内容: string; 时间: string; 邀请: 群邀请 }

const props = defineProps<{ 当前用户: 用户 | null }>()
const router = useRouter()
const 好友列表 = ref<好友[]>([])
const 群组列表 = ref<群组[]>([])
const 邀请列表 = ref<群邀请[]>([])
const 正在加载 = ref(false)
const 正在处理编号 = ref('')
const 错误信息 = ref('')
const 提示信息 = ref('')
const 搜索词 = ref('')
const 当前分类 = ref<联系人分类>('全部')

const 分类列表 = computed(() => [
  { 名称: '全部' as const, 数量: 好友列表.value.length + 群组列表.value.length + 邀请列表.value.length },
  { 名称: '好友' as const, 数量: 好友列表.value.length },
  { 名称: '群聊' as const, 数量: 群组列表.value.length },
  { 名称: '群邀请' as const, 数量: 邀请列表.value.length },
])

const 联系人项目 = computed<联系人项[]>(() => [
  ...好友列表.value.map((好友项) => ({
    类型: '好友' as const,
    编号: `friend:${好友项.用户编号}`,
    名称: 好友项.昵称,
    搜索内容: `${好友项.昵称} ${好友项.登录名}`,
    时间: 好友项.建立时间,
    好友: 好友项,
  })),
  ...群组列表.value.map((群组) => ({
    类型: '群聊' as const,
    编号: `group:${群组.群组编号}`,
    名称: 群组.群组名称,
    搜索内容: `${群组.群组名称} ${群组.最近消息 ?? ''} ${群组.群组公告}`,
    时间: 群组.最近消息时间 ?? '',
    群组,
  })),
  ...邀请列表.value.map((邀请) => ({
    类型: '群邀请' as const,
    编号: `invitation:${邀请.邀请编号}`,
    名称: 邀请.群组名称,
    搜索内容: `${邀请.群组名称} ${邀请.邀请人昵称}`,
    时间: 邀请.创建时间,
    邀请,
  })),
].sort((左侧, 右侧) => 右侧.时间.localeCompare(左侧.时间)))

const 筛选后项目 = computed(() => {
  const 查询词 = 搜索词.value.trim().toLocaleLowerCase()
  return 联系人项目.value.filter((项目) =>
    (当前分类.value === '全部' || 项目.类型 === 当前分类.value) &&
    (!查询词 || 项目.搜索内容.toLocaleLowerCase().includes(查询词)),
  )
})

function 获取令牌() {
  return localStorage.getItem('renwu_token') || ''
}

async function 加载联系人() {
  if (!props.当前用户) {
    好友列表.value = []
    群组列表.value = []
    邀请列表.value = []
    return
  }

  正在加载.value = true
  错误信息.value = ''
  try {
    const 令牌 = 获取令牌()
    const [好友, 群组, 邀请] = await Promise.all([
      发送请求<好友[]>('/my/friends', {}, 令牌),
      发送请求<群组[]>('/my/groups', {}, 令牌),
      发送请求<群邀请[]>('/my/group-invitations', {}, 令牌),
    ])
    好友列表.value = 好友
    群组列表.value = 群组
    邀请列表.value = 邀请
  } catch (原因) {
    错误信息.value = 原因 instanceof Error ? 原因.message : '加载联系人失败'
  } finally {
    正在加载.value = false
  }
}

function 打开好友私聊(好友项: 好友) {
  void router.push({
    name: 'friend-chat',
    params: { friendId: 好友项.用户编号 },
    query: { nickname: 好友项.昵称 },
  })
}

function 打开群聊(群组: 群组) {
  void router.push({ name: 'group-chat', params: { groupId: 群组.群组编号 } })
}

async function 删除好友(好友项: 好友) {
  if (!window.confirm(`确定删除好友 ${好友项.昵称} 吗？历史聊天记录会保留。`)) return
  正在处理编号.value = 好友项.用户编号
  错误信息.value = ''
  try {
    await 发送请求(`/my/friends/${encodeURIComponent(好友项.用户编号)}`, {
      method: 'DELETE',
    }, 获取令牌())
    await 加载联系人()
  } catch (原因) {
    错误信息.value = 原因 instanceof Error ? 原因.message : '删除好友失败'
  } finally {
    正在处理编号.value = ''
  }
}

async function 处理群邀请(邀请: 群邀请, 操作: 'accept' | 'reject') {
  正在处理编号.value = 邀请.邀请编号
  错误信息.value = ''
  提示信息.value = ''
  try {
    await 发送请求(`/group-invitations/${encodeURIComponent(邀请.邀请编号)}/${操作}`, {
      method: 'POST',
    }, 获取令牌())
    提示信息.value = 操作 === 'accept' ? `已加入「${邀请.群组名称}」` : '已拒绝群邀请'
    await 加载联系人()
    if (操作 === 'accept') {
      void router.push({ name: 'group-chat', params: { groupId: 邀请.群组编号 } })
    }
  } catch (原因) {
    错误信息.value = 原因 instanceof Error ? 原因.message : '处理群邀请失败'
  } finally {
    正在处理编号.value = ''
  }
}

function 处理实时事件(事件: { 事件类型: string; 数据?: Record<string, unknown> }) {
  if (事件.事件类型 === 'group.message_created' && 事件.数据) {
    const 数据 = 事件.数据
    const 群组编号 = 数据.群组编号
    if (typeof 群组编号 === 'string') {
      const 群组 = 群组列表.value.find((项目) => 项目.群组编号 === 群组编号)
      if (群组) {
        if (typeof 数据.消息正文 === 'string') 群组.最近消息 = 数据.消息正文
        if (typeof 数据.创建时间 === 'string') 群组.最近消息时间 = 数据.创建时间
        if (typeof 数据.群内序号 === 'number') {
          群组.最后消息序号 = Math.max(群组.最后消息序号, 数据.群内序号)
        }
        群组列表.value = [...群组列表.value].sort((左侧, 右侧) =>
          (右侧.最近消息时间 ?? '').localeCompare(左侧.最近消息时间 ?? ''),
        )
      }
    }
  }
  if (事件.事件类型.startsWith('group.invitation_')) void 加载联系人()
}

const 取消实时订阅 = 订阅实时事件(处理实时事件)
onMounted(() => void 加载联系人())
watch(() => props.当前用户?.用户编号, () => void 加载联系人())
onBeforeUnmount(取消实时订阅)

function 格式化时间(时间: string) {
  if (!时间) return ''
  const 日期 = new Date(时间)
  return Number.isNaN(日期.getTime())
    ? 时间
    : 日期.toLocaleString('zh-CN', { dateStyle: 'medium', timeStyle: 'short' })
}
</script>

<template>
  <section class="im-inbox contacts-inbox">
    <header class="im-inbox-header">
      <div>
        <div class="section-eyebrow">好友与群组</div>
        <h1>联系人</h1>
        <p>好友和已加入的群聊集中在这里。</p>
      </div>
      <button
        v-if="当前用户"
        class="button button-quiet im-refresh"
        :disabled="正在加载"
        aria-label="刷新联系人"
        @click="加载联系人"
      >{{ 正在加载 ? '同步中…' : '刷新 ↻' }}</button>
    </header>

    <p v-if="错误信息" class="contacts-feedback contacts-error" role="alert">{{ 错误信息 }}</p>
    <p v-if="提示信息" class="contacts-feedback contacts-success" role="status">{{ 提示信息 }}</p>

    <template v-if="当前用户">
      <div class="im-toolbar contacts-toolbar">
        <label class="im-search">
          <span aria-hidden="true">⌕</span>
          <input v-model="搜索词" type="search" placeholder="搜索好友或群聊" aria-label="搜索好友或群聊">
        </label>
        <div class="im-filters" role="tablist" aria-label="联系人分类">
          <button
            v-for="分类 in 分类列表"
            :key="分类.名称"
            type="button"
            role="tab"
            :aria-selected="当前分类 === 分类.名称"
            :class="{ active: 当前分类 === 分类.名称 }"
            @click="当前分类 = 分类.名称"
          >{{ 分类.名称 }}<span>{{ 分类.数量 }}</span></button>
        </div>
      </div>

      <div v-if="筛选后项目.length" class="im-conversation-list">
        <template v-for="项目 in 筛选后项目" :key="项目.编号">
          <article v-if="项目.类型 === '好友'" class="im-conversation contact-item">
            <button class="contact-open" type="button" @click="打开好友私聊(项目.好友)">
              <span class="im-avatar">{{ 项目.好友.昵称.slice(0, 1) }}</span>
              <span class="im-conversation-copy">
                <span class="im-conversation-topline">
                  <strong>{{ 项目.好友.昵称 }}</strong>
                  <time>{{ 格式化时间(项目.好友.建立时间) }}</time>
                </span>
                <span class="im-conversation-preview">
                  <span>@{{ 项目.好友.登录名 }}</span>
                  <span class="im-conversation-tags"><small class="im-tag">好友</small></span>
                </span>
              </span>
            </button>
            <button
              class="contact-remove"
              :disabled="正在处理编号 === 项目.好友.用户编号"
              :aria-label="`删除好友 ${项目.好友.昵称}`"
              @click="删除好友(项目.好友)"
            >删除</button>
          </article>

          <button
            v-else-if="项目.类型 === '群聊'"
            class="im-conversation"
            type="button"
            @click="打开群聊(项目.群组)"
          >
            <span class="im-avatar im-avatar-群聊">群</span>
            <span class="im-conversation-copy">
              <span class="im-conversation-topline">
                <strong>{{ 项目.群组.群组名称 }}</strong>
                <time>{{ 格式化时间(项目.群组.最近消息时间 ?? '') }}</time>
              </span>
              <span class="im-conversation-preview">
                <span>{{ 项目.群组.最近消息 || 项目.群组.群组公告 || '还没有消息' }}</span>
                <span class="im-conversation-tags">
                  <small class="im-tag">{{ 项目.群组.成员数 }} 人</small>
                </span>
              </span>
            </span>
          </button>

          <article v-else class="im-conversation contact-item">
            <span class="im-avatar im-avatar-群聊">群</span>
            <span class="im-conversation-copy">
              <span class="im-conversation-topline">
                <strong>{{ 项目.邀请.群组名称 }} 邀请</strong>
                <time>{{ 格式化时间(项目.邀请.创建时间) }}</time>
              </span>
              <span class="im-conversation-preview">
                <span>{{ 项目.邀请.邀请人昵称 }} 邀请你加入群聊</span>
                <span class="im-conversation-tags"><small class="im-tag">待处理</small></span>
              </span>
            </span>
            <span class="contact-invitation-actions">
              <button
                class="button button-primary button-small"
                :disabled="正在处理编号 === 项目.邀请.邀请编号"
                @click="处理群邀请(项目.邀请, 'accept')"
              >加入</button>
              <button
                class="button button-quiet button-small"
                :disabled="正在处理编号 === 项目.邀请.邀请编号"
                @click="处理群邀请(项目.邀请, 'reject')"
              >拒绝</button>
            </span>
          </article>
        </template>
      </div>
      <div v-else class="im-empty">
        <span class="im-empty-icon">♧</span>
        <strong>{{ 正在加载 ? '正在同步联系人' : 搜索词 ? '没有找到匹配的联系人' : '这里还没有联系人' }}</strong>
        <p>{{ 搜索词 ? '试试其他好友昵称、登录名或群聊名称。' : '点击顶部 + 创建群聊或添加好友。' }}</p>
      </div>
    </template>

    <div v-else class="im-empty">
      <span class="im-empty-icon">◉</span>
      <strong>登录后查看联系人</strong>
      <p>登录后即可查看好友和已加入的群聊。</p>
      <button class="button button-primary" @click="router.push({ name: 'profile' })">前往登录</button>
    </div>
  </section>
</template>

<style scoped>
.contacts-toolbar{margin-top:4px}
.contacts-feedback{margin-bottom:12px;padding:10px 12px;border-radius:9px;font-size:12px}
.contacts-error{background:#fff0ec;color:#a84229}
.contacts-success{background:#e7f4ef;color:#126b56}
.contact-item{cursor:default}
.contact-item:hover{background:#fff}
.contact-open{display:flex;flex:1;align-items:center;gap:13px;min-width:0;padding:0;border:0;background:transparent;text-align:left}
.contact-remove{padding:7px 9px;border:0;border-radius:7px;background:transparent;color:#9b6d66;font-size:11px}
.contact-remove:hover:not(:disabled){background:#fff0ee}
.contact-invitation-actions{display:flex;flex:none;gap:6px}
@media(max-width:680px){
  .contact-item{flex-wrap:wrap}
  .contact-open{min-width:calc(100% - 62px)}
  .contact-invitation-actions{width:100%;justify-content:flex-end}
}
</style>
