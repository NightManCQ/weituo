// 委托分类与头像兜底：委托广场列表、委托详情页共用同一套规则。
// 两处各写一份的话，很快就会出现「列表标着视觉设计、点进去变成技能开发」的前后不一致。
//
// 后端任务表没有分类字段，也没有头像字段，只能按「标题 + 描述」关键字加权推断分类，
// 按发布者编号做稳定散列取头像。

export type 分类规则 = { 名称 : string; 标签 : string; 色 : string; 词 : string[] }

// 关键词越长权重越高，命中总分最高的那一类胜出；一条都不命中归到「综合」
export const 分类规则表 : 分类规则[] = [
	{
		名称: '技能开发', 标签: '💻 技能开发', 色: 'zi',
		词: ['代码', '开发', '程序', '接口', 'api', '前端', '后端', '数据库', '部署',
			'bug', '报错', '脚本', '爬虫', '算法', '测试', '联调', '重构', '服务'],
	},
	{
		名称: '跑腿代办', 标签: '🏃 跑腿代办', 色: 'lv',
		词: ['代取', '取件', '送货', '取货', '帮买', '跑腿', '快递', '排队', '搬运',
			'打印', '代办', '代收', '送到', '带一份'],
	},
	{
		名称: '视觉设计', 标签: '🎨 视觉设计', 色: 'zi',
		词: ['设计', 'logo', '海报', '插画', '表情包', 'ui', '配色', '视觉', '封面',
			'包装', '排版', '原型', 'banner', '图标'],
	},
	{
		名称: '学习互助', 标签: '📚 学习互助', 色: 'zi',
		词: ['雅思', '托福', '英语', '辅导', '对练', '陪练', '复习', '论文', '考试',
			'课程', '答疑', '学习', '口语', '家教', '作业'],
	},
	{
		名称: '资料调研', 标签: '🔍 资料调研', 色: 'zi',
		词: ['调研', '整理', '资料', '问卷', '采集', '搜集', '汇总', '报告', '翻译',
			'竞品', '访谈', '数据'],
	},
]

export function 推断分类(标题 : string, 描述 : string) : 分类规则 | null {
	const 文本 = (标题 + ' ' + 描述).toLowerCase()
	let 最佳 : 分类规则 | null = null
	let 最佳分 = 0
	for (let i = 0; i < 分类规则表.length; i++) {
		const 规则 = 分类规则表[i]
		let 分 = 0
		for (let j = 0; j < 规则.词.length; j++) {
			const 词 = 规则.词[j]
			if (文本.includes(词)) 分 += 词.length
		}
		if (分 > 最佳分) {
			最佳分 = 分
			最佳 = 规则
		}
	}
	return 最佳
}

export function 推断分类名(标题 : string, 描述 : string) : string {
	const 规则 = 推断分类(标题, 描述)
	return 规则 == null ? '综合' : 规则.名称
}

export const 头像池 : string[] = [
	'/static/wei_tuo/avatar_alex.jpg',
	'/static/wei_tuo/avatar_maya.jpg',
	'/static/wei_tuo/avatar_momo.jpg',
	'/static/hui_hua/avatar_design.jpg',
	'/static/hui_hua/avatar_frontend.jpg',
	'/static/hui_hua/avatar_zhoumin.jpg',
]

// 按编号做稳定散列：同一个用户在任何页面、任何时候都是同一张头像
export function 取头像(编号 : string) : string {
	let 和 = 0
	for (let i = 0; i < 编号.length; i++) 和 += 编号.charCodeAt(i)
	return 头像池[和 % 头像池.length]
}
