<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import type { 任务位置 } from '../领域/类型'
import {
  加载高德地图,
  type 高德地图实例,
  type 高德地图SDK,
  type 高德地理编码器,
  type 高德标记,
  type 高德圆形标记,
  type 高德逆地理结果,
} from '../地图/高德地图'

const props = defineProps<{
  初始位置: 任务位置 | null
}>()

const emit = defineEmits<{
  选择位置: [位置: 任务位置]
  取消: []
}>()

const 地图容器 = ref<HTMLElement | null>(null)
const 正在加载地图 = ref(true)
const 正在查询地址 = ref(false)
const 正在定位 = ref(false)
const 错误信息 = ref('')
const 地址提示 = ref('')
const 搜索文字 = ref('')
const 选中位置 = ref<任务位置 | null>(
  props.初始位置 ? { ...props.初始位置 } : null,
)
const 详细地址 = computed({
  get: () => 选中位置.value?.地址 ?? '',
  set: (地址: string) => {
    if (选中位置.value) {
      选中位置.value = { ...选中位置.value, 地址: 地址 || null }
    }
  },
})

let 地图SDK: 高德地图SDK | null = null
let 地图实例: 高德地图实例 | null = null
let 地理编码器: 高德地理编码器 | null = null
let 位置标记: 高德标记 | null = null
let 当前位置标记: 高德圆形标记 | null = null
let 逆地理请求序号 = 0
let 正向地理请求序号 = 0
let 定位请求序号 = 0

function 提取坐标(坐标: [number, number] | { getLng: () => number; getLat: () => number }) {
  return Array.isArray(坐标)
    ? 坐标
    : [坐标.getLng(), 坐标.getLat()] as [number, number]
}

function 文本或空值(值: string | undefined): string | null {
  const 清理后 = 值?.trim()
  return 清理后 ? 清理后 : null
}

function 首个文本(值: string | string[] | undefined): string | null {
  return 文本或空值(Array.isArray(值) ? 值.find((项) => 项.trim()) : 值)
}

function 村名(值: string | { name?: string } | undefined): string | null {
  return 文本或空值(typeof 值 === 'string' ? 值 : 值?.name)
}

function 更新标记(经纬: [number, number]) {
  if (!地图SDK || !地图实例) return
  if (!位置标记) {
    位置标记 = new 地图SDK.Marker({
      position: 经纬,
      title: '委托位置',
    })
    地图实例.add(位置标记)
  } else {
    位置标记.setPosition(经纬)
  }
}

function 定位当前位置() {
  const 地图 = 地图实例
  const SDK = 地图SDK
  if (!地图 || !SDK) {
    地址提示.value = '地图尚未准备好，请稍后重试'
    return
  }
  if (!navigator.geolocation) {
    地址提示.value = '当前网站环境不支持定位，请使用 HTTPS 或 localhost 后重试'
    return
  }

  const 请求序号 = ++定位请求序号
  正在定位.value = true
  地址提示.value = ''

  const 请求设备定位 = () => {
    navigator.geolocation.getCurrentPosition(
      ({ coords }) => {
        if (请求序号 !== 定位请求序号 || 地图实例 !== 地图) return
        const 经纬: [number, number] = [coords.longitude, coords.latitude]
        地图.setCenter(经纬)
        地图.setZoom(16)
        当前位置标记?.setMap(null)
        当前位置标记 = new SDK.CircleMarker({
          center: 经纬,
          radius: 8,
          strokeColor: '#ffffff',
          strokeWeight: 3,
          strokeOpacity: 1,
          fillColor: '#1677ff',
          fillOpacity: 0.95,
        })
        地图.add(当前位置标记)
        地址提示.value = ''
        正在定位.value = false
      },
      (原因) => {
        if (请求序号 !== 定位请求序号 || 地图实例 !== 地图) return
        if (原因.code === GeolocationPositionError.PERMISSION_DENIED) {
          地址提示.value = '网站定位权限未开启，请在浏览器网站设置中允许定位后再试'
        } else if (原因.code === GeolocationPositionError.POSITION_UNAVAILABLE) {
          地址提示.value = '暂时无法确定当前位置，请检查设备定位服务'
        } else {
          地址提示.value = '获取当前位置超时，请检查定位服务后重试'
        }
        正在定位.value = false
      },
      { enableHighAccuracy: true, maximumAge: 30_000, timeout: 10_000 },
    )
  }

  if (!navigator.permissions) {
    请求设备定位()
    return
  }

  void navigator.permissions.query({ name: 'geolocation' }).then((权限) => {
    if (请求序号 !== 定位请求序号 || 地图实例 !== 地图) return
    if (权限.state === 'denied') {
      地址提示.value = '定位权限当前为拒绝状态，正在重新请求；若浏览器未弹出授权，请先在网站设置中允许定位'
    } else if (权限.state === 'prompt') {
      地址提示.value = '请在浏览器弹窗中允许网站使用定位'
    }
    请求设备定位()
  }).catch(() => {
    if (请求序号 !== 定位请求序号 || 地图实例 !== 地图) return
    地址提示.value = '无法读取定位权限状态，正在直接请求定位'
    请求设备定位()
  })
}

