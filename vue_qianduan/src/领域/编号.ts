export function 生成唯一编号(): string {
  const 随机字节 = new Uint8Array(16)
  globalThis.crypto.getRandomValues(随机字节)
  随机字节[6] = (随机字节[6] & 0x0f) | 0x40
  随机字节[8] = (随机字节[8] & 0x3f) | 0x80

  const 十六进制 = Array.from(随机字节, (字节) => 字节.toString(16).padStart(2, '0')).join('')
  return [
    十六进制.slice(0, 8),
    十六进制.slice(8, 12),
    十六进制.slice(12, 16),
    十六进制.slice(16, 20),
    十六进制.slice(20),
  ].join('-')
}
