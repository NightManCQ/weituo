export type 用户 = {
  用户编号: string
  登录名: string
  昵称: string
}

export type 任务位置 = {
  经度: number
  纬度: number
  国: string | null
  省: string | null
  市: string | null
  县: string | null
  镇: string | null
  村: string | null
  地址: string | null
}

export type 任务 = {
  任务编号: string
  发布者编号: string
  接单者编号: string | null
  标题: string
  描述: string
  任务状态: string
  截止时间: string | null
  创建时间: string
  版本号: number
  位置: 任务位置 | null
  报酬积分: number | null
}

export type 积分账户 = {
  可用积分: number
  冻结积分: number
  总积分: number
}

export type 积分流水 = {
  流水编号: string
  交易编号: string
  交易类型: string
  委托编号: string | null
  说明: string
  可用变动: number
  冻结变动: number
  可用余额: number
  冻结余额: number
  创建时间: string
}

export type 消息 = {
  消息编号: string
  任务编号: string
  发送者编号: string
  发送者昵称: string
  客户端消息编号: string
  消息正文: string
  创建时间: string
}

export type 聊天会话 = {
  任务编号: string
  任务标题: string
  任务状态: string
  联系人编号: string
  联系人昵称: string
  最近消息: string
  最近消息时间: string
}

export type 认证结果 = {
  访问令牌: string
  令牌类型: string
  有效秒数: number
  用户: 用户
}

export type 任务操作 = 'claim' | 'start' | 'complete' | 'accept-completion' | 'cancel' | 'release'

export type 实时消息事件 = {
  事件类型?: string
  任务编号?: string
  客户端消息编号?: string
  错误消息?: string
  数据?: 消息
}