function 读取逆地理信息(结果: 高德逆地理结果) {
  const 逆地理 = 结果.regeocode
  const 行政区 = 逆地理?.addressComponent
  if (!逆地理 || !行政区 || !选中位置.value) return false

  选中位置.value = {
    ...选中位置.value,
    国: 文本或空值(行政区.country),
    省: 文本或空值(行政区.province),
    市: 首个文本(行政区.city),
    县: 文本或空值(行政区.district),
    镇: 文本或空值(行政区.township),
    村: 文本或空值(行政区.neighborhoodName) ?? 村名(行政区.neighborhood),
    地址: 文本或空值(逆地理.formattedAddress) ?? 选中位置.value.地址,
  }
  return true
}

async function 选择坐标(经纬: [number, number]) {
  正向地理请求序号 += 1
  更新标记(经纬)
  地图实例?.setCenter(经纬)
  选中位置.value = {
    经度: 经纬[0],
    纬度: 经纬[1],
    国: null,
    省: null,
    市: null,
    县: null,
    镇: null,
    村: null,
    地址: null,
  }
  地址提示.value = ''
  const 请求序号 = ++逆地理请求序号
  if (!地理编码器) {
    地址提示.value = '地图服务未就绪，地址信息暂不可查询'
    return
  }

  正在查询地址.value = true
  await new Promise<void>((完成) => {
    地理编码器?.getAddress(经纬, (状态, 结果) => {
      if (请求序号 !== 逆地理请求序号) {
        完成()
        return
      }
      正在查询地址.value = false
      if (状态 !== 'complete' || 结果.info !== 'OK' || !读取逆地理信息(结果)) {
        地址提示.value = '未能自动识别该点的行政区，请检查网络或手动补充详细地址'
      }
      完成()
    })
  })
}

async function 初始化地图() {
  正在加载地图.value = true
  错误信息.value = ''
  try {
    地图SDK = await 加载高德地图()
    await nextTick()
    if (!地图容器.value) return
    地图实例 = new 地图SDK.Map(地图容器.value, {
      center: props.初始位置
        ? [props.初始位置.经度, props.初始位置.纬度]
        : [116.397428, 39.90923],
      zoom: props.初始位置 ? 15 : 11,
      viewMode: '2D',
    })
    地图实例.addControl(new 地图SDK.Scale())
    地图实例.addControl(new 地图SDK.ToolBar())
    定位当前位置()
    地理编码器 = new 地图SDK.Geocoder({ radius: 1000, extensions: 'all' })
    地图实例.on('click', ({ lnglat }) => {
      void 选择坐标([lnglat.getLng(), lnglat.getLat()])
    })
    if (props.初始位置) {
      更新标记([props.初始位置.经度, props.初始位置.纬度])
    }
  } catch (原因) {
    错误信息.value = 原因 instanceof Error ? 原因.message : '高德地图初始化失败'
  } finally {
    正在加载地图.value = false
  }
}

