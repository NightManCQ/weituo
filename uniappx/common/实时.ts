import { 获取令牌, 读取当前用户 } from "./wangluo";
import { 读取MQTT地址 } from "./本地存储";
import { 会话列表, 新增或刷新会话, 更新会话最近消息 } from "../pages/xiaoxi/会话列表";
import { 使用全局变量, 用户信息 } from "../状态/全局.ts";
import type { 用户 } from "./leixing";
import type { MQTT事件数据, 实时事件, 实时监听器 } from "./实时.类型";

export type { MQTT事件, MQTT事件数据, MQTT回调, MQTT连接参数, MQTT遗嘱消息, 实时事件, 实时监听器 } from "./实时.类型";

import * as emqxMqtt from "@/uni_modules/changbi-mqtt";

const 实时监听器集合 = new Set<实时监听器>();

let 当前用户: 用户 | null = null;
let 当前身份键 = "";
let 连接中 = false;
let 连接代次 = 0;
let 正在连接: Promise<void> | null = null;
let 重连定时器: number | null = null;
let 重连次数 = 0;

function 读取记录值(记录: Record<string, unknown>, 名称列表: string[]): unknown {
  for (const 名称 of 名称列表) {
    const 值 = 记录[名称];
    if (值 !== undefined && 值 !== null) return 值;
  }
  return undefined;
}

function ArrayBuffer转文本(数据: ArrayBuffer): string {
  const 字节 = new Uint8Array(数据);
  let 文本 = "";
  const 分段长度 = 8192;
  for (let 偏移 = 0; 偏移 < 字节.length; 偏移 += 分段长度) {
    文本 += String.fromCharCode(...字节.slice(偏移, 偏移 + 分段长度));
  }
  return decodeURIComponent(escape(文本));
}

function 解析MQTT载荷(载荷: string | ArrayBuffer | undefined): Record<string, unknown> | null {
  if (载荷 == null) return null;
  const 文本 = typeof 载荷 == "string" ? 载荷 : ArrayBuffer转文本(载荷);
  const 内容 = JSON.parse(文本) as unknown;
  if (typeof 内容 != "object" || 内容 == null || Array.isArray(内容)) return null;
  return 内容 as Record<string, unknown>;
}

function 通知关闭时重连() {
  if (重连定时器 != null || 当前用户 == null || !获取令牌()) return;
  const 延迟 = Math.min(30000, 1000 * 2 ** Math.min(重连次数, 5));
  重连次数++;
  重连定时器 = setTimeout(() => {
    重连定时器 = null;
    void 连接();
  }, 延迟);
}

export function 订阅实时事件(回调: 实时监听器): () => void {
  实时监听器集合.add(回调);
  return () => 实时监听器集合.delete(回调);
}

