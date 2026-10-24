<script setup lang="ts">
import type { 用户 } from '../领域/类型'

type 认证表单 = { login_name: string; password: string; nickname: string }

defineProps<{
  当前用户: 用户 | null
  正在处理: boolean
  是否注册: boolean
  认证表单数据: 认证表单
}>()

const emit = defineEmits<{
  登录: []
  退出登录: []
  '更新:是否注册': [是否注册: boolean]
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
