<script setup lang="ts">
import { computed, watch } from 'vue'
import { RouterLink, RouterView, useRoute, useRouter } from 'vue-router'
import { 使用任务工作台 } from './组合式/使用任务工作台'
import type { 聊天会话, 任务, 任务位置 } from './领域/类型'

type 页面路由名 = 'tasks' | 'square' | 'publish' | 'messages' | 'chat' | 'profile'

const route = useRoute()
const router = useRouter()
const {
  当前用户,
  任务列表,
  我的任务列表,
  聊天会话列表,
  当前任务,
  当前委托详情,
  消息列表,
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
  消息表单数据,
  状态筛选,
  可以聊天,
  可以发送消息,
  正在发送聊天消息,
  消息待确认,
  待确认消息,
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
  选择任务,
  选择聊天会话,
  加载聊天任务,
  发送聊天消息,
  获取任务操作,
  格式化日期,
} = 使用任务工作台()

const 导航项目: { 名称: string; 路由名: 页面路由名; 图标: string }[] = [
  { 名称: '委托', 路由名: 'tasks', 图标: '▤' },
  { 名称: '委托广场', 路由名: 'square', 图标: '▦' },
  { 名称: '发布委托', 路由名: 'publish', 图标: '＋' },
  { 名称: '消息', 路由名: 'messages', 图标: '◌' },
  { 名称: '我', 路由名: 'profile', 图标: '○' },
]

const 当前联系人昵称 = computed(() => {
  const 任务编号 = route.params.taskId
  if (typeof 任务编号 !== 'string') return '委托参与者'
  return 聊天会话列表.value.find((会话) => 会话.任务编号 === 任务编号)?.联系人昵称
    ?? '委托参与者'
})

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
        聊天会话列表: 聊天会话列表.value,
        正在加载: 正在加载聊天会话.value,
        格式化日期,
        on刷新: 加载聊天会话,
        on打开会话: 打开聊天会话,
        on前往个人中心: () => router.push({ name: 'profile' }),
      }
    case 'chat': {
      const 任务编号 = route.params.taskId
      const 当前路由任务 = typeof 任务编号 === 'string' &&
        当前任务.value?.任务编号 === 任务编号
      return {
        当前用户: 当前用户.value,
        联系人昵称: 当前联系人昵称.value,
        当前任务: 当前路由任务 ? 当前任务.value : null,
        消息列表: 当前路由任务 ? 消息列表.value : [],
        消息表单数据: 消息表单数据.value,
        可以聊天: 当前路由任务 && 可以聊天.value,
        正在处理: 正在处理.value,
        正在发送聊天消息: 正在发送聊天消息.value,
        消息待确认: 消息待确认.value,
        有待确认消息: 待确认消息.value !== null,
        可以发送消息: 当前路由任务 && 可以发送消息.value,
        on返回: () => router.push({ name: 'messages' }),
        'on更新:消息表单数据': (内容: string) => {
          消息表单数据.value = 内容
        },
        on发送消息: 发送聊天消息,
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

watch(
  () => route.name === 'chat' ? route.params.taskId : undefined,
  async (任务编号) => {
    if (typeof 任务编号 !== 'string' || !当前用户.value) return
    if (当前任务.value?.任务编号 === 任务编号) return

    const 任务项 = await 加载聊天任务(任务编号)
    if (
      任务项 &&
      当前用户.value &&
      route.name === 'chat' &&
      route.params.taskId === 任务编号
    ) {
      await 选择任务(任务项)
    }
  },
  { immediate: true },
)

async function 打开任务消息(任务项: 任务) {
  await 选择任务(任务项)
  await router.push({ name: 'chat', params: { taskId: 任务项.任务编号 } })
}

async function 打开聊天会话(会话: 聊天会话) {
  await 选择聊天会话(会话)
  if (当前任务.value?.任务编号 === 会话.任务编号) {
    await router.push({ name: 'chat', params: { taskId: 会话.任务编号 } })
  }
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
</script>

<template>
  <main
    class="app-shell"
    :class="{
      'chat-app-shell': route.name === 'chat',
      'map-picker-shell': route.name === 'location',
    }"
  >
    <header v-if="route.name !== 'chat'" class="topbar">
      <RouterLink class="brand" :to="{ name: 'square' }">
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
      v-if="route.name !== 'chat' && route.name !== 'location' && route.name !== 'commission-detail'"
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
            (route.name === 'chat' && 项目.路由名 === 'messages') ? 'page' : undefined"
          :class="{
            active: route.name === 项目.路由名 ||
              (route.name === 'chat' && 项目.路由名 === 'messages'),
          }"
          @click="navigate"
        >
          <span class="nav-icon" aria-hidden="true">{{ 项目.图标 }}</span>
          <span class="nav-label">{{ 项目.名称 }}</span>
          <i v-if="项目.路由名 === 'messages' && 聊天会话列表.length" class="nav-indicator"></i>
        </button>
      </RouterLink>
    </nav>
  </main>
</template>
