import { defineStore } from 'pinia'
import { ref } from 'vue'
import { 发送请求 } from '../common/wangluo'
import { 订阅实时事件 } from '../common/实时'
import type { 消息会话 } from '../common/leixing'

export const 使用消息会话状态 = defineStore('消息会话', () => {
  const 会话列表 = ref<消息会话[]>([])
  const 正在加载 = ref(false)
  const 加载错误 = ref('')
  let 令牌 = ''
  let 当前用户编号 = ''
  let 请求序号 = 0
  let 已开始监听实时事件 = false

  async function 刷新会话列表() {
    if (!令牌 || !当前用户编号) {
      会话列表.value = []
      正在加载.value = false
      加载错误.value = ''
      return
    }

    const 当前请求序号 = ++请求序号
    正在加载.value = true
    try {
      const 列表 = await 发送请求<消息会话[]>('/my/message-conversations')
      if (当前请求序号 !== 请求序号) return
      会话列表.value = 列表
      加载错误.value = ''
    } catch (原因) {
      if (当前请求序号 === 请求序号) {
        加载错误.value = 原因 instanceof Error ? 原因.message : '会话列表加载失败'
        console.error('消息会话列表同步失败。', 原因)
      }
    } finally {
      if (当前请求序号 === 请求序号) 正在加载.value = false
    }
  }

  function 设置身份(访问令牌: string, 用户编号: string | null) {
    const 新令牌 = 用户编号 ? 访问令牌 : ''
    const 新用户编号 = 用户编号 ?? ''
    if (新令牌 === 令牌 && 新用户编号 === 当前用户编号) return

    令牌 = 新令牌
    当前用户编号 = 新用户编号
    请求序号 += 1
    会话列表.value = []
    加载错误.value = ''
    正在加载.value = false
  }

  function 开始监听实时事件() {
    if (已开始监听实时事件) return
    已开始监听实时事件 = true
    订阅实时事件((事件) => {
      const 影响会话的事件 =
        事件.事件类型.startsWith('task.') ||
        事件.事件类型.startsWith('group.') ||
        事件.事件类型.startsWith('friend.')
      if (影响会话的事件 && 令牌 && 当前用户编号) {
        void 刷新会话列表()
      }
    })
  }

  return {
    会话列表,
    正在加载,
    加载错误,
    刷新会话列表,
    设置身份,
    开始监听实时事件,
  }
})
