<script setup lang="ts">
import type { 任务位置 } from '../领域/类型'

type 任务表单 = {
  title: string
  description: string
  deadline: string
  位置: 任务位置 | null
}

defineProps<{
  当前用户: { 昵称: string } | null
  任务表单数据: 任务表单
  正在处理: boolean
}>()

const emit = defineEmits<{
  发布: []
  前往个人中心: []
  选择地址: []
  清除地址: []
}>()

function 地址摘要(位置: 任务位置) {
  return [位置.国, 位置.省, 位置.市, 位置.县, 位置.镇, 位置.村, 位置.地址]
    .filter(Boolean)
    .join(' · ')
}
</script>

<template>
  <section class="page-panel">
    <template v-if="当前用户">
      <div class="page-heading">
        <div class="section-eyebrow">发布委托</div>
        <h1>把委托说明白</h1>
        <p class="muted">填写标题和说明，方便合适的伙伴了解并接单。</p>
      </div>
      <form class="publish-card publish-page-card" @submit.prevent="emit('发布')">
        <div class="card-heading">
          <div><span class="heading-icon">＋</span><div><h2>发布一个委托</h2><p>提交后会在委托广场公开展示</p></div></div>
          <span class="private-label">公开委托</span>
        </div>
        <div class="form-grid">
          <label class="wide-field">委托标题<input v-model.trim="任务表单数据.title" required maxlength="120" placeholder="例如：帮忙整理一份活动方案" /></label>
          <label class="wide-field">委托说明<textarea v-model.trim="任务表单数据.description" rows="5" maxlength="10000" placeholder="补充背景、交付内容或其他要求…"></textarea></label>
          <label>截止时间（可选）<input v-model="任务表单数据.deadline" type="datetime-local" /></label>
          <div class="wide-field publish-location">
            <span>委托地址（可选）</span>
            <div v-if="任务表单数据.位置" class="publish-location-selected">
              <span class="publish-location-icon" aria-hidden="true">⌖</span>
              <div>
                <strong>{{ 任务表单数据.位置.地址 || 地址摘要(任务表单数据.位置) || '已选择地图位置' }}</strong>
                <small>
                  {{ [任务表单数据.位置.省, 任务表单数据.位置.市, 任务表单数据.位置.县,
                    任务表单数据.位置.镇, 任务表单数据.位置.村].filter(Boolean).join(' · ') }}
                  {{ 任务表单数据.位置.经度.toFixed(6) }}, {{ 任务表单数据.位置.纬度.toFixed(6) }}
                </small>
              </div>
            </div>
            <div class="publish-location-actions">
              <button class="button button-outline" type="button" @click="emit('选择地址')">
                {{ 任务表单数据.位置 ? '更改地址' : '⌖ 选择地址' }}
              </button>
              <button
                v-if="任务表单数据.位置"
                class="text-button publish-location-clear"
                type="button"
                @click="emit('清除地址')"
              >清除地址</button>
            </div>
          </div>
        </div>
        <div class="form-footer">
          <span>发布人：{{ 当前用户.昵称 }}</span>
          <button class="button button-primary" :disabled="正在处理 || !任务表单数据.title.trim()">
            {{ 正在处理 ? '正在发布…' : '发布委托 ↗' }}
          </button>
        </div>
      </form>
    </template>
    <div v-else class="empty-state access-prompt">
      <div class="empty-icon">◉</div><h3>登录后发布委托</h3><p>登录或注册账号，即可发布你的委托。</p>
      <button class="button button-primary" @click="emit('前往个人中心')">前往登录</button>
    </div>
  </section>
</template>
