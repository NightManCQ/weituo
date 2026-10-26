<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { RouterLink } from 'vue-router'
import type { 任务, 任务操作, 用户 } from '../领域/类型'

type 任务操作项 = { key: 任务操作; label: string }
type 委托分类 = 'all' | 'ongoing' | 'completed'
type 委托归属 = 'published' | 'claimed'

const 属性 = defineProps<{
  当前用户: 用户 | null
  任务列表: 任务[]
  总数: number
  正在加载: boolean
  还有更多: boolean
  加载错误: string
  委托归属: 委托归属
  格式化日期: (日期: string) => string
  获取任务操作: (任务项: 任务) => 任务操作项[]
}>()

const emit = defineEmits<{
  分类变更: [分类: 委托分类]
  归属变更: [归属: 委托归属]
  加载更多: []
  刷新: []
  执行操作: [任务项: 任务, 操作: 任务操作]
  查看消息: [任务项: 任务]
  前往广场: []
  前往个人中心: []
}>()

const 当前分类 = ref<委托分类>('all')
const 加载哨兵 = ref<HTMLElement | null>(null)
let 加载观察器: IntersectionObserver | null = null

const 分类列表: { key: 委托分类; label: string }[] = [
  { key: 'all', label: '全部' },
  { key: 'ongoing', label: '正在进行的委托' },
  { key: 'completed', label: '已完成' },
]

function 切换分类(分类: 委托分类) {
  if (当前分类.value === 分类) return
  当前分类.value = 分类
  emit('分类变更', 分类)
}

function 切换委托归属() {
  emit('归属变更', 属性.委托归属 === 'published' ? 'claimed' : 'published')
}

onMounted(() => {
  if (!('IntersectionObserver' in window)) return
  加载观察器 = new IntersectionObserver(([条目]) => {
    if (条目?.isIntersecting) emit('加载更多')
  }, { rootMargin: '240px 0px' })
})

watch(() => 属性.当前用户?.用户编号, (用户编号) => {
  if (用户编号) emit('分类变更', 当前分类.value)
}, { immediate: true })

watch(加载哨兵, (新哨兵, 旧哨兵) => {
  if (旧哨兵) 加载观察器?.unobserve(旧哨兵)
  if (新哨兵) 加载观察器?.observe(新哨兵)
}, { flush: 'post' })

onBeforeUnmount(() => 加载观察器?.disconnect())
</script>

