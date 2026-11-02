const 用户信息键 = "用户信息";
const 令牌键 = "访问令牌";
const MQTT地址键 = "MQTT地址";
const API地址键 = "API地址";
const 委托位置键 = "委托位置";

export function 存储用户信息(数据: Any): void {
  uni.setStorageSync(用户信息键, 数据);
  uni.removeStorageSync("renwu_user");
}

export function 读取用户信息(): Any {
  const 数据 = uni.getStorageSync(用户信息键);
  if (typeof 数据 != "undefined" && 数据 != null) return 数据;

  const 旧数据 = uni.getStorageSync("renwu_user");
  if (typeof 旧数据 != "undefined" && 旧数据 != null) {
    存储用户信息(旧数据 as Any);
    return 旧数据;
  }
  return {};
}

export function 移除用户信息(): void {
  uni.removeStorageSync(用户信息键);
  uni.removeStorageSync("renwu_user");
}

export function 存储令牌(令牌: string): void {
  uni.setStorageSync(令牌键, 令牌);
  uni.removeStorageSync("renwu_token");
}

export function 读取令牌(): string {
  const 令牌 = uni.getStorageSync(令牌键);
  if (typeof 令牌 == "string" && 令牌.length > 0) return 令牌;

  const 旧令牌 = uni.getStorageSync("renwu_token");
  if (typeof 旧令牌 == "string" && 旧令牌.length > 0) {
    存储令牌(旧令牌);
    return 旧令牌;
  }
  return "";
}

export function 移除令牌(): void {
  uni.removeStorageSync(令牌键);
  uni.removeStorageSync("renwu_token");
}

export function 存储MQTT地址(地址: string): void {
  uni.setStorageSync(MQTT地址键, 地址);
  uni.removeStorageSync("renwu_mqtt_url");
}

export function 读取MQTT地址(): string {
  const 地址 = uni.getStorageSync(MQTT地址键);
  if (typeof 地址 == "string") return 地址;

  const 旧地址 = uni.getStorageSync("renwu_mqtt_url");
  if (typeof 旧地址 == "string") {
    存储MQTT地址(旧地址);
    return 旧地址;
  }
  return "";
}

export function 存储API地址(地址: string): void {
  uni.setStorageSync(API地址键, 地址);
  uni.removeStorageSync("renwu_api_base");
}

export function 读取API地址(): string {
  const 地址 = uni.getStorageSync(API地址键);
  if (typeof 地址 == "string") return 地址;

  const 旧地址 = uni.getStorageSync("renwu_api_base");
  if (typeof 旧地址 == "string") {
    存储API地址(旧地址);
    return 旧地址;
  }
  return "";
}

export function 存储委托位置(位置: Any): void {
  uni.setStorageSync(委托位置键, 位置);
  uni.removeStorageSync("renwu_location");
}

export function 读取委托位置(): Any {
  const 位置 = uni.getStorageSync(委托位置键);
  if (typeof 位置 != "undefined" && 位置 != null) return 位置;

  const 旧位置 = uni.getStorageSync("renwu_location");
  if (typeof 旧位置 != "undefined" && 旧位置 != null) {
    存储委托位置(旧位置 as Any);
    return 旧位置;
  }
  return null;
}

export function 移除委托位置(): void {
  uni.removeStorageSync(委托位置键);
  uni.removeStorageSync("renwu_location");
}

export function 移除登录信息(): void {
  移除令牌();
  移除用户信息();
}
