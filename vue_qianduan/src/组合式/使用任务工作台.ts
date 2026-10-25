import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { 请求异常, 发送请求 } from '../接口/请求'
import { 生成唯一编号 } from '../领域/编号'
import { 连接全局实时服务, 断开全局实时服务, 订阅实时事件, type 实时事件 } from '../实时/消息中心'
import type { 认证结果, 好友聊天会话, 聊天会话, 积分账户, 积分流水, 消息, 消息会话, 任务, 任务操作, 任务位置, 用户 } from '../领域/类型'

type 认证表单 = { login_name: string; password: string; nickname: string }
type 任务表单 = {
  title: string
  description: string
  deadline: string
  reward_points: number
  位置: 任务位置 | null
}
type 任务操作项 = { key: 任务操作; label: string }
type 发布反馈 = { 状态: 'idle' | 'submitting' | 'success' | 'error'; 消息: string }
type 待确认聊天消息 = { 任务编号: string; 客户端消息编号: string; 正文: string }

const 用户令牌存储键 = 'renwu_token'
const 用户资料存储键 = 'renwu_user'

function 读取当前用户(): 用户 | null {
  const 用户资料 = localStorage.getItem(用户资料存储键)
  if (!用户资料) return null

  try {
    const 用户信息: unknown = JSON.parse(用户资料)
    if (typeof 用户信息 === 'object' && 用户信息 !== null) {
      if (
        '用户编号' in 用户信息 &&
        typeof 用户信息.用户编号 === 'string' &&
        '登录名' in 用户信息 &&
        typeof 用户信息.登录名 === 'string' &&
        '昵称' in 用户信息 &&
        typeof 用户信息.昵称 === 'string'
      ) {
        return 用户信息 as 用户
      }
      if (
        'user_id' in 用户信息 &&
        typeof 用户信息.user_id === 'string' &&
        'login_name' in 用户信息 &&
        typeof 用户信息.login_name === 'string' &&
        'nickname' in 用户信息 &&
        typeof 用户信息.nickname === 'string'
      ) {
        const 迁移后用户: 用户 = {
          用户编号: 用户信息.user_id,
          登录名: 用户信息.login_name,
          昵称: 用户信息.nickname,
        }
        localStorage.setItem(用户资料存储键, JSON.stringify(迁移后用户))
        return 迁移后用户
      }
    }
  } catch (原因) {
    console.warn('本地用户信息无效，将要求重新登录。', 原因)
  }

  console.warn('本地用户信息格式无效，将要求重新登录。')
  localStorage.removeItem(用户资料存储键)
  localStorage.removeItem(用户令牌存储键)
  return null
}