// MQTT 事件在这里统一解析后，同时更新会话列表并通知聊天页面。
emqxMqtt.注册事件监听("message", (数据: MQTT事件数据) => {
  if (typeof 数据.主题 != "string") return;
  try {
    const 原始事件 = 解析MQTT载荷(数据.载荷 ?? 数据.二进制载荷);
    console.log("---------------mqtt事件", 原始事件);

    if (原始事件 == null) return;
    const 事件类型值 = 读取记录值(原始事件, ["事件类型", "event_type"]);
    const 事件类型 = typeof 事件类型值 == "string" ? 事件类型值 : "notification.unknown";
    const 数据值 = 读取记录值(原始事件, ["数据", "data"]);
    const 事件数据 = typeof 数据值 == "object" && 数据值 != null && !Array.isArray(数据值) ? (数据值 as Record<string, unknown>) : {};
    const 会话编号值 = 读取记录值(原始事件, ["会话编号", "conversation_id"]) ?? 读取记录值(事件数据, ["会话编号", "conversation_id"]);
    const 会话编号 = typeof 会话编号值 == "string" ? 会话编号值 : undefined;
    const 实时事件对象: 实时事件 = {
      主题: 数据.主题,
      事件类型,
      会话编号,
      数据: 事件数据,
      原始事件,
    };
    实时监听器集合.forEach((回调) => {
      try {
        回调(实时事件对象);
      } catch (原因) {
        console.error("实时消息监听处理失败", 原因);
      }
    });

    if (事件类型 == "私聊消息已创建") {
      const 消息正文 = 读取记录值(事件数据, ["消息正文", "message_body"]);
      const 创建时间 = 读取记录值(事件数据, ["创建时间", "created_at", "发生时间"]);
      const 私聊会话编号 = 读取记录值(事件数据, ["会话编号", "conversation_id"]);
      const 发送者编号 = 读取记录值(事件数据, ["发送者编号", "sender_user_id"]);
      const 当前用户编号 = 当前用户?.用户编号 || 用户信息.value.用户编号;
      if (发送者编号 != 当前用户编号 && typeof 私聊会话编号 == "string" && typeof 消息正文 == "string") {
        更新会话最近消息(私聊会话编号, 消息正文, typeof 创建时间 == "string" ? 创建时间 : new Date().toISOString(), 数据.主题);
      }
    }

    if (事件类型 == "群聊消息已创建") {
      const 消息正文 = 读取记录值(事件数据, ["消息正文", "message_body"]);
      const 创建时间 = 读取记录值(事件数据, ["创建时间", "created_at", "发生时间"]);
      const 群组编号 = 读取记录值(事件数据, ["群组编号", "group_id"]);
      const 发送者编号 = 读取记录值(事件数据, ["发送者编号", "sender_user_id"]);
      const 当前用户编号 = 当前用户?.用户编号 || 用户信息.value.用户编号;
      if (发送者编号 != 当前用户编号 && typeof 消息正文 == "string") {
        更新会话最近消息(typeof 群组编号 == "string" ? 群组编号 : "", 消息正文, typeof 创建时间 == "string" ? 创建时间 : new Date().toISOString(), 数据.主题);
      }
    }

    const 当前用户编号 = 当前用户?.用户编号 || 用户信息.value.用户编号;
    const 是当前用户通知主题 = 数据.主题 == `u/${当前用户编号}`;

    if (是当前用户通知主题 && 事件类型 == "好友验证已创建") {
      const 申请编号 = 读取记录值(事件数据, ["申请编号", "request_id"]);
      const 申请人编号 = 读取记录值(事件数据, ["申请人编号", "request_user_id"]);
      const 申请人登录名 = 读取记录值(事件数据, ["申请人登录名", "requester_login_name"]);
      const 申请人昵称 = 读取记录值(事件数据, ["申请人昵称", "requester_nickname"]);
      const 申请附言 = 读取记录值(事件数据, ["申请附言", "request_message"]);
      if (typeof 申请编号 == "string" && typeof 申请人编号 == "string") {
        新增或刷新会话({
          会话类型: "好友验证",
          会话编号: 申请编号,
          关联编号: 申请人编号,
          会话名称: typeof 申请人昵称 == "string" ? 申请人昵称 : typeof 申请人登录名 == "string" ? 申请人登录名 : "好友验证",
          联系人昵称: typeof 申请人昵称 == "string" ? 申请人昵称 : null,
          会话说明: "好友验证",
          最近消息: typeof 申请附言 == "string" ? 申请附言 : "请求添加你为好友",
          最近消息时间: typeof 原始事件.发生时间 == "string" ? 原始事件.发生时间 : new Date().toISOString(),
          会话状态: "待接受",
          成员数: null,
          申请人登录名: typeof 申请人登录名 == "string" ? 申请人登录名 : null,
          申请附言: typeof 申请附言 == "string" ? 申请附言 : null,
          MQTT主题: `u/${当前用户编号}`,
        });
      }
    }

    if (是当前用户通知主题 && 事件类型 == "群聊邀请已创建") {
      const 邀请编号 = 读取记录值(事件数据, ["邀请编号", "invitation_id"]);
      const 群组编号 = 读取记录值(事件数据, ["群组编号", "group_id"]);
      const 群组名称 = 读取记录值(事件数据, ["群组名称", "group_name"]);
      const 邀请人编号 = 读取记录值(事件数据, ["邀请人编号", "inviter_user_id"]);
      if (typeof 邀请编号 == "string" && typeof 群组编号 == "string") {
        新增或刷新会话({
          会话类型: "群邀请",
          会话编号: 邀请编号,
          关联编号: 群组编号,
          会话名称: typeof 群组名称 == "string" ? 群组名称 : "群聊邀请",
          联系人昵称: null,
          会话说明: "群聊邀请",
          最近消息: typeof 群组名称 == "string" ? `邀请你加入群聊：${群组名称}` : "邀请你加入群聊",
          最近消息时间: typeof 原始事件.发生时间 == "string" ? 原始事件.发生时间 : new Date().toISOString(),
          会话状态: "待接受",
          成员数: null,
          邀请人编号: typeof 邀请人编号 == "string" ? 邀请人编号 : null,
          MQTT主题: `u/${当前用户编号}`,
        });
      }
    }
  } catch (原因) {
    console.error("无法解析 MQTT 实时消息", 原因);
  }
});

