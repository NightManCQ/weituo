<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { 发送请求 } from '../接口/请求'
import type { 用户 } from '../领域/类型'

type 用户搜索结果 = {
  用户编号: string
  登录名: string
  昵称: string
  好友状态: '已是好友' | '待你处理' | '已发送申请' | '可添加'
}

defineProps<{ 当前用户: 用户 | null }>()
const router = useRouter()
const 登录名查询 = ref('')
const 申请附言 = ref('')
const 搜索结果 = ref<用户搜索结果 | null>(null)
const 正在搜索 = ref(false)
const 正在处理 = ref(false)
const 错误信息 = ref('')
const 提示信息 = ref('')

async function 搜索用户() {
  const 登录名 = 登录名查询.value.trim()
  if (!登录名) {
    错误信息.value = '请输入要搜索的完整登录名'
    return
  }
  正在搜索.value = true
  错误信息.value = ''
  提示信息.value = ''
  搜索结果.value = null
  try {
    const 查询 = new URLSearchParams({ login_name: 登录名 })
    const 结果 = await 发送请求<用户搜索结果[]>(
      `/my/friends/search?${查询.toString()}`,
      {},
      localStorage.getItem('renwu_token') || '',
    )
    搜索结果.value = 结果[0] ?? null
    if (!搜索结果.value) 提示信息.value = '没有找到该登录名对应的用户'
  } catch (原因) {
    错误信息.value = 原因 instanceof Error ? 原因.message : '搜索用户失败'
  } finally {
    正在搜索.value = false
  }
}

async function 发送好友申请(用户: 用户搜索结果) {
  正在处理.value = true
  错误信息.value = ''
  提示信息.value = ''
  try {
    await 发送请求('/my/friend-requests', {
      method: 'POST',
      body: JSON.stringify({
        target_user_id: 用户.用户编号,
        message: 申请附言.value.trim(),
      }),
    }, localStorage.getItem('renwu_token') || '')
    申请附言.value = ''
    await 搜索用户()
    提示信息.value = '好友申请已发送，可在消息列表中查看处理进度'
  } catch (原因) {
    错误信息.value = 原因 instanceof Error ? 原因.message : '发送好友申请失败'
  } finally {
    正在处理.value = false
  }
}
</script>

<template>
  <section class="page-panel add-friend-page">
    <header class="page-heading">
      <div class="section-eyebrow">联系人</div>
      <h1>添加好友</h1>
      <p class="muted">通过用户登录名查找好友并发送验证消息。</p>
    </header>

    <div v-if="!当前用户" class="empty-state access-prompt">
      <h3>登录后添加好友</h3>
      <button class="button button-primary" @click="router.push({ name: 'profile' })">前往登录</button>
    </div>
    <template v-else>
      <p v-if="错误信息" class="add-friend-feedback add-friend-error" role="alert">{{ 错误信息 }}</p>
      <p v-if="提示信息" class="add-friend-feedback add-friend-success" role="status">{{ 提示信息 }}</p>
      <form class="add-friend-form" @submit.prevent="搜索用户">
        <label for="friend-login-search">用户登录名</label>
        <div class="add-friend-search-row">
          <input
            id="friend-login-search"
            v-model="登录名查询"
            maxlength="64"
            autocomplete="off"
            placeholder="输入完整登录名"
          />
          <button class="button button-primary" :disabled="正在搜索">
            {{ 正在搜索 ? '搜索中…' : '搜索' }}
          </button>
        </div>
      </form>

      <article v-if="搜索结果" class="add-friend-result">
        <span class="im-avatar">{{ 搜索结果.昵称.slice(0, 1) }}</span>
        <span class="add-friend-copy">
          <strong>{{ 搜索结果.昵称 }}</strong>
          <small>@{{ 搜索结果.登录名 }}</small>
        </span>
        <form
          v-if="搜索结果.好友状态 === '可添加'"
          class="add-friend-submit"
          @submit.prevent="发送好友申请(搜索结果)"
        >
          <input
            v-model="申请附言"
            maxlength="200"
            aria-label="好友申请附言"
            placeholder="验证消息（可选）"
          />
          <button class="button button-primary" :disabled="正在处理">
            {{ 正在处理 ? '发送中…' : '发送好友申请' }}
          </button>
        </form>
        <span v-else class="add-friend-status">{{ 搜索结果.好友状态 }}</span>
      </article>
    </template>
  </section>
</template>

<style scoped>
.add-friend-page{max-width:760px;margin:0 auto}
.add-friend-page>.page-heading{margin-bottom:24px}
.add-friend-form{padding:22px;border:1px solid var(--line);border-radius:14px;background:#fff}
.add-friend-form label{display:block;margin-bottom:8px;color:#566460;font-size:12px;font-weight:600}
.add-friend-search-row{display:flex;gap:9px}
.add-friend-search-row input{min-width:0;flex:1}
.add-friend-result{display:flex;align-items:center;gap:13px;margin-top:16px;padding:17px;border:1px solid var(--line);border-radius:14px;background:#fff}
.add-friend-copy{display:grid;gap:5px;min-width:90px}
.add-friend-copy strong{font-size:14px}
.add-friend-copy small{color:var(--muted);font-size:11px}
.add-friend-submit{display:flex;flex:1;gap:8px;min-width:0;margin-left:auto}
.add-friend-submit input{min-width:0;flex:1}
.add-friend-status{margin-left:auto;color:var(--muted);font-size:12px}
.add-friend-feedback{padding:11px 13px;border-radius:9px;font-size:12px}
.add-friend-error{background:#fff0ec;color:#a84229}
.add-friend-success{background:#e7f4ef;color:#126b56}
@media(max-width:600px){.add-friend-result{flex-wrap:wrap}.add-friend-submit{flex-basis:100%;margin-left:53px}.add-friend-submit button{padding:9px;font-size:11px}}
</style>
