export type 高德经纬点 = {
  getLng: () => number
  getLat: () => number
  lng?: number
  lat?: number
}

export type 高德坐标 = [number, number] | 高德经纬点

export type 高德逆地理结果 = {
  info?: string
  regeocode?: {
    formattedAddress?: string
    addressComponent?: {
      country?: string
      province?: string
      city?: string | string[]
      district?: string
      township?: string
      neighborhood?: string | { name?: string }
      neighborhoodName?: string
    }
  }
  geocodes?: Array<{ location: 高德坐标 }>
}

export type 高德地图实例 = {
  add: (覆盖物: 高德标记 | 高德圆形标记) => void
  addControl: (控件: unknown) => void
  destroy: () => void
  on: (事件: 'click', 回调: (事件数据: { lnglat: 高德经纬点 }) => void) => void
  setCenter: (坐标: [number, number]) => void
  setZoom: (级别: number) => void
}

export type 高德标记 = {
  setMap: (地图: 高德地图实例 | null) => void
  setPosition: (坐标: [number, number]) => void
}

export type 高德圆形标记 = {
  setMap: (地图: 高德地图实例 | null) => void
}

export type 高德地理编码器 = {
  getAddress: (
    坐标: [number, number],
    回调: (状态: string, 结果: 高德逆地理结果) => void,
  ) => void
  getLocation: (
    地址: string,
    回调: (状态: string, 结果: 高德逆地理结果) => void,
  ) => void
}

export type 高德地图SDK = {
  Map: new (
    容器: HTMLElement,
    选项: { center: [number, number]; zoom: number; viewMode: '2D' },
  ) => 高德地图实例
  Marker: new (选项: { position: [number, number]; title: string }) => 高德标记
  Geocoder: new (选项: { radius: number; extensions: 'all' }) => 高德地理编码器
  CircleMarker: new (选项: {
    center: [number, number]
    radius: number
    strokeColor: string
    strokeWeight: number
    strokeOpacity: number
    fillColor: string
    fillOpacity: number
  }) => 高德圆形标记
  Scale: new () => unknown
  ToolBar: new () => unknown
}

declare global {
  interface Window {
    AMap?: 高德地图SDK
    _AMapSecurityConfig?: {
      serviceHost: string
    }
  }
}

let 地图加载任务: Promise<高德地图SDK> | null = null

export function 加载高德地图(): Promise<高德地图SDK> {
  const 地图Key = import.meta.env.VITE_AMAP_KEY?.trim()
  if (!地图Key) {
    return Promise.reject(new Error('请在前端环境变量 VITE_AMAP_KEY 中配置高德地图 Key'))
  }
  if (window.AMap) return Promise.resolve(window.AMap)
  if (地图加载任务) return 地图加载任务

  const 服务地址 = (
    import.meta.env.VITE_AMAP_SERVICE_HOST?.trim() || window.location.origin
  ).replace(/\/+$/, '')
  window._AMapSecurityConfig = {
    serviceHost: `${服务地址}/_AMapService`,
  }

  地图加载任务 = new Promise<高德地图SDK>((resolve, reject) => {
    const 脚本 = document.createElement('script')
    脚本.src = `https://webapi.amap.com/maps?v=2.0&key=${encodeURIComponent(地图Key)}&plugin=AMap.Geocoder,AMap.Scale,AMap.ToolBar`
    脚本.async = true
    脚本.dataset.amapJsapi = 'true'
    脚本.onload = () => {
      if (window.AMap) {
        resolve(window.AMap)
      } else {
        reject(new Error('高德地图加载完成，但 JS API 未初始化'))
      }
    }
    脚本.onerror = () => {
      脚本.remove()
      reject(new Error('高德地图加载失败，请检查网络和 VITE_AMAP_KEY 配置'))
    }
    document.head.append(脚本)
  }).catch((原因: unknown) => {
    地图加载任务 = null
    throw 原因
  })

  return 地图加载任务
}
