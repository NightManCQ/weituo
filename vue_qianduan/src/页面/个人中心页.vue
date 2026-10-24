<script setup lang="ts">
import type { 积分账户, 积分流水, 用户 } from '../领域/类型'

type 认证表单 = { login_name: string; password: string; nickname: string }

defineProps<{
  当前用户: 用户 | null
  积分账户: 积分账户 | null
  积分流水: 积分流水[]
  正在处理: boolean
  是否注册: boolean
  认证表单数据: 认证表单
}>()

const emit = defineEmits<{
  登录: []
  退出登录: []
  '更新:是否注册': [是否注册: boolean]
  刷新积分: []
}>()
</script>

<template>
  <section class="page-panel profile-page">
    <div class="page-heading"><div class="section-eyebrow">个人中心</div><h1>我</h1><p class="muted">管理个人资料和账号。</p></div>
    <section v-if="当前用户" class="profile-card">
      <div class="profile-summary">
        <div class="profile-avatar">{{ 当前用户.昵称.slice(0, 1) }}</div>
        <div><div class="section-eyebrow">已登录</div><h2>{{ 当前用户.昵称 }}</h2><p class="muted">账号：{{ 当前用户.登录名 }}</p></div>
      </div>
      <dl class="profile-details">
        <div><dt>昵称</dt><dd>{{ 当前用户.昵称 }}</dd></div>
        <div><dt>登录名</dt><dd>{{ 当前用户.登录名 }}</dd></div>
        <div><dt>用户编号</dt><dd>{{ 当前用户.用户编号 }}</dd></div>
      </dl>
      <section class="points-card">
        <div class="points-card-heading">
          <div><div class="section-eyebrow">内部积分</div><h3>我的积分</h3></div>
          <button class="button button-quiet" type="button" @click="emit('刷新积分')">刷新</button>
        </div>
        <div class="points-balance">
          <div><span>可用积分</span><strong>{{ 积分账户?.可用积分 ?? '—' }}</strong></div>
          <div><span>托管中</span><strong>{{ 积分账户?.冻结积分 ?? '—' }}</strong></div>
          <div><span>总积分</span><strong>{{ 积分账户?.总积分 ?? '—' }}</strong></div>
        </div>
        <p class="muted points-note">积分不可提现或用户间自由转让。发布委托时会冻结报酬；委托方验收后发放。</p>
        <div v-if="积分流水.length" class="points-ledger">
          <h4>最近积分流水</h4>
          <div v-for="流水 in 积分流水" :key="流水.流水编号" class="points-ledger-row">
            <div>
              <strong>{{ 流水.说明 || 流水.交易类型 }}</strong>
              <small>{{ 流水.交易类型 }} · {{ new Date(流水.创建时间).toLocaleString('zh-CN') }}</small>
              <small>
                可用 {{ 流水.可用变动 > 0 ? '+' : '' }}{{ 流水.可用变动 }}
                · 冻结 {{ 流水.冻结变动 > 0 ? '+' : '' }}{{ 流水.冻结变动 }}
              </small>
            </div>
            <span :class="{ 'points-positive': 流水.可用变动 > 0, 'points-negative': 流水.可用变动 < 0 }">
              {{ 流水.可用变动 > 0 ? '+' : '' }}{{ 流水.可用变动 }}
            </span>
          </div>
        </div>
        <p v-else class="muted points-note">暂无积分流水。如需发放积分，请联系运营员或管理员。</p>
      </section>
      <button class="button button-outline" @click="emit('退出登录')">退出登录</button>
    </section>
    <form v-else class="auth-card profile-auth-card" @submit.prevent="emit('登录')">
      <div class="section-eyebrow">欢迎加入</div>
      <h2>{{ 是否注册 ? '创建账号' : '登录账号' }}</h2>
      <p class="muted">登录后即可发布委托、接单和参与委托沟通。</p>
      <label v-if="是否注册">昵称<input v-model.trim="认证表单数据.nickname" maxlength="40" autocomplete="nickname" required placeholder="怎么称呼你" /></label>
      <label>登录名<input v-model.trim="认证表单数据.login_name" minlength="3" maxlength="64" autocomplete="username" required placeholder="请输入登录名" /></label>
      <label>密码<input v-model="认证表单数据.password" type="password" minlength="10" maxlength="128" :autocomplete="是否注册 ? 'new-password' : 'current-password'" required placeholder="至少 10 位" /></label>
      <button class="button button-primary full-width" :disabled="正在处理">{{ 正在处理 ? '请稍候…' : (是否注册 ? '注册并登录' : '登录') }}</button>
      <button type="button" class="text-button" @click="emit('更新:是否注册', !是否注册)">
        {{ 是否注册 ? '已有账号？返回登录' : '还没有账号？立即注册' }}
      </button>
    </form>
  </section>
</template>