function 搜索地址() {
  const 地址 = 搜索文字.value.trim()
  if (!地址 || !地理编码器) return
  const 请求序号 = ++正向地理请求序号
  错误信息.value = ''
  正在查询地址.value = true
  地理编码器.getLocation(地址, (状态, 结果) => {
    if (请求序号 !== 正向地理请求序号) return
    正在查询地址.value = false
    const 命中 = 结果.geocodes?.[0]
    if (状态 !== 'complete' || 结果.info !== 'OK' || !命中) {
      地址提示.value = '没有找到该地址，请换个关键词或直接点击地图选点'
      return
    }
    地址提示.value = ''
    void 选择坐标(提取坐标(命中.location))
  })
}

function 确认位置() {
  if (选中位置.value) emit('选择位置', { ...选中位置.value })
}

onMounted(() => void 初始化地图())

onBeforeUnmount(() => {
  逆地理请求序号 += 1
  正向地理请求序号 += 1
  定位请求序号 += 1
  位置标记?.setMap(null)
  当前位置标记?.setMap(null)
  当前位置标记 = null
  地图实例?.destroy()
  地图实例 = null
})
</script>

<template>
  <section class="map-picker-page">
    <div class="map-picker-heading">
      <div>
        <div class="section-eyebrow">委托位置</div>
        <h1>选择委托地址</h1>
        <p class="muted">搜索地点，或直接点击地图选点；详细地址可在下方补充。</p>
      </div>
      <button class="button button-quiet" type="button" @click="emit('取消')">取消</button>
    </div>

    <form class="map-search" @submit.prevent="搜索地址">
      <input
        v-model.trim="搜索文字"
        aria-label="搜索地点"
        placeholder="搜索地点或道路名称"
      />
      <button class="button button-primary" :disabled="正在查询地址 || !搜索文字.trim()">
        {{ 正在查询地址 ? '查询中…' : '搜索' }}
      </button>
    </form>

    <div class="map-canvas-wrap">
      <div ref="地图容器" class="map-canvas" aria-label="高德地图选点区域"></div>
      <div v-if="正在加载地图" class="map-overlay">
        <span class="map-loading-dot"></span>正在加载地图…
      </div>
      <div v-else-if="错误信息" class="map-overlay map-load-error" role="alert">
        <p>{{ 错误信息 }}</p>
        <button class="button button-outline" type="button" @click="初始化地图">重试</button>
      </div>
      <div class="map-hint">点击地图上的位置设置委托地点</div>
    </div>

    <section class="map-location-card" aria-live="polite">
      <template v-if="选中位置">
        <div class="map-location-summary">
          <strong>已选位置</strong>
          <span>
            {{ [选中位置.国, 选中位置.省, 选中位置.市, 选中位置.县, 选中位置.镇, 选中位置.村]
              .filter(Boolean).join(' · ') || '正在识别行政区' }}
          </span>
          <small>{{ 选中位置.经度.toFixed(6) }}, {{ 选中位置.纬度.toFixed(6) }}</small>
        </div>
        <label class="map-address-field">
          详细地址（可编辑）
          <input v-model="详细地址" maxlength="1000" placeholder="楼栋、单元、门牌号等" />
        </label>
      </template>
      <p v-else class="map-location-empty">选择地图位置后，这里会显示识别到的地址。</p>
      <p v-if="正在查询地址" class="map-address-status">正在识别地址…</p>
      <p v-else-if="地址提示" class="map-address-status map-address-warning" role="status">
        {{ 地址提示 }}
      </p>
      <div class="map-location-actions">
        <button class="button button-quiet" type="button" @click="emit('取消')">返回发布页</button>
        <button
          class="button button-outline"
          type="button"
          :disabled="正在加载地图 || 正在定位"
          @click="定位当前位置"
        >{{ 正在定位 ? '正在定位…' : '当前定位' }}</button>
        <button
          class="button button-primary"
          type="button"
          :disabled="正在加载地图 || 正在查询地址 || !选中位置"
          @click="确认位置"
        >使用此地址</button>
      </div>
    </section>
  </section>
</template>
