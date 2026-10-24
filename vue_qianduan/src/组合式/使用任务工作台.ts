import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import mqtt, { type MqttClient } from 'mqtt'
import { 请求异常, 发送请求 } from '../接口/请求'
import { 生成唯一编号 } from '../领域/编号'
import type { 认证结果, 聊天会话, 积分账户, 积分流水, 消息, 实时消息事件, 任务, 任务操作, 任务位置, 用户 } from '../领域/类型'

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
type 实时凭据 = { 消息地址: string; 消息令牌: string; 主题列表: string[] }
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

function 解析实时消息(内容: string): 实时消息事件 {
  const 事件: unknown = JSON.parse(内容)
  if (typeof 事件 !== 'object' || 事件 === null) {
    throw new Error('实时消息必须是 JSON 对象')
  }

  const 记录 = 事件 as Record<string, unknown>
  const 事件类型 = typeof 记录.事件类型 === 'string' ? 记录.事件类型 : 记录.event_type
  const 任务编号 = typeof 记录.任务编号 === 'string' ? 记录.任务编号 : 记录.task_id
  if (
    (事件类型 !== undefined && typeof 事件类型 !== 'string') ||
    (任务编号 !== undefined && typeof 任务编号 !== 'string')
  ) {
    throw new Error('实时消息字段格式无效')
  }

  const 原始数据 = 记录.数据 ?? 记录.data
  let 数据: 消息 | undefined
  if (typeof 原始数据 === 'object' && 原始数据 !== null) {
    const 消息记录 = 原始数据 as Record<string, unknown>
    const 消息编号 = 消息记录.消息编号 ?? 消息记录.message_id
    if (typeof 消息编号 === 'string') {
      const 任务编号 = 消息记录.任务编号 ?? 消息记录.task_id
      const 发送者编号 = 消息记录.发送者编号 ?? 消息记录.sender_id
      const 发送者昵称 = 消息记录.发送者昵称 ?? 消息记录.sender_nickname
      const 客户端消息编号 = 消息记录.客户端消息编号 ?? 消息记录.client_message_id
      const 消息正文 = 消息记录.消息正文 ?? 消息记录.content
      const 创建时间 = 消息记录.创建时间 ?? 消息记录.created_at
      if (
        typeof 任务编号 !== 'string' ||
        typeof 发送者编号 !== 'string' ||
        typeof 发送者昵称 !== 'string' ||
        typeof 客户端消息编号 !== 'string' ||
        typeof 消息正文 !== 'string' ||
        typeof 创建时间 !== 'string'
      ) {
        throw new Error('实时聊天消息字段格式无效')
      }
      数据 = {
        消息编号,
        任务编号,
        发送者编号,
        发送者昵称,
        客户端消息编号,
        消息正文,
        创建时间,
      }
    }
  }

  return {
    事件类型: 事件类型 as string | undefined,
    任务编号: 任务编号 as string | undefined,
    客户端消息编号: typeof 记录.客户端消息编号 === 'string'
      ? 记录.客户端消息编号
      : undefined,
    错误消息: typeof 记录.错误消息 === 'string' ? 记录.错误消息 : undefined,
    数据,
  }
}

