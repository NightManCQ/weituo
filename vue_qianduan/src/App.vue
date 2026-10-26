<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { RouterLink, RouterView, useRoute, useRouter } from 'vue-router'
import { 使用任务工作台 } from './组合式/使用任务工作台'
import type { 消息会话, 任务, 任务位置 } from './领域/类型'

type 页面路由名 = 'tasks' | 'square' | 'publish' | 'messages' | 'contacts' | 'groups' | 'chat' | 'profile'

const route = useRoute()
const router = useRouter()
const {
  当前用户,
  任务列表,
  我的任务列表,
  消息会话列表,
  当前委托详情,
  错误信息,
  提示信息,
  正在处理,
  正在加载聊天会话,
  正在加载委托详情,
  委托详情错误,
  是否注册,
  认证表单数据,
  任务表单数据,
  发布反馈状态,
  我的积分账户,
  我的积分流水,
  状态筛选,
  提交认证,
  退出登录,
  加载任务,
  加载我的任务,
  加载积分账户,
  加载委托详情,
  清除委托详情,
  加载聊天会话,
  发布任务,
  执行任务操作,
  获取任务操作,
  格式化日期,
} = 使用任务工作台()

const 导航项目: { 名称: string; 路由名: 页面路由名; 图标: string }[] = [
  { 名称: '委托', 路由名: 'tasks', 图标: '▤' },
  { 名称: '委托广场', 路由名: 'square', 图标: '▦' },
  { 名称: '发布委托', 路由名: 'publish', 图标: '＋' },
  { 名称: '消息', 路由名: 'messages', 图标: '◌' },
  { 名称: '联系人', 路由名: 'contacts', 图标: '♧' },
  { 名称: '我', 路由名: 'profile', 图标: '○' },
]

const 页面属性 = computed<Record<string, unknown>>(() => {
  switch (route.name) {
    case 'tasks':
      return {
        当前用户: 当前用户.value,
        任务列表: 我的任务列表.value,
        正在加载: 正在处理.value,
        格式化日期,
        获取任务操作,
        on刷新: 加载我的任务,
        on执行操作: 执行任务操作,
        on查看消息: 打开任务消息,
        on前往广场: () => router.push({ name: 'square' }),
        on前往个人中心: () => router.push({ name: 'profile' }),
      }
    case 'square':
      return {
        当前用户: 当前用户.value,
        任务列表: 任务列表.value,
        状态筛选: 状态筛选.value,
        格式化日期,
        获取任务操作,
        'on更新:状态筛选': async (状态: string) => {
          状态筛选.value = 状态
          await 加载任务()
        },
        on刷新: 加载任务,
        on发布需求: () => router.push({ name: 'publish' }),
        on执行操作: 执行任务操作,
        on查看消息: 打开任务消息,
        on登录: 打开登录,
      }
    case 'publish':
      return {
        当前用户: 当前用户.value,
        任务表单数据: 任务表单数据.value,
        积分账户: 我的积分账户.value,
        发布反馈: 发布反馈状态.value,
        正在处理: 正在处理.value,
        on发布: 发布任务,
        on选择地址: () => router.push({ name: 'location' }),
        on清除地址: () => {
          任务表单数据.value.位置 = null
        },
        on前往个人中心: () => router.push({ name: 'profile' }),
      }
    case 'messages':
      return {
        当前用户: 当前用户.value,
        消息会话列表: 消息会话列表.value,
        正在加载: 正在加载聊天会话.value,
        格式化日期,
        on刷新: 加载聊天会话,
        on打开会话: 打开消息会话,
        on前往个人中心: () => router.push({ name: 'profile' }),
      }
    case 'contacts':
      return { 当前用户: 当前用户.value }
    case 'group-create':
    case 'friend-search':
      return { 当前用户: 当前用户.value }
    case 'group-chat':
      return {
        当前用户: 当前用户.value,
        群组编号: typeof route.params.groupId === 'string' ? route.params.groupId : '',
      }
    case 'friend-chat':
      return {
        当前用户: 当前用户.value,
        好友私聊: {
          用户编号: typeof route.params.friendId === 'string' ? route.params.friendId : '',
          联系人昵称: typeof route.query.nickname === 'string' ? route.query.nickname : '好友',
        },
        on返回: () => router.push({ name: 'contacts' }),
      }
    case 'chat': {
      return {
        当前用户: 当前用户.value,
        任务编号: typeof route.params.taskId === 'string' ? route.params.taskId : '',
      }
    }
    case 'commission-detail':
      return {
        委托编号: typeof route.params.id === 'string' ? route.params.id : '',
        委托详情: 当前委托详情.value,
        正在加载: 正在加载委托详情.value,
        错误信息: 委托详情错误.value,
        格式化日期,
        on返回列表: () => router.push({ name: 'square' }),
        on重新加载: 加载委托详情,
      }
    case 'location':
      return {
        初始位置: 任务表单数据.value.位置,
        on选择位置: 完成位置选择,
        on取消: 返回发布页,
      }
    case 'profile':
      return {
        当前用户: 当前用户.value,
        积分账户: 我的积分账户.value,
        积分流水: 我的积分流水.value,
        正在处理: 正在处理.value,
        是否注册: 是否注册.value,
        认证表单数据: 认证表单数据.value,
        on登录: 提交认证,
        on退出登录: 退出登录,
        on刷新积分: 加载积分账户,
        'on更新:是否注册': (值: boolean) => {
          是否注册.value = 值
        },
      }
    default:
      return {}
  }
})

