<script setup lang="ts">
import { computed, ref } from 'vue'
import type { 消息会话, 用户 } from '../领域/类型'

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

const 当前分类 = ref<'全部' | 消息会话['会话类型']>('全部')
const 搜索词 = ref('')

const 分类列表 = computed(() => [
  { 名称: '全部' as const, 数量: 属性.消息会话列表.length },
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

const 筛选后会话 = computed(() => {
  const 查询词 = 搜索词.value.trim().toLocaleLowerCase()
  return 属性.消息会话列表.filter((会话) => {
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
        :disabled="正在加载"
        aria-label="刷新会话"
        @click="emit('刷新')"
      >
        {{ 正在加载 ? '同步中…' : '刷新 ↻' }}
      </button>
    </header>

    <template v-if="当前用户">
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

      <div v-if="筛选后会话.length" class="im-conversation-list">
        <button
          v-for="会话 in 筛选后会话"
          :key="`${会话.会话类型}:${会话.会话编号}`"
          class="im-conversation"
          type="button"
          @click="emit('打开会话', 会话)"
        >
          <span class="im-avatar" :class="`im-avatar-${会话.会话类型}`">
            {{ 会话.会话类型 === '群聊' ? '群' : 会话.会话名称.slice(0, 1) }}
          </span>
          <span class="im-conversation-copy">
            <span class="im-conversation-topline">
              <strong>{{ 会话.会话名称 }}</strong>
              <time>{{ 格式化日期(会话.最近消息时间) }}</time>
            </span>
            <span class="im-conversation-preview">
              <span>{{ 会话.最近消息 || (会话.会话类型 === '好友' ? '开始聊天吧' : '暂无消息') }}</span>
              <span class="im-conversation-tags">
                <small v-if="会话.会话类型 === '委托' && 会话.会话状态" class="im-tag">
                  {{ 会话.会话状态 }}
                </small>
                <small v-else-if="会话.会话类型 === '群聊'" class="im-tag">
                  {{ 会话.成员数 ?? 0 }} 人
                </small>
                <small v-else-if="会话.会话类型 === '好友'" class="im-tag">好友</small>
              </span>
            </span>
          </span>
        </button>
      </div>
      <div v-else class="im-empty">
        <span class="im-empty-icon">◌</span>
        <strong>{{ 正在加载 ? '正在同步会话' : 搜索词 ? '没有找到匹配的会话' : '还没有消息会话' }}</strong>
        <p>
          {{ 搜索词 ? '试试其他名称或消息关键词。' : '开始好友私聊、群聊或委托沟通后，会话会集中显示在这里。' }}
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
