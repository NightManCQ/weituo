export class 请求异常 extends Error {
  constructor(
    message: string,
    readonly 状态码: number,
  ) {
    super(message)
    this.name = '请求异常'
  }
}

interface 统一接口响应<T> {
  状态码: number
  消息: string
  数据: T
}

function 是统一接口响应(内容: unknown): 内容 is 统一接口响应<unknown> {
  return typeof 内容 === 'object'
    && 内容 !== null
    && '状态码' in 内容
    && typeof 内容.状态码 === 'number'
    && '消息' in 内容
    && typeof 内容.消息 === 'string'
    && '数据' in 内容
}

export async function 发送请求<T>(
  路径: string,
  初始化: RequestInit = {},
  令牌 = '',
): Promise<T> {
  const 请求头 = new Headers(初始化.headers)
  请求头.set('Content-Type', 'application/json')
  if (令牌) 请求头.set('Authorization', `Bearer ${令牌}`)

  const 响应 = await fetch(`${import.meta.env.VITE_API_BASE || '/api/v1'}${路径}`, {
    ...初始化,
    headers: 请求头,
  })
  const 原始内容 = await 响应.text()
  let 响应内容: unknown

  if (原始内容) {
    try {
      响应内容 = JSON.parse(原始内容)
    } catch {
      if (响应.ok) throw new Error('服务器返回了无法识别的响应')
      响应内容 = { 消息: 原始内容 }
    }
  }

  if (!响应.ok) {
    const 错误信息 = 是统一接口响应(响应内容)
      ? 响应内容.消息 || `请求失败 (${响应.status})`
      : typeof 响应内容 === 'object'
        && 响应内容 !== null
        && '消息' in 响应内容
        && typeof 响应内容.消息 === 'string'
        ? 响应内容.消息
      : `请求失败 (${响应.status})`
    throw new 请求异常(错误信息, 响应.status)
  }

  if (响应内容 === undefined) throw new Error('服务器返回了空响应')
  if (!是统一接口响应(响应内容)) {
    throw new Error('服务器返回了无法识别的响应结构')
  }
  if (响应内容.状态码 !== 响应.status) {
    throw new Error('服务器响应状态码与响应内容不一致')
  }
  return 响应内容.数据 as T
}
