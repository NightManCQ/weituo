import { defineStore } from 'pinia'
import { ref } from 'vue'


export const 使用全局变量 = defineStore('全局变量', () => {

	const 系统信息 = ref<any>({})
	return {
		系统信息
	}
})