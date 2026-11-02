import { ref } from 'vue'
import { 获取令牌, 读取当前用户, 发送请求 } from '../../common/wangluo'
import type { 联系人列表数据 } from '../../common/leixing'

export const 联系人列表 = ref<联系人列表数据>({
  数据: [],
  联系人个数: 0,
  群聊个数: 0,
})
export const 联系人列表正在加载 = ref(false)
export const 联系人列表加载错误 = ref('')

let 正在加载联系人列表的请求: Promise<联系人列表数据> | null = null

export function 刷新联系人列表(): Promise<联系人列表数据> {
  if (!获取令牌() || !读取当前用户()) {
    联系人列表.value = {
      数据: [],
      联系人个数: 0,
      群聊个数: 0,
    }
    联系人列表加载错误.value = ''
    return Promise.resolve(联系人列表.value)
  }

  if (正在加载联系人列表的请求) return 正在加载联系人列表的请求

  联系人列表正在加载.value = true
  正在加载联系人列表的请求 = 发送请求<联系人列表数据>('/my/contacts')
    .then((列表) => {
      联系人列表.value = {
        数据: Array.isArray(列表?.数据) ? 列表.数据 : [],
        联系人个数: typeof 列表?.联系人个数 == 'number' ? 列表.联系人个数 : 0,
        群聊个数: typeof 列表?.群聊个数 == 'number' ? 列表.群聊个数 : 0,
      }
      联系人列表加载错误.value = ''
      return 列表
    })
    .catch((原因) => {
      联系人列表加载错误.value = 原因 instanceof Error ? 原因.message : '加载联系人列表失败'
      throw 原因
    })
    .finally(() => {
      正在加载联系人列表的请求 = null
      联系人列表正在加载.value = false
    })
  return 正在加载联系人列表的请求
}

export function 发送好友申请(目标用户编号 : string, 附言 : string) : Promise<unknown> {
 return 发送请求('/my/friend-requests', {
   method: 'POST',
   data: {
     target_user_id: 目标用户编号,
     message: 附言,
   } as UTSJSONObject,
 })
}

export function 处理好友申请(申请编号 : string, 操作 : 'accept' | 'reject') : Promise<unknown> {
 return 发送请求(`/my/friend-requests/${encodeURIComponent(申请编号)}/${操作}`, {
   method: 'POST',
 })
}
