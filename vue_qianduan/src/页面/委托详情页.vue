<script setup lang="ts">
import type { 任务 } from '../领域/类型'

defineProps<{
  委托编号: string
  委托详情: 任务 | null
  正在加载: boolean
  错误信息: string
  格式化日期: (日期: string) => string
}>()

const emit = defineEmits<{
  返回列表: []
  重新加载: [委托编号: string]
}>()

function 委托地址(委托: 任务) {
  if (!委托.位置) return '未填写委托地址'
  return [
    委托.位置.国,
    委托.位置.省,
    委托.位置.市,
    委托.位置.县,
    委托.位置.镇,
    委托.位置.村,
    委托.位置.地址,
  ].filter(Boolean).join(' · ') || '已选择地图位置'
}
</script>

<template>
  <section class="page-panel commission-detail-page">
    <div class="commission-detail-heading">
      <button class="button button-quiet" type="button" @click="emit('返回列表')">← 返回委托广场</button>
      <div class="section-eyebrow">委托详情</div>
    </div>

    <div v-if="正在加载" class="empty-state">
      <h3>正在加载委托详情…</h3>
    </div>
    <div v-else-if="错误信息" class="empty-state" role="alert">
      <h3>委托详情加载失败</h3>
      <p>{{ 错误信息 }}</p>
      <div class="commission-detail-actions">
        <button class="button button-outline" type="button" @click="emit('返回列表')">返回委托广场</button>
        <button class="button button-primary" type="button" @click="emit('重新加载', 委托编号)">重试</button>
      </div>
    </div>
    <div v-else-if="委托详情" class="commission-detail-card">
      <div class="commission-detail-title">
        <span class="status-badge" :class="`status-${委托详情.任务状态}`"><i></i>{{ 委托详情.任务状态 }}</span>
        <h1>{{ 委托详情.标题 }}</h1>
        <p>委托编号：{{ 委托详情.任务编号 }}</p>
      </div>

      <section class="commission-detail-section">
        <h2>委托说明</h2>
        <p class="commission-detail-description">{{ 委托详情.描述 || '委托方暂未提供补充说明。' }}</p>
      </section>

      <section class="commission-reward-panel">
        <span>委托报酬</span>
        <strong>{{ 委托详情.报酬积分 !== null ? `${委托详情.报酬积分} 积分` : '历史委托未设置积分报酬' }}</strong>
        <small v-if="委托详情.报酬积分 !== null">
          报酬已托管，接单方提交完成后由委托方验收并发放。
        </small>
      </section>

      <dl class="commission-detail-facts">
        <div><dt>委托方</dt><dd>{{ 委托详情.发布者编号 }}</dd></div>
        <div><dt>接单方</dt><dd>{{ 委托详情.接单者编号 || '暂未有人接单' }}</dd></div>
        <div><dt>发布时间</dt><dd>{{ 格式化日期(委托详情.创建时间) }}</dd></div>
        <div><dt>截止时间</dt><dd>{{ 委托详情.截止时间 ? 格式化日期(委托详情.截止时间) : '未设置' }}</dd></div>
        <div class="commission-detail-location"><dt>委托地址</dt><dd>{{ 委托地址(委托详情) }}</dd></div>
        <div v-if="委托详情.位置"><dt>经纬度</dt><dd>{{ 委托详情.位置.经度.toFixed(6) }}, {{ 委托详情.位置.纬度.toFixed(6) }}</dd></div>
      </dl>

      <div class="commission-detail-actions">
        <button class="button button-outline" type="button" @click="emit('返回列表')">返回委托广场</button>
      </div>
    </div>
  </section>
</template>
