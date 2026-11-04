import type { 任务, 任务投标 } from '../../common/leixing'
import { 创建编号, 发送请求 } from '../../common/wangluo'

// 投标截止判断：没有设置投标截止时间也算已截止，避免过期委托还能投标。
// 委托广场和委托详情两个入口共用同一套判断，避免两边阈值不一致。
export function 投标已截止(委托 : 任务) : boolean {
	if (!委托.投标截止时间) return true
	return new Date(委托.投标截止时间).getTime() <= Date.now()
}

// 是否已经投过标：列表里存在自己的、且没有被撤回的投标。
// 撤回后后端允许重新投标（ON CONFLICT 会把「已撤回」改回「投标中」），所以这里不算已投标。
export function 已投标(投标列表 : 任务投标[], 用户编号 : string) : boolean {
	return 投标列表.some((投标) => 投标.投标用户编号 == 用户编号 && 投标.投标状态 != '已撤回')
}

// 提交投标：委托广场「投标」按钮和委托详情页「投标」按钮共用，不弹表单、直接发起请求。
// 报价必须显式带上：后端 bid_points 缺省是 0，不传会变成「0 积分投标」，
// 这里沿用委托方给出的报酬积分；投标说明留空，后端允许空串。
export async function 提交委托投标(委托 : 任务) : Promise<任务投标> {
	if (!委托.任务编号) throw new Error('委托编号缺失，无法投标')
	if (投标已截止(委托)) throw new Error('委托投标已截止')
	return await 发送请求<任务投标>(`/tasks/${encodeURIComponent(委托.任务编号)}/bids`, {
		method: 'POST',
		headers: { 'Idempotency-Key': 创建编号() },
		data: {
			bid_points: 委托.报酬积分 ?? 0,
			message: '',
		} as UTSJSONObject,
	})
}

// 找出当前用户在这条委托上的投标；没投过返回 null。
// 注意：发布者查到的投标列表里包含所有人，所以这里按投标用户编号匹配，发布者永远是 null。
export function 查找我的投标(投标列表 : 任务投标[], 用户编号 : string) : 任务投标 | null {
	for (let i = 0; i < 投标列表.length; i++) {
		if (投标列表[i].投标用户编号 == 用户编号) return 投标列表[i]
	}
	return null
}

// 撤回投标：只有投标人自己在投标截止前、且委托仍在投标阶段时可用。
export async function 撤回委托投标(任务编号 : string, 投标编号 : string) : Promise<任务投标> {
	return await 发送请求<任务投标>(`/tasks/${encodeURIComponent(任务编号)}/bids/${encodeURIComponent(投标编号)}/withdraw`, {
		method: 'POST',
		headers: { 'Idempotency-Key': 创建编号() },
	})
}

// 选择承接：只有委托发布者可用；委托状态仍为投标中时可接受任意投标。
export async function 选择委托投标(任务编号 : string, 投标编号 : string) : Promise<任务> {
	return await 发送请求<任务>(`/tasks/${encodeURIComponent(任务编号)}/bids/${encodeURIComponent(投标编号)}/select`, {
		method: 'POST',
		headers: { 'Idempotency-Key': 创建编号() },
	})
}