export function 使用任务工作台() {
  const 令牌 = ref(localStorage.getItem(用户令牌存储键) || '')
  const 当前用户 = ref(读取当前用户())
  const 任务列表 = ref<任务[]>([])
  const 我的任务列表 = ref<任务[]>([])
  const 聊天会话列表 = ref<聊天会话[]>([])
  const 好友聊天会话列表 = ref<好友聊天会话[]>([])
  const 消息会话列表 = ref<消息会话[]>([])
  const 当前任务 = ref<任务 | null>(null)
  const 当前委托详情 = ref<任务 | null>(null)
  const 正在加载委托详情 = ref(false)
  const 委托详情错误 = ref('')
  const 消息列表 = ref<消息[]>([])
  const 错误信息 = ref('')
  const 提示信息 = ref('')
  const 正在处理 = ref(false)
  const 正在加载聊天会话 = ref(false)
  const 是否注册 = ref(false)
  const 认证表单数据 = ref<认证表单>({ login_name: '', password: '', nickname: '' })
  const 任务表单数据 = ref<任务表单>({
    title: '',
    description: '',
    deadline: '',
    reward_points: 0,
    位置: null,
  })
  const 我的积分账户 = ref<积分账户 | null>(null)
  const 我的积分流水 = ref<积分流水[]>([])
  const 发布反馈状态 = ref<发布反馈>({ 状态: 'idle', 消息: '' })
  const 消息表单数据 = ref('')
  const 待确认消息 = ref<待确认聊天消息 | null>(null)
  const 正在发送聊天消息 = ref(false)
  const 消息待确认 = ref(false)
  const 状态筛选 = ref('')
  const 发布任务幂等键 = ref<string | null>(null)
  const 任务操作幂等键 = new Map<string, string>()
  let 委托详情请求序号 = 0

  const 可见任务 = computed(() => 任务列表.value)
  const 我的协作任务 = computed(() => {
    if (!当前用户.value) return []
    return 任务列表.value.filter((任务项) =>
      任务项.发布者编号 === 当前用户.value?.用户编号 ||
      任务项.接单者编号 === 当前用户.value?.用户编号)
  })
  const 可以聊天 = computed(() => !!当前用户.value && !!当前任务.value &&
    (当前任务.value.发布者编号 === 当前用户.value.用户编号 ||
      当前任务.value.接单者编号 === 当前用户.value.用户编号))
  const 可以发送消息 = computed(() => !!当前任务.value &&
    (待确认消息.value !== null || 消息表单数据.value.trim().length > 0))

  function 清除提示() {
    错误信息.value = ''
    提示信息.value = ''
  }

  async function 请求<T>(路径: string, 初始化: RequestInit = {}): Promise<T> {
    try {
      return await 发送请求<T>(路径, 初始化, 令牌.value)
    } catch (原因) {
      if (原因 instanceof 请求异常 && 原因.状态码 === 401 && 令牌.value) 退出登录()
      throw 原因
    }
  }

  async function 加载任务() {
    清除提示()
    try {
      const 查询参数 = 状态筛选.value ? `?status=${encodeURIComponent(状态筛选.value)}` : ''
      const 任务结果 = await 请求<任务[]>(`/tasks${查询参数}`)
      任务列表.value = 任务结果
      if (当前任务.value) {
        当前任务.value = 任务结果.find((任务项) => 任务项.任务编号 === 当前任务.value?.任务编号) || 当前任务.value
      }
    } catch (原因) {
      错误信息.value = 原因 instanceof Error ? 原因.message : '委托加载失败'
    }
  }

  async function 加载我的任务() {
    if (!当前用户.value) {
      我的任务列表.value = []
      return
    }
    try {
      我的任务列表.value = await 请求<任务[]>('/my/tasks')
    } catch (原因) {
      错误信息.value = 原因 instanceof Error ? 原因.message : '我的委托加载失败'
    }
  }

  async function 加载积分账户() {
    if (!当前用户.value) {
      我的积分账户.value = null
      我的积分流水.value = []
      return
    }
    try {
      const [账户, 流水] = await Promise.all([
        请求<积分账户>('/my/points'),
        请求<积分流水[]>('/my/points/ledger?limit=30'),
      ])
      我的积分账户.value = 账户
      我的积分流水.value = 流水
    } catch (原因) {
      错误信息.value = 原因 instanceof Error ? 原因.message : '积分账户加载失败'
    }
  }

  async function 加载委托详情(委托编号: string) {
    const 请求序号 = ++委托详情请求序号
    当前委托详情.value = null
    委托详情错误.value = ''
    正在加载委托详情.value = true
    try {
      const 委托详情 = await 请求<任务>(`/weituo/${encodeURIComponent(委托编号)}`)
      if (请求序号 === 委托详情请求序号) 当前委托详情.value = 委托详情
    } catch (原因) {
      if (请求序号 === 委托详情请求序号) {
        委托详情错误.value = 原因 instanceof Error ? 原因.message : '委托详情加载失败'
      }
    } finally {
      if (请求序号 === 委托详情请求序号) 正在加载委托详情.value = false
    }
  }

  function 清除委托详情() {
    委托详情请求序号 += 1
    当前委托详情.value = null
    委托详情错误.value = ''
    正在加载委托详情.value = false
  }

  async function 加载聊天会话() {
    if (!当前用户.value) {
      聊天会话列表.value = []
      好友聊天会话列表.value = []
      消息会话列表.value = []
      return
    }
    正在加载聊天会话.value = true
    try {
      const 会话列表 = await 请求<消息会话[]>('/my/message-conversations')
      消息会话列表.value = 会话列表
      聊天会话列表.value = 会话列表
        .filter((会话) => 会话.会话类型 === '委托')
        .map((会话) => ({
          任务编号: 会话.会话编号,
          任务标题: 会话.会话名称,
          任务状态: 会话.会话状态 ?? '待接单',
          联系人编号: 会话.关联编号,
          联系人昵称: 会话.联系人昵称 ?? 会话.会话名称,
          最近消息: 会话.最近消息 ?? '尚无消息',
          最近消息时间: 会话.最近消息时间,
        }))
      好友聊天会话列表.value = 会话列表
        .filter((会话) => 会话.会话类型 === '好友')
        .map((会话) => ({
          会话编号: 会话.会话编号,
          联系人编号: 会话.关联编号,
          联系人昵称: 会话.会话名称,
          最近消息: 会话.最近消息,
          最近消息时间: 会话.最近消息时间,
        }))
    } catch (原因) {
      错误信息.value = 原因 instanceof Error ? 原因.message : '聊天会话加载失败'
    } finally {
      正在加载聊天会话.value = false
    }
  }

  async function 加载消息(任务编号: string) {
    try {
      const 消息结果 = await 请求<消息[]>(`/tasks/${任务编号}/messages?limit=100`)
      if (当前任务.value?.任务编号 === 任务编号) {
        const 合并消息 = new Map(
          [...消息结果, ...消息列表.value].map((消息项) => [消息项.消息编号, 消息项]),
        )
        消息列表.value = [...合并消息.values()].sort((左侧, 右侧) =>
          左侧.创建时间.localeCompare(右侧.创建时间),
        )
      }
    } catch (原因) {
      错误信息.value = 原因 instanceof Error ? 原因.message : '聊天消息加载失败'
    }
  }

  function 处理全局实时事件(事件: 实时事件) {
    if (事件.事件类型 === 'chat.message_created' && 事件.数据) {
      const 数据 = 事件.数据
      const 消息编号 = 数据.消息编号 ?? 数据.message_id
      const 任务编号 = 数据.任务编号 ?? 数据.task_id ?? 事件.任务编号
      const 发送者编号 = 数据.发送者编号 ?? 数据.sender_id
      const 发送者昵称 = 数据.发送者昵称 ?? 数据.sender_nickname
      const 客户端消息编号 = 数据.客户端消息编号 ?? 数据.client_message_id
      const 消息正文 = 数据.消息正文 ?? 数据.content
      const 创建时间 = 数据.创建时间 ?? 数据.created_at
      if (
        typeof 消息编号 === 'string' &&
        typeof 任务编号 === 'string' &&
        typeof 发送者编号 === 'string' &&
        typeof 发送者昵称 === 'string' &&
        typeof 客户端消息编号 === 'string' &&
        typeof 消息正文 === 'string' &&
        typeof 创建时间 === 'string'
      ) {
        const 消息项: 消息 = {
          消息编号,
          任务编号,
          发送者编号,
          发送者昵称,
          客户端消息编号,
          消息正文,
          创建时间,
        }
        if (
          当前任务.value?.任务编号 === 任务编号 &&
          !消息列表.value.some((已有消息) => 已有消息.消息编号 === 消息项.消息编号)
        ) {
          消息列表.value = [...消息列表.value, 消息项].sort((左侧, 右侧) =>
            左侧.创建时间.localeCompare(右侧.创建时间),
          )
        }
        void 加载聊天会话()
      } else {
        console.error('任务聊天 MQTT 事件缺少消息字段。', 事件.原始事件)
      }
      return
    }

    if (事件.事件类型 === 'friend.message_created') {
      const 数据 = 事件.数据
      const 会话编号 = 事件.会话编号 ?? 数据?.会话编号
      const 联系人编号 = 数据?.联系人编号
      const 联系人昵称 = 数据?.联系人昵称
      const 最近消息 = 数据?.消息正文
      const 最近消息时间 = 数据?.创建时间
      if (
        typeof 会话编号 !== 'string' ||
        typeof 联系人编号 !== 'string' ||
        typeof 联系人昵称 !== 'string' ||
        typeof 最近消息 !== 'string' ||
        typeof 最近消息时间 !== 'string'
      ) {
        console.error('好友私聊 MQTT 事件缺少会话列表字段。', 事件.原始事件)
        return
      }

      const 已有会话 = 好友聊天会话列表.value.find(
        (会话) => 会话.会话编号 === 会话编号,
      )
      const 新会话: 好友聊天会话 = {
        会话编号,
        联系人编号,
        联系人昵称,
        最近消息,
        最近消息时间,
      }
      const 当前最新时间 = 已有会话?.最近消息时间
      if (当前最新时间 && 当前最新时间 > 最近消息时间) {
        新会话.最近消息 = 已有会话.最近消息
        新会话.最近消息时间 = 当前最新时间
      }
      好友聊天会话列表.value = [
        新会话,
        ...好友聊天会话列表.value.filter((会话) => 会话.会话编号 !== 会话编号),
      ].sort((左侧, 右侧) =>
        (右侧.最近消息时间 ?? '').localeCompare(左侧.最近消息时间 ?? ''),
      )
      void 加载聊天会话()
      return
    }

    if (事件.事件类型 === 'group.message_created') {
      void 加载聊天会话()
      return
    }

    if (事件.事件类型.startsWith('task.')) {
      void 加载任务()
      void 加载我的任务()
      return
    }

    const 通知文本 = 事件.数据?.消息 ?? 事件.数据?.message
    if (typeof 通知文本 === 'string' && 通知文本.trim()) {
      提示信息.value = 通知文本
    } else if (
      事件.事件类型.startsWith('notification.') ||
      事件.事件类型.startsWith('friend.request_')
    ) {
      提示信息.value = '收到一条新通知'
    }
  }

  const 取消实时事件订阅 = 订阅实时事件(处理全局实时事件)

  async function 连接实时服务(强制刷新 = false) {
    if (!令牌.value || !当前用户.value) return
    try {
      await 连接全局实时服务(
        令牌.value,
        当前用户.value.用户编号,
        (状态, 消息) => {
          if (状态 === 'error') {
            错误信息.value = 消息 || '实时消息连接失败'
          } else if (状态 === 'reconnecting') {
            提示信息.value = 消息 || '实时连接正在恢复…'
          } else if (状态 === 'connected' && 消息) {
            提示信息.value = 消息
          }
        },
        强制刷新,
      )
    } catch (原因) {
      错误信息.value = 原因 instanceof Error ? 原因.message : '实时消息配置失败'
    }
  }

  async function 提交认证() {
    清除提示()
    正在处理.value = true
    try {
      const 路径 = 是否注册.value ? '/auth/register' : '/auth/login'
      const 认证结果 = await 发送请求<认证结果>(路径, {
        method: 'POST',
        body: JSON.stringify(认证表单数据.value),
      })
      令牌.value = 认证结果.访问令牌
      当前用户.value = 认证结果.用户
      localStorage.setItem(用户令牌存储键, 令牌.value)
      localStorage.setItem(用户资料存储键, JSON.stringify(当前用户.value))
      await 加载任务()
      await 加载我的任务()
      await 加载积分账户()
      await 加载聊天会话()
      await 连接实时服务()
    } catch (原因) {
      错误信息.value = 原因 instanceof Error ? 原因.message : '登录失败'
    } finally {
      正在处理.value = false
    }
  }

  function 退出登录() {
    断开全局实时服务()
    清除待确认消息()
    令牌.value = ''
    当前用户.value = null
    我的任务列表.value = []
    我的积分账户.value = null
    我的积分流水.value = []
    聊天会话列表.value = []
    好友聊天会话列表.value = []
    消息会话列表.value = []
    当前任务.value = null
    消息列表.value = []
    localStorage.removeItem(用户令牌存储键)
    localStorage.removeItem(用户资料存储键)
  }

  async function 发布任务() {
    清除提示()
    发布反馈状态.value = { 状态: 'submitting', 消息: '正在提交委托…' }
    正在处理.value = true
    try {
      const 幂等键 = 发布任务幂等键.value || 生成唯一编号()
      const 报酬积分 = Number(任务表单数据.value.reward_points)
      发布任务幂等键.value = 幂等键
      await 请求<任务>('/tasks', {
        method: 'POST',
        headers: { 'Idempotency-Key': 幂等键 },
        body: JSON.stringify({
          title: 任务表单数据.value.title,
          description: 任务表单数据.value.description,
          ...(Number.isInteger(报酬积分)
            ? { reward_points: 报酬积分 }
            : {}),
          deadline: 任务表单数据.value.deadline
            ? new Date(任务表单数据.value.deadline).toISOString()
            : null,
          location: 任务表单数据.value.位置
            ? {
                longitude: 任务表单数据.value.位置.经度,
                latitude: 任务表单数据.value.位置.纬度,
                country: 任务表单数据.value.位置.国,
                province: 任务表单数据.value.位置.省,
                city: 任务表单数据.value.位置.市,
                county: 任务表单数据.value.位置.县,
                town: 任务表单数据.value.位置.镇,
                village: 任务表单数据.value.位置.村,
                address: 任务表单数据.value.位置.地址,
              }
            : null,
        }),
      })
      发布任务幂等键.value = null
      任务表单数据.value = {
        title: '',
        description: '',
        deadline: '',
        reward_points: 0,
        位置: null,
      }
      发布反馈状态.value = {
        状态: 'success',
        消息: 报酬积分 > 0
          ? '委托发布成功，报酬积分已托管。'
          : '委托发布成功，本委托不设置积分报酬。',
      }
      提示信息.value = '委托已发布'
      await 加载任务()
      await 加载我的任务()
      await 加载积分账户()
      await 加载聊天会话()
      await 连接实时服务()
    } catch (原因) {
      const 错误 = 原因 instanceof Error ? 原因.message : '发布失败，请稍后重试'
      发布反馈状态.value = { 状态: 'error', 消息: `委托发布失败：${错误}` }
      错误信息.value = 错误
    } finally {
      正在处理.value = false
    }
  }

  async function 执行任务操作(任务项: 任务, 操作: 任务操作) {
    清除提示()
    if (
      操作 === 'accept-completion' &&
      !window.confirm(`确认验收“${任务项.标题}”并向接单方发放 ${任务项.报酬积分 ?? 0} 积分？`)
    ) return
    const 操作标识 = `${任务项.任务编号}/${操作}`
    const 幂等键 = 任务操作幂等键.get(操作标识) || 生成唯一编号()
    任务操作幂等键.set(操作标识, 幂等键)
    try {
      const 已更新任务 = await 请求<任务>(`/tasks/${任务项.任务编号}/${操作}`, {
        method: 'POST',
        headers: { 'Idempotency-Key': 幂等键 },
      })
      任务操作幂等键.delete(操作标识)
      当前任务.value = 已更新任务
      提示信息.value = 操作 === 'accept-completion'
        ? '验收完成，积分已发放'
        : 操作 === 'complete'
          ? '已提交完成，等待委托方验收'
          : '委托状态已更新'
      await 加载任务()
      await 加载我的任务()
      await 加载积分账户()
      await 加载聊天会话()
      await 连接实时服务(true)
    } catch (原因) {
      错误信息.value = 原因 instanceof Error ? 原因.message : '操作失败'
      await 加载任务()
      await 加载我的任务()
    }
  }

  async function 选择任务(任务项: 任务) {
    清除待确认消息()
    当前任务.value = 任务项
    消息列表.value = []
    消息表单数据.value = ''
    if (
      当前用户.value &&
      (任务项.发布者编号 === 当前用户.value.用户编号 || 任务项.接单者编号 === 当前用户.value.用户编号)
    ) {
      await 加载消息(任务项.任务编号)
    }
  }

  async function 选择聊天会话(会话: 聊天会话) {
    try {
      const 任务项 = await 请求<任务>(`/tasks/${会话.任务编号}`)
      await 选择任务(任务项)
    } catch (原因) {
      错误信息.value = 原因 instanceof Error ? 原因.message : '聊天会话打开失败'
    }
  }

  async function 加载聊天任务(任务编号: string): Promise<任务 | null> {
    try {
      return await 请求<任务>(`/tasks/${encodeURIComponent(任务编号)}`)
    } catch (原因) {
      错误信息.value = 原因 instanceof Error ? 原因.message : '聊天会话打开失败'
      return null
    }
  }

  function 清除待确认消息() {
    待确认消息.value = null
    消息待确认.value = false
    正在发送聊天消息.value = false
  }

  async function 发送聊天消息() {
    if (!当前任务.value || !当前用户.value || 正在发送聊天消息.value) return
    const 待发送项 = 待确认消息.value ?? {
      任务编号: 当前任务.value.任务编号,
      客户端消息编号: 生成唯一编号(),
      正文: 消息表单数据.value.trim(),
    }
    if (!待发送项.正文) return
    if (待发送项.任务编号 !== 当前任务.value.任务编号) {
      错误信息.value = '当前待确认消息属于其他委托，请重新打开会话'
      return
    }

    待确认消息.value = 待发送项
    正在发送聊天消息.value = true
    消息待确认.value = true
    错误信息.value = ''
    try {
      const 已保存消息 = await 请求<消息>(
        `/tasks/${encodeURIComponent(待发送项.任务编号)}/messages`,
        {
          method: 'POST',
          body: JSON.stringify({
            客户端消息编号: 待发送项.客户端消息编号,
            消息正文: 待发送项.正文,
          }),
        },
      )
      if (当前任务.value?.任务编号 === 待发送项.任务编号) {
        const 消息按编号 = new Map(
          [...消息列表.value, 已保存消息].map((消息项) => [消息项.消息编号, 消息项]),
        )
        消息列表.value = [...消息按编号.values()].sort((左侧, 右侧) =>
          左侧.创建时间.localeCompare(右侧.创建时间),
        )
      }
      消息表单数据.value = ''
      清除待确认消息()
      提示信息.value = '消息已保存并发送'
      void 加载聊天会话()
    } catch (原因) {
      错误信息.value = 原因 instanceof Error ? 原因.message : '任务聊天消息发送失败'
    } finally {
      正在发送聊天消息.value = false
      消息待确认.value = false
    }
  }

  function 获取任务操作(任务项: 任务): 任务操作项[] {
    if (!当前用户.value) return []
    const 是发布者 = 任务项.发布者编号 === 当前用户.value.用户编号
    const 是接单者 = 任务项.接单者编号 === 当前用户.value.用户编号
    if (任务项.任务状态 === '待接单') {
      if (是发布者 || 任务项.报酬积分 === null) return []
      return [{ key: 'claim', label: '接单' }]
    }
    if (任务项.任务状态 === '已接单') {
      if (是接单者) return [{ key: 'start', label: '开始' }, { key: 'release', label: '释放' }]
      if (是发布者) return [{ key: 'cancel', label: '取消' }]
    }
    if (任务项.任务状态 === '进行中' && 是接单者) return [{ key: 'complete', label: '完成' }]
    if (任务项.任务状态 === '待验收' && 是发布者) {
      return [{ key: 'accept-completion', label: '验收并发放积分' }]
    }
    return []
  }

  function 格式化日期(日期: string) {
    return new Date(日期).toLocaleString('zh-CN', { dateStyle: 'medium', timeStyle: 'short' })
  }

  function 聚焦任务标题() {
    document.getElementById('publish-title')?.focus()
  }

  onMounted(async () => {
    await 加载任务()
    if (令牌.value && 当前用户.value) {
      await 加载我的任务()
      await 加载积分账户()
      await 加载聊天会话()
      await 连接实时服务(true)
    }
  })

  onBeforeUnmount(() => {
    取消实时事件订阅()
    断开全局实时服务()
  })

  return {
    当前用户,
    任务列表,
    我的任务列表,
    我的积分账户,
    我的积分流水,
    聊天会话列表,
    好友聊天会话列表,
    消息会话列表,
    当前任务,
    当前委托详情,
    正在加载委托详情,
    委托详情错误,
    消息列表,
    错误信息,
    提示信息,
    正在处理,
    正在加载聊天会话,
    是否注册,
    认证表单数据,
    任务表单数据,
    发布反馈状态,
    消息表单数据,
    状态筛选,
    可见任务,
    我的协作任务,
    可以聊天,
    可以发送消息,
    正在发送聊天消息,
    消息待确认,
    待确认消息,
    提交认证,
    退出登录,
    加载任务,
    加载我的任务,
    加载积分账户,
    加载委托详情,
    清除委托详情,
    加载聊天会话,
    发布任务,
    执行任务操作,
    选择任务,
    选择聊天会话,
    加载聊天任务,
    发送聊天消息,
    获取任务操作,
    格式化日期,
    聚焦任务标题,
  }
}
