import { defineStore } from "pinia";
import { ref } from "vue";
import { 读取用户信息 } from "../common/本地存储";

export const 使用全局变量 = defineStore("全局变量", () => {
  const 系统信息 = ref<any>({});
  return {
    系统信息,
  };
});

export const 用户信息 = ref<any>(读取用户信息());

// export function 全局变量初始化() {
//   用户信息.value = 读取用户信息();
// }
