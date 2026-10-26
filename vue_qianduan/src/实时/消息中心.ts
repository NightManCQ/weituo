import mqtt, { type MqttClient } from "mqtt";
import { 发送请求 } from "../接口/请求";

export type 实时事件 = {
  主题: string;
  事件类型: string;
  任务编号?: string;
  会话编号?: string;
  数据?: Record<string, unknown>;
  原始事件: Record<string, unknown>;
};

type 实时凭据 = {
  消息地址: string;
  消息令牌: string;
  主题列表: string[];
};

type 实时监听器 = (事件: 实时事件) => void;
type 连接状态监听器 = (状态: "connecting" | "connected" | "reconnecting" | "error", 消息?: string) => void;

const 监听器集合 = new Set<实时监听器>();
let 客户端: MqttClient | null = null;
let 当前身份键 = "";
let 连接代数 = 0;
let 正在连接身份键 = "";
let 正在连接: Promise<void> | null = null;
let 状态监听器: 连接状态监听器 | null = null;

export function 订阅实时事件(监听器: 实时监听器) {
  监听器集合.add(监听器);
  return () => 监听器集合.delete(监听器);
}

function 通知连接状态(状态: Parameters<连接状态监听器>[0], 消息?: string) {
  状态监听器?.(状态, 消息);
}

function 分发事件(事件: 实时事件) {
  for (const 监听器 of 监听器集合) {
    try {
      监听器(事件);
    } catch (原因) {
      console.error("实时消息监听器处理失败。", 原因);
    }
  }
}

function 关闭现有连接() {
  客户端?.end(true);
  客户端 = null;
  当前身份键 = "";
}

export async function 连接全局实时服务(
  令牌: string,
  用户编号: string,
  用户登录名: string,
  状态回调: 连接状态监听器,
  强制刷新 = false,
): Promise<void> {
  状态监听器 = 状态回调;
  if (!令牌 || !用户编号) {
    断开全局实时服务();
    return;
  }
  const 身份键 = `${用户编号}:${令牌}`;
  if (!强制刷新 && 当前身份键 === 身份键 && 客户端) return;
  if (!强制刷新 && 正在连接身份键 === 身份键 && 正在连接) return 正在连接;

  const 代数 = ++连接代数;
  关闭现有连接();
  正在连接身份键 = 身份键;
  通知连接状态("connecting");

  const 连接任务 = (async () => {
    const 凭据 = await 发送请求<实时凭据>("/realtime/credentials", { method: "POST" }, 令牌);
    if (代数 !== 连接代数) return;

    const MQTT地址 = import.meta.env.VITE_MQTT_URL || 凭据.消息地址;
    const 新客户端 = mqtt.connect(MQTT地址, {
      clientId: `${用户登录名}`,
      username: 用户编号,
      password: 凭据.消息令牌,
      clean: true,
      reconnectPeriod: 3000,
      connectTimeout: 8000,
    });
    客户端 = 新客户端;
    当前身份键 = 身份键;

    新客户端.on("connect", () => {
      if (客户端 !== 新客户端) return;
      if (!凭据.主题列表.length) {
        通知连接状态("connected", "实时连接成功，当前没有可订阅的主题");
        return;
      }
      新客户端.subscribe(凭据.主题列表, { qos: 1 }, (原因, 授权主题) => {
        if (客户端 !== 新客户端) return;
        const 被拒绝主题 = 授权主题?.filter((主题) => (主题.qos as number) === 128) ?? [];
        if (原因 || 被拒绝主题.length) {
          console.error("MQTT 实时主题订阅未获授权。", {
            原因: 原因?.message,
            被拒绝主题: 被拒绝主题.map((主题) => 主题.topic),
          });
          通知连接状态("error", "实时消息订阅失败，请检查 MQTT 主题权限");
          return;
        }
        通知连接状态("connected", `实时消息已连接并订阅 ${授权主题?.length ?? 0} 个主题`);
      });
    });
    新客户端.on("reconnect", () => {
      if (客户端 === 新客户端) 通知连接状态("reconnecting", "实时连接正在恢复…");
    });
    新客户端.on("message", (主题, 内容) => {
      if (客户端 !== 新客户端) return;
      const 原始文本 = 内容.toString();
      // console.log(`[MQTT] 收到主题「${主题}」的数据:`, 原始文本);
      try {
        const 原始事件: unknown = JSON.parse(原始文本);
        if (typeof 原始事件 !== "object" || 原始事件 === null || Array.isArray(原始事件)) {
          throw new Error("实时消息必须是 JSON 对象");
        }
        const 记录 = 原始事件 as Record<string, unknown>;
        const 事件类型 = 记录.事件类型 ?? 记录.event_type;
        const 任务编号 = 记录.任务编号 ?? 记录.task_id;
        const 会话编号 = 记录.会话编号 ?? 记录.conversation_id;
        const 数据 = 记录.数据 ?? 记录.data;
        if (事件类型 !== undefined && typeof 事件类型 !== "string") {
          throw new Error("实时消息事件类型无效");
        }
        if (任务编号 !== undefined && typeof 任务编号 !== "string") {
          throw new Error("实时消息任务编号无效");
        }
        if (会话编号 !== undefined && typeof 会话编号 !== "string") {
          throw new Error("实时消息会话编号无效");
        }
        if (数据 !== undefined && (typeof 数据 !== "object" || 数据 === null || Array.isArray(数据))) {
          throw new Error("实时消息数据必须是对象");
        }
        const 实时事件对象: 实时事件 = {
          主题,
          事件类型: typeof 事件类型 === "string" ? 事件类型 : "notification.unknown",
          任务编号: typeof 任务编号 === "string" ? 任务编号 : undefined,
          会话编号: typeof 会话编号 === "string" ? 会话编号 : undefined,
          数据: 数据 as Record<string, unknown> | undefined,
          原始事件: 记录,
        };
        console.log(`[MQTT] 主题「${主题}」解析成功:`, 实时事件对象);
        分发事件(实时事件对象);
      } catch (原因) {
        console.error("无法解析 MQTT 实时消息。", 原因);
        通知连接状态("error", "收到无法识别的实时消息");
      }
    });
    新客户端.on("error", (原因) => {
      if (客户端 === 新客户端) {
        console.error("MQTT 实时连接发生错误。", 原因);
        通知连接状态("error", "实时连接暂不可用");
      }
    });

  })();
  正在连接 = 连接任务;
  try {
    await 连接任务;
  } catch (原因) {
    if (代数 === 连接代数) {
      关闭现有连接();
      通知连接状态("error", 原因 instanceof Error ? 原因.message : "实时连接失败");
    }
    throw 原因;
  } finally {
    if (正在连接 === 连接任务) {
      正在连接 = null;
      正在连接身份键 = "";
    }
  }
}

export function 断开全局实时服务() {
  连接代数 += 1;
  正在连接 = null;
  正在连接身份键 = "";
  关闭现有连接();
  状态监听器 = null;
}
