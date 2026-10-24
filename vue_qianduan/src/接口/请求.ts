export class 请求异常 extends Error {
  constructor(
    message: string,
    readonly 状态码: number,
  ) {
    super(message)
    this.name = '请求异常'
  }
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
    const 错误内容 = 响应内容 as { 消息?: unknown } | undefined
    const 错误信息 = typeof 错误内容?.消息 === 'string'
      ? 错误内容.消息
      : `请求失败 (${响应.status})`
    throw new 请求异常(错误信息, 响应.status)
  }

  if (响应内容 === undefined) throw new Error('服务器返回了空响应')
  return 响应内容 as T
}