emqxMqtt.注册事件监听("close", () => {
  if (uni.getSystemInfoSync().platform != "android") 通知关闭时重连();
});

export async function 连接实时消息(用户: 用户 | null = 读取当前用户()) {
  当前用户 = 用户;
  if (!用户 || !获取令牌()) {
    await 断开实时消息();
    return;
  }

  const 新身份键 = `${用户.用户编号}:${获取令牌()}`;
  if (新身份键 != 当前身份键) {
    await 断开实时消息();
    当前用户 = 用户;
    当前身份键 = 新身份键;
  }
  if (emqxMqtt.是否已连接()) return;
  if (正在连接 != null) return await 正在连接;
  await 连接();
}

async function 连接() {
  const 用户信息 = 当前用户;
  const 令牌 = 获取令牌();
  if (!用户信息 || !令牌 || 连接中) return;
  连接中 = true;
  const 本次连接代次 = 连接代次;
  const 连接任务 = (async () => {
    if (本次连接代次 != 连接代次 || 当前用户?.用户编号 != 用户信息.用户编号 || 获取令牌() != 令牌) return;

    try {
      const 保存地址 = 读取MQTT地址();
      let 地址 = import.meta.env.VITE_MQTT_URL || 保存地址 || "ws://localhost:9001/mqtt";
      // EMQX 通过 WebSocket 对外暴露，H5 与 App 原生插件使用不同传输地址
      if (uni.getSystemInfoSync().platform == "android") {
        地址 = "mqtt" + 地址 + ":1883";
      } else {
        地址 = "ws" + 地址 + ":8084/wss";
      }

      await emqxMqtt.连接({
        地址,
        客户端编号: (使用全局变量.系统信息.系统名 + "-" + 当前用户.用户编号).replace(/\s+/g, "") + "-" + Math.random(),
        用户名: 当前用户.用户编号,
        密码: 令牌,
        清理会话: true,
        连接超时: 10000,
        心跳间隔: 60,
        自动重连: true,
        重连间隔: 3000,
        最大重连次数: 10,
      });
      if (本次连接代次 != 连接代次) {
        await emqxMqtt.断开连接();
        return;
      }
      重连次数 = 0;
      const 主题列表 = [...new Set([`u/${用户信息.用户编号}`, ...会话列表.value.flatMap((会话) => (会话.MQTT主题 ? [会话.MQTT主题] : []))])];
      console.log("订阅的主题列表", 主题列表);

      await emqxMqtt.订阅(主题列表, 1);
    } catch (原因) {
      console.error("无法连接 MQTT 实时消息服务", 原因);
      if (本次连接代次 == 连接代次) 通知关闭时重连();
    }
  })();
  正在连接 = 连接任务;
  try {
    await 连接任务;
  } finally {
    if (正在连接 == 连接任务) {
      正在连接 = null;
      连接中 = false;
    }
  }
}

export async function 断开实时消息() {
  连接代次++;
  连接中 = false;
  当前用户 = null;
  当前身份键 = "";
  重连次数 = 0;
  if (重连定时器 != null) {
    clearTimeout(重连定时器);
    重连定时器 = null;
  }
  const 旧连接任务 = 正在连接;
  正在连接 = null;
  if (旧连接任务 != null) await 旧连接任务;
  try {
    await emqxMqtt.断开连接();
  } catch (原因) {
    console.error("断开 MQTT 实时连接失败", 原因);
  }
}