watch(() => route.name, (路由名) => {
  if (路由名 === 'tasks' && 当前用户.value) void 加载我的任务()
  if (路由名 === 'messages' && 当前用户.value) void 加载聊天会话()
  if (路由名 === 'publish' && 当前用户.value) void 加载积分账户()
})

watch(
  () => route.name === 'commission-detail' ? route.params.id : undefined,
  (委托编号) => {
    if (typeof 委托编号 === 'string') {
      void 加载委托详情(委托编号)
    } else {
      清除委托详情()
    }
  },
  { immediate: true },
)

async function 打开任务消息(任务项: 任务) {
  await router.push({ name: 'chat', params: { taskId: 任务项.任务编号 } })
}

async function 打开消息会话(会话: 消息会话) {
  if (会话.会话类型 === '委托') {
    await router.push({ name: 'group-chat', params: { groupId: 会话.会话编号 } })
    return
  }
  if (会话.会话类型 === '群聊') {
    await router.push({ name: 'group-chat', params: { groupId: 会话.会话编号 } })
    return
  }
  await router.push({
    name: 'friend-chat',
    params: { friendId: 会话.关联编号 },
    query: { nickname: 会话.会话名称 },
  })
}

function 打开登录() {
  void router.push({ name: 'profile' })
  错误信息.value = '请先登录后再使用此功能'
}

function 完成位置选择(位置: 任务位置) {
  任务表单数据.value.位置 = 位置
  void router.replace({ name: 'publish' })
}

function 返回发布页() {
  void router.replace({ name: 'publish' })
}

function 打开联系人快捷菜单(路由名: 'group-create' | 'friend-search') {
  联系人快捷菜单已打开.value = false
  void router.push({ name: 路由名 })
}

const 联系人快捷菜单已打开 = ref(false)

watch(() => route.name, () => {
  联系人快捷菜单已打开.value = false
})
</script>

<template>
  <main
    class="app-shell"
    :class="{
      'chat-app-shell': route.name === 'chat' || route.name === 'friend-chat' || route.name === 'group-chat',
      'map-picker-shell': route.name === 'location',
    }"
  >
    <header v-if="route.name !== 'chat' && route.name !== 'friend-chat' && route.name !== 'group-chat'" class="topbar">
      <div v-if="route.name === 'contacts'" class="contact-create-menu">
        <button
          class="contact-create-trigger"
          type="button"
          aria-label="创建或添加"
          :aria-expanded="联系人快捷菜单已打开"
          @click="联系人快捷菜单已打开 = !联系人快捷菜单已打开"
        >+</button>
        <div v-if="联系人快捷菜单已打开" class="contact-create-popover" role="menu">
          <button type="button" role="menuitem" @click="打开联系人快捷菜单('group-create')">
            <span>＋</span>创建群聊
          </button>
          <button type="button" role="menuitem" @click="打开联系人快捷菜单('friend-search')">
            <span>⌕</span>添加好友
          </button>
        </div>
      </div>
      <RouterLink v-else class="brand" :to="{ name: 'square' }">
        <span class="brand-mark">委</span><span>委托协作</span>
      </RouterLink>
      <div v-if="当前用户" class="user-menu">
        <span class="online-dot"></span><span>{{ 当前用户.昵称 }}</span>
      </div>
      <div v-else class="top-note">发布委托 · 找到帮手 · 实时协作</div>
    </header>

    <div class="page-content">
      <div v-if="错误信息" class="notice error-notice" role="alert">
        {{ 错误信息 }}<button @click="错误信息 = ''">关闭</button>
      </div>
      <div v-if="提示信息" class="notice info-notice" role="status">
        {{ 提示信息 }}<button @click="提示信息 = ''">关闭</button>
      </div>

      <RouterView v-slot="{ Component }">
        <component :is="Component" v-bind="页面属性" />
      </RouterView>
    </div>

    <nav
      v-if="route.name !== 'chat' && route.name !== 'friend-chat' && route.name !== 'group-chat' &&
        route.name !== 'location' && route.name !== 'commission-detail'"
      class="bottom-nav"
      aria-label="主导航"
    >
      <RouterLink
        v-for="项目 in 导航项目"
        :key="项目.路由名"
        :to="{ name: 项目.路由名 }"
        custom
        v-slot="{ navigate }"
      >
        <button
          :aria-current="route.name === 项目.路由名 ||
            (route.name === 'chat' && 项目.路由名 === 'messages') ||
            ((route.name === 'friend-search' || route.name === 'group-create') &&
              项目.路由名 === 'contacts') ? 'page' : undefined"
          :class="{
            active: route.name === 项目.路由名 ||
              (route.name === 'chat' && 项目.路由名 === 'messages') ||
              ((route.name === 'friend-search' || route.name === 'group-create') &&
                项目.路由名 === 'contacts'),
          }"
          @click="navigate"
        >
          <span class="nav-icon" aria-hidden="true">{{ 项目.图标 }}</span>
          <span class="nav-label">{{ 项目.名称 }}</span>
          <i v-if="项目.路由名 === 'messages' && 消息会话列表.length" class="nav-indicator"></i>
        </button>
      </RouterLink>
    </nav>
  </main>
</template>