<template>
  <section class="page-panel">
    <div class="welcome-row">
      <div>
        <div class="section-eyebrow">委托管理</div>
        <div class="task-heading-row">
          <h1>我的委托 <span class="count-pill">{{ 总数 }}</span></h1>
          <button
            type="button"
            class="button button-outline task-mode-switch"
            :aria-label="委托归属 === 'published' ? '切换到我接单的委托' : '切换到我发布的委托'"
            @click="切换委托归属"
          >{{ 委托归属 === 'published' ? '接单委托 →' : '我发布的委托 →' }}</button>
        </div>
        <p class="muted">
          {{ 委托归属 === 'published' ? '查看你发布的委托及处理进度。' : '查看你接单的委托及处理进度。' }}
        </p>
      </div>
      <button
        v-if="当前用户"
        class="button button-quiet"
        :disabled="正在加载"
        @click="emit('刷新')"
      >
        {{ 正在加载 ? '加载中…' : '刷新 ↻' }}
      </button>
    </div>

    <template v-if="当前用户">
      <div class="im-toolbar">
        <div class="im-filters" role="tablist" aria-label="委托分类">
          <button
            v-for="分类 in 分类列表"
            :key="分类.key"
            type="button"
            role="tab"
            :aria-selected="当前分类 === 分类.key"
            :class="{ active: 当前分类 === 分类.key }"
            @click="切换分类(分类.key)"
          >{{ 分类.label }}</button>
        </div>
      </div>

      <div v-if="任务列表.length" class="task-grid">
        <article v-for="任务项 in 任务列表" :key="任务项.任务编号" class="task-card">
          <RouterLink
            class="commission-card-content"
            :to="{ name: 'commission-detail', params: { id: 任务项.任务编号 } }"
          >
            <div class="task-card-top">
              <span class="status-badge" :class="`status-${任务项.任务状态}`">
                <i></i>{{ 任务项.任务状态 }}
              </span>
              <span class="task-date">{{ 格式化日期(任务项.创建时间) }}</span>
            </div>
            <h3>{{ 任务项.标题 }}</h3>
            <p class="task-description">{{ 任务项.描述 || '暂无补充描述' }}</p>
            <div class="task-meta">
              <span>{{ 委托归属 === 'published' ? '我发布' : '我接单' }}</span>
              <span v-if="任务项.截止时间">截止 {{ 格式化日期(任务项.截止时间) }}</span>
              <span class="task-reward">
                {{ 任务项.报酬积分 !== null ? `${任务项.报酬积分} 积分报酬` : '未设置积分报酬' }}
              </span>
            </div>
          </RouterLink>
          <div class="task-card-footer">
            <span class="task-version">#{{ 任务项.任务编号.slice(0, 8) }}</span>
            <div class="action-row">
              <button
                v-for="操作 in 获取任务操作(任务项)"
                :key="操作.key"
                class="button button-small"
                :class="操作.key === 'cancel' ? 'button-quiet' : 'button-primary'"
                @click="emit('执行操作', 任务项, 操作.key)"
              >{{ 操作.label }}</button>
              <button
                v-if="任务项.接单者编号"
                class="button button-small button-outline"
                @click="emit('查看消息', 任务项)"
              >查看消息</button>
            </div>
          </div>
        </article>
      </div>
      <div v-else-if="正在加载" class="empty-state">
        <div class="empty-icon">◌</div><h3>正在加载委托</h3><p>请稍候…</p>
      </div>
      <div v-else-if="加载错误" class="empty-state">
        <div class="empty-icon">!</div><h3>委托加载失败</h3>
        <p>{{ 加载错误 }}</p>
        <button class="button button-primary" @click="emit('刷新')">重新加载</button>
      </div>
      <div v-else class="empty-state">
        <div class="empty-icon">▤</div>
        <h3>{{ 当前分类 === 'all' ? (委托归属 === 'published' ? '还没有发布委托' : '还没有接单委托') : '此分类下暂无委托' }}</h3>
        <p>{{ 委托归属 === 'published' ? '前往委托广场发布或浏览委托。' : '前往委托广场发现并接取委托。' }}</p>
        <button class="button button-primary" @click="emit('前往广场')">前往委托广场</button>
      </div>

      <div v-if="任务列表.length && 还有更多" ref="加载哨兵" class="task-load-more" aria-live="polite">
        <template v-if="加载错误">
          <span>加载失败：{{ 加载错误 }}</span>
          <button class="text-button" @click="emit('加载更多')">点击重试</button>
        </template>
        <template v-else>
          {{ 正在加载 ? '正在加载更多…' : '继续下滑加载更多委托' }}
        </template>
      </div>
      <div v-else-if="任务列表.length && !还有更多 && !加载错误" class="task-load-more">
        已显示全部 {{ 总数 }} 条委托
      </div>
    </template>

    <div v-else class="empty-state access-prompt">
      <div class="empty-icon">◉</div><h3>登录后查看我的委托</h3><p>登录后即可查看你发布的委托。</p>
      <button class="button button-primary" @click="emit('前往个人中心')">前往登录</button>
    </div>
  </section>
</template>

<style scoped>
.task-load-more{padding:20px 0;text-align:center;color:#75817d;font-size:12px}
.task-heading-row{display:flex;align-items:center;gap:12px}
.task-heading-row h1{margin-right:0}
.task-mode-switch{flex:none}
@media(max-width:680px){.task-heading-row{gap:8px;flex-wrap:wrap}.task-mode-switch{padding:8px 10px;font-size:11px}}
</style>
