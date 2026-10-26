import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { 请求异常, 发送请求 } from '../接口/请求'
import { 生成唯一编号 } from '../领域/编号'
import { 连接全局实时服务, 断开全局实时服务, 订阅实时事件, type 实时事件 } from '../实时/消息中心'
import { 使用消息会话状态 } from '../状态/消息会话状态'
import type { 认证结果, 积分账户, 积分流水, 任务, 任务操作, 任务位置, 用户 } from '../领域/类型'

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
type 我发布的委托分类 = 'all' | 'ongoing' | 'completed'
type 我的委托归属 = 'published' | 'claimed'
type 分页委托响应 = { 任务列表: 任务[]; 总数: number }
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
  const 消息会话状态 = 使用消息会话状态()
  const 令牌 = ref(localStorage.getItem(用户令牌存储键) || '')
  const 当前用户 = ref(读取当前用户())
  const 任务列表 = ref<任务[]>([])
  const 我的任务列表 = ref<任务[]>([])
  const 我发布的委托列表 = ref<任务[]>([])
  const 我发布的委托总数 = ref(0)
  const 正在加载我发布的委托 = ref(false)
  const 我发布的委托还有更多 = ref(false)
  const 我发布的委托加载错误 = ref('')
  const 我发布的委托分类 = ref<我发布的委托分类>('all')
  const 我的委托归属 = ref<我的委托归属>('published')
  const 当前委托详情 = ref<任务 | null>(null)
  const 正在加载委托详情 = ref(false)
  const 委托详情错误 = ref('')
  const 错误信息 = ref('')
  const 提示信息 = ref('')
  const 正在处理 = ref(false)
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
  const 状态筛选 = ref('待接单')
  const 发布任务幂等键 = ref<string | null>(null)
  const 任务操作幂等键 = new Map<string, string>()
  let 委托详情请求序号 = 0
  let 我发布的委托请求序号 = 0

  消息会话状态.开始监听实时事件()
  消息会话状态.设置身份(令牌.value, 当前用户.value?.用户编号 ?? null)

  const 可见任务 = computed(() => 任务列表.value)
  const 我的协作任务 = computed(() => {
    if (!当前用户.value) return []
    return 任务列表.value.filter((任务项) =>
      任务项.发布者编号 === 当前用户.value?.用户编号 ||
      任务项.接单者编号 === 当前用户.value?.用户编号)
  })
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
      const 查询参数 = `?status=${encodeURIComponent(状态筛选.value)}`
      const 任务结果 = await 请求<任务[]>(`/tasks${查询参数}`)
      任务列表.value = 任务结果
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

  async function 加载我发布的委托(
    分类: 我发布的委托分类 = 我发布的委托分类.value,
    重置 = false,
    归属: 我的委托归属 = 我的委托归属.value,
  ) {
    if (!当前用户.value) {
      我发布的委托请求序号 += 1
      我发布的委托列表.value = []
      我发布的委托总数.value = 0
      我发布的委托还有更多.value = false
      我发布的委托加载错误.value = ''
      正在加载我发布的委托.value = false
      return
    }
    if (!重置 && (正在加载我发布的委托.value || !我发布的委托还有更多.value)) return

    if (重置) {
      我发布的委托分类.value = 分类
      我的委托归属.value = 归属
      我发布的委托请求序号 += 1
      我发布的委托列表.value = []
      我发布的委托总数.value = 0
      我发布的委托还有更多.value = true
    } else if (分类 !== 我发布的委托分类.value) {
      return
    }

    我发布的委托加载错误.value = ''
    const 请求序号 = 我发布的委托请求序号
    const 偏移量 = 重置 ? 0 : 我发布的委托列表.value.length
    正在加载我发布的委托.value = true
    try {
      const 参数 = new URLSearchParams({
        status: 分类,
        limit: '10',
        offset: String(偏移量),
      })
      const 路径 = 归属 === 'published' ? '/my/published-tasks' : '/my/claimed-tasks'
      const 结果 = await 请求<分页委托响应>(`${路径}?${参数}`)
      if (请求序号 !== 我发布的委托请求序号) return
      我发布的委托列表.value = 重置
        ? 结果.任务列表
        : [...我发布的委托列表.value, ...结果.任务列表]
      我发布的委托总数.value = 结果.总数
      我发布的委托还有更多.value = 我发布的委托列表.value.length < 结果.总数
    } catch (原因) {
      if (请求序号 === 我发布的委托请求序号) {
        我发布的委托加载错误.value =
          原因 instanceof Error ? 原因.message : '我发布的委托加载失败'
        错误信息.value = 原因 instanceof Error ? 原因.message : '我发布的委托加载失败'
      }
    } finally {
      if (请求序号 === 我发布的委托请求序号) {
        正在加载我发布的委托.value = false
      }
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
    await 消息会话状态.刷新会话列表()
  }

  function 处理全局实时事件(事件: 实时事件) {
    if (事件.事件类型.startsWith('task.')) {
      const 是当前用户委托被接单 =
        事件.事件类型 === 'task.claimed' &&
        事件.数据 &&
        事件.数据.发布者编号 === 当前用户.value?.用户编号
      void 加载任务()
      void 加载我的任务()
      void 加载我发布的委托(我发布的委托分类.value, true)
      if (事件.事件类型 === 'task.completed') void 连接实时服务(true)
      if (是当前用户委托被接单 && 事件.数据) {
        const 事件消息 = 事件.数据.消息
        const 标题 = 事件.数据.标题
        提示信息.value = typeof 事件消息 === 'string' && 事件消息
          ? 事件消息
          : typeof 标题 === 'string'
            ? `你的委托《${标题}》已被接单`
            : '你的委托已被接单'
      }
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
        当前用户.value.登录名,
        (状态, 消息) => {
          if (状态 === 'error') {
            错误信息.value = 消息 || '实时消息连接失败'
          } else if (状态 === 'reconnecting') {
            提示信息.value = 消息 || '实时连接正在恢复…'
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
      消息会话状态.设置身份(令牌.value, 当前用户.value.用户编号)
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
    令牌.value = ''
    当前用户.value = null
    消息会话状态.设置身份('', null)
    我的任务列表.value = []
    我发布的委托请求序号 += 1
    我发布的委托列表.value = []
    我发布的委托总数.value = 0
    我发布的委托还有更多.value = false
    我发布的委托加载错误.value = ''
    正在加载我发布的委托.value = false
    我的积分账户.value = null
    我的积分流水.value = []
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
      void 加载我发布的委托(我发布的委托分类.value, true)
      await 加载任务()
      await 加载我的任务()
      void 加载我发布的委托(我发布的委托分类.value, true)
      await 加载积分账户()
      await 加载聊天会话()
      await 连接实时服务(true)
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
      await 请求<任务>(`/tasks/${任务项.任务编号}/${操作}`, {
        method: 'POST',
        headers: { 'Idempotency-Key': 幂等键 },
      })
      任务操作幂等键.delete(操作标识)
      提示信息.value = 操作 === 'accept-completion'
        ? '验收完成，积分已发放'
        : 操作 === 'complete'
          ? '已提交完成，等待委托方验收'
          : '委托状态已更新'
      await 加载任务()
      await 加载我的任务()
      void 加载我发布的委托(我发布的委托分类.value, true)
      await 加载积分账户()
      await 加载聊天会话()
      await 连接实时服务(true)
    } catch (原因) {
      错误信息.value = 原因 instanceof Error ? 原因.message : '操作失败'
      await 加载任务()
      await 加载我的任务()
      void 加载我发布的委托(我发布的委托分类.value, true)
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
    我发布的委托列表,
    我发布的委托总数,
    正在加载我发布的委托,
    我发布的委托还有更多,
    我发布的委托加载错误,
    我的委托归属,
    我的积分账户,
    我的积分流水,
    当前委托详情,
    正在加载委托详情,
    委托详情错误,
    错误信息,
    提示信息,
    正在处理,
    是否注册,
    认证表单数据,
    任务表单数据,
    发布反馈状态,
    状态筛选,
    可见任务,
    我的协作任务,
    提交认证,
    退出登录,
    加载任务,
    加载我的任务,
    加载我发布的委托,
    加载积分账户,
    加载委托详情,
    清除委托详情,
    加载聊天会话,
    发布任务,
    执行任务操作,
    获取任务操作,
    格式化日期,
    聚焦任务标题,
  }
}