export function 使用任务工作台() {
  const 令牌 = ref(localStorage.getItem(用户令牌存储键) || '')
  const 当前用户 = ref(读取当前用户())
  const 任务列表 = ref<任务[]>([])
  const 我的任务列表 = ref<任务[]>([])
  const 聊天会话列表 = ref<聊天会话[]>([])
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
  let 实时客户端: MqttClient | null = null
  let 实时连接序号 = 0
  let 实时凭据刷新定时器: ReturnType<typeof setTimeout> | null = null
  let 消息回执定时器: ReturnType<typeof setTimeout> | null = null
  let 聊天发送序号 = 0
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
      return
    }
    正在加载聊天会话.value = true
    try {
      聊天会话列表.value = await 请求<聊天会话[]>('/my/conversations')
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

  async function 连接实时服务() {
    if (实时凭据刷新定时器) clearTimeout(实时凭据刷新定时器)
    实时凭据刷新定时器 = null
    if (!令牌.value || !当前用户.value) return
    const 连接序号 = ++实时连接序号
    const 当前令牌 = 令牌.value
    const 用户编号 = 当前用户.value.用户编号
    实时客户端?.end(true)
    实时客户端 = null

    try {
      const 凭据 = await 请求<实时凭据>('/realtime/credentials', { method: 'POST' })
      if (
        连接序号 !== 实时连接序号 ||
        令牌.value !== 当前令牌 ||
        当前用户.value?.用户编号 !== 用户编号
      ) return

      const MQTT地址 = import.meta.env.VITE_MQTT_URL || 凭据.消息地址
      const 新客户端 = mqtt.connect(MQTT地址, {
        username: 用户编号,
        password: 凭据.消息令牌,
        clean: true,
        reconnectPeriod: 3000,
        connectTimeout: 8000,
      })
      实时客户端 = 新客户端

      新客户端.on('connect', () => {
        if (实时客户端 !== 新客户端 || !凭据.主题列表.length) return
        新客户端.subscribe(凭据.主题列表, { qos: 1 }, (原因, 授权主题) => {
          if (实时客户端 !== 新客户端) return
          const 被拒绝主题 = 授权主题?.filter((主题) => (主题.qos as number) === 128) ?? []
          if (原因 || 被拒绝主题.length) {
            提示信息.value = '实时聊天订阅失败，请检查 EMQX 主题权限后刷新'
          }
        })
      })
      新客户端.on('reconnect', () => {
        if (实时客户端 === 新客户端) 提示信息.value = '实时连接正在恢复…'
      })
      新客户端.on('message', (_主题, 内容) => {
        if (实时客户端 !== 新客户端) return
        try {
          const 事件 = 解析实时消息(内容.toString())
          if (
            事件.事件类型 === 'chat.message_created' &&
            事件.数据
          ) {
            if (
              事件.任务编号 === 当前任务.value?.任务编号 &&
              !消息列表.value.some((消息项) => 消息项.消息编号 === 事件.数据?.消息编号)
            ) {
              消息列表.value = [...消息列表.value, 事件.数据].sort((左侧, 右侧) =>
                左侧.创建时间.localeCompare(右侧.创建时间),
              )
            }
            void 加载聊天会话()
            if (
              待确认消息.value?.客户端消息编号 === 事件.数据.客户端消息编号
            ) {
              清除待确认消息()
              提示信息.value = '消息已通过服务器验证并送达'
            }
          }
          if (事件.事件类型 === 'chat.message_rejected' && 待确认消息.value) {
            const 待确认项 = 待确认消息.value
            if (
              事件.客户端消息编号 === 待确认项.客户端消息编号 ||
              (!事件.客户端消息编号 && 事件.任务编号 === 待确认项.任务编号)
            ) {
              if (!消息表单数据.value) 消息表单数据.value = 待确认项.正文
              清除待确认消息()
              错误信息.value = 事件.错误消息 || '服务器拒绝了这条消息'
            }
          }
          if (事件.事件类型?.startsWith('task.') && 事件.任务编号) {
            void 加载任务()
            void 加载我的任务()
          }
        } catch (原因) {
          console.error('无法处理实时消息。', 原因)
          错误信息.value = '收到无法识别的实时消息'
        }
      })
      新客户端.on('error', () => {
        if (实时客户端 === 新客户端) 提示信息.value = '实时连接暂不可用，仍可通过刷新同步委托'
      })
      实时凭据刷新定时器 = setTimeout(() => {
        void 连接实时服务()
      }, 4 * 60 * 1000)
    } catch (原因) {
      if (连接序号 === 实时连接序号) {
        错误信息.value = 原因 instanceof Error ? 原因.message : '实时消息配置失败'
      }
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
    实时连接序号 += 1
    if (实时凭据刷新定时器) clearTimeout(实时凭据刷新定时器)
    实时凭据刷新定时器 = null
    清除待确认消息()
    令牌.value = ''
    当前用户.value = null
    我的任务列表.value = []
    我的积分账户.value = null
    我的积分流水.value = []
    聊天会话列表.value = []
    当前任务.value = null
    消息列表.value = []
    localStorage.removeItem(用户令牌存储键)
    localStorage.removeItem(用户资料存储键)
    实时客户端?.end(true)
    实时客户端 = null
  }

  async function 发布任务() {
    清除提示()
    发布反馈状态.value = { 状态: 'submitting', 消息: '正在提交委托…' }
    正在处理.value = true
    try {
      const 幂等键 = 发布任务幂等键.value || 生成唯一编号()
      发布任务幂等键.value = 幂等键
      await 请求<任务>('/tasks', {
        method: 'POST',
        headers: { 'Idempotency-Key': 幂等键 },
        body: JSON.stringify({
          title: 任务表单数据.value.title,
          description: 任务表单数据.value.description,
          reward_points: 任务表单数据.value.reward_points,
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
      发布反馈状态.value = { 状态: 'success', 消息: '委托发布成功，报酬积分已托管。' }
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
      await 连接实时服务()
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
    聊天发送序号 += 1
    if (消息回执定时器) clearTimeout(消息回执定时器)
    消息回执定时器 = null
    待确认消息.value = null
    消息待确认.value = false
    正在发送聊天消息.value = false
  }

  async function 发送聊天消息() {
    if (!当前任务.value || !当前用户.value || 正在发送聊天消息.value) return
    if (!实时客户端?.connected) {
      错误信息.value = '实时消息服务未连接，请稍后重试'
      return
    }

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
    消息待确认.value = false
    const 发送序号 = ++聊天发送序号
    if (消息回执定时器) clearTimeout(消息回执定时器)
    const 等待服务器验证 = () => {
      消息回执定时器 = setTimeout(() => {
        if (
          聊天发送序号 === 发送序号 &&
          待确认消息.value?.客户端消息编号 === 待发送项.客户端消息编号
        ) {
          正在发送聊天消息.value = false
          消息待确认.value = false
          提示信息.value = '服务器暂未确认，可点击“重试发送”再次提交'
        }
      }, 15_000)
    }
    等待服务器验证()
    const 主题 = `renwu/${待发送项.任务编号}/message_requests/${当前用户.value.用户编号}`
    const 请求内容 = JSON.stringify({
      客户端消息编号: 待发送项.客户端消息编号,
      消息正文: 待发送项.正文,
    })
    实时客户端.publish(主题, 请求内容, { qos: 1, retain: false }, (原因) => {
      if (
        聊天发送序号 !== 发送序号 ||
        待确认消息.value?.客户端消息编号 !== 待发送项.客户端消息编号
      ) return
      if (消息回执定时器) clearTimeout(消息回执定时器)
      消息回执定时器 = null
      正在发送聊天消息.value = false
      if (原因) {
        消息待确认.value = false
        错误信息.value = `消息发送失败：${原因.message}`
        return
      }
      消息表单数据.value = ''
      消息待确认.value = true
      提示信息.value = '消息已发送，等待服务器验证…'
      等待服务器验证()
    })
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
      await 连接实时服务()
    }
  })

  onBeforeUnmount(() => {
    if (实时凭据刷新定时器) clearTimeout(实时凭据刷新定时器)
    if (消息回执定时器) clearTimeout(消息回执定时器)
    实时客户端?.end(true)
  })

  return {
    当前用户,
    任务列表,
    我的任务列表,
    我的积分账户,
    我的积分流水,
    聊天会话列表,
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
