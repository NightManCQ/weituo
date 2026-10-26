<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { 发送请求 } from '../接口/请求'
import type { 用户 } from '../领域/类型'

defineProps<{ 当前用户: 用户 | null }>()
const router = useRouter()
const 群组名称 = ref('')
const 正在创建 = ref(false)
const 错误信息 = ref('')

async function 创建群聊() {
  const 名称 = 群组名称.value.trim()
  if (!名称 || 正在创建.value) return
  正在创建.value = true
  错误信息.value = ''
  try {
    const 结果 = await 发送请求<{ 群组编号: string }>('/groups', {
      method: 'POST',
      body: JSON.stringify({ name: 名称 }),
    }, localStorage.getItem('renwu_token') || '')
    await router.push({ name: 'group-chat', params: { groupId: 结果.群组编号 } })
  } catch (原因) {
    错误信息.value = 原因 instanceof Error ? 原因.message : '创建群聊失败'
  } finally {
    正在创建.value = false
  }
}
</script>

<template>
  <section class="page-panel create-group-page">
    <header class="page-heading">
      <div class="section-eyebrow">联系人</div>
      <h1>创建群聊</h1>
      <p class="muted">创建群组后，可在群聊页面邀请好友加入。</p>
    </header>

    <div v-if="!当前用户" class="empty-state access-prompt">
      <h3>登录后创建群聊</h3>
      <button class="button button-primary" @click="router.push({ name: 'profile' })">前往登录</button>
    </div>
    <form v-else class="create-group-form" @submit.prevent="创建群聊">
      <label for="new-group-name">群聊名称</label>
      <input
        id="new-group-name"
        v-model="群组名称"
        maxlength="80"
        autocomplete="off"
        placeholder="给群聊起个名字"
      />
      <p v-if="错误信息" class="create-group-error" role="alert">{{ 错误信息 }}</p>
      <div class="create-group-actions">
        <button class="button button-quiet" type="button" @click="router.push({ name: 'contacts' })">取消</button>
        <button class="button button-primary" :disabled="正在创建 || !群组名称.trim()">
          {{ 正在创建 ? '创建中…' : '创建群聊' }}
        </button>
      </div>
    </form>
  </section>
</template>

<style scoped>
.create-group-page{max-width:760px;margin:0 auto}
.create-group-page>.page-heading{margin-bottom:24px}
.create-group-form{display:grid;gap:10px;padding:22px;border:1px solid var(--line);border-radius:14px;background:#fff}
.create-group-form label{color:#566460;font-size:12px;font-weight:600}
.create-group-actions{display:flex;justify-content:flex-end;gap:8px;margin-top:8px}
.create-group-error{margin:0;color:#a84229;font-size:12px}
</style>
