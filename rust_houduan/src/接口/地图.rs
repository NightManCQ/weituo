use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::共享::*;

// 逆地理编码查询参数：前端拖动地图后把中心点传进来
#[derive(Deserialize)]
pub(crate) struct 逆向地理编码查询 {
    #[serde(rename = "longitude", alias = "lng")]
    经度: f64,
    #[serde(rename = "latitude", alias = "lat")]
    纬度: f64,
}

// 只返回前端回填地址需要的字段，避免多余数据
#[derive(Serialize)]
pub(crate) struct 坐标地址视图 {
    经度: f64,
    纬度: f64,
    国: Option<String>,
    省: Option<String>,
    市: Option<String>,
    县: Option<String>,
    镇: Option<String>,
    村: Option<String>,
    地址: String,
}

/// 逆地理编码：把地图中心点解析成可读地址。
/// 由后端统一调用高德 Web 服务接口，前端（App 与 Web）复用同一份结果，
/// 好处是端上不需要保存高德 Web 服务密钥，也绕开了浏览器跨域限制。
pub(crate) async fn 逆向地理编码(
    State(状态): State<应用状态>,
    查询: Query<逆向地理编码查询>,
) -> Result<Json<坐标地址视图>, 应用错误> {
    if !查询.经度.is_finite()
        || !查询.纬度.is_finite()
        || !(-180.0..=180.0).contains(&查询.经度)
        || !(-90.0..=90.0).contains(&查询.纬度)
    {
        return Err(应用错误::请求错误("经纬度超出有效范围".into()));
    }
    let 密钥 = 状态
        .高德服务密钥
        .as_deref()
        .ok_or_else(|| 应用错误::服务不可用("地址解析服务尚未配置".into()))?;

    let 坐标 = format!("{:.6},{:.6}", 查询.经度, 查询.纬度);
    let 响应 = 状态
        .高德客户端
        .get("https://restapi.amap.com/v3/geocode/regeo")
        .query(&[
            ("key", 密钥),
            ("location", 坐标.as_str()),
            ("extensions", "base"),
            ("radius", "200"),
        ])
        .send()
        .await
        .map_err(|_| 应用错误::内部错误(anyhow::anyhow!("高德逆地理请求失败")))?;

    // 上游状态码不直接透传：高德业务失败也会返回 200，需要看 status 字段
    if !响应.status().is_success() {
        return Err(应用错误::服务不可用(format!(
            "地址解析服务返回异常（{}）",
            响应.status().as_u16()
        )));
    }
    let 响应文本 = 响应
        .text()
        .await
        .map_err(|_| 应用错误::内部错误(anyhow::anyhow!("读取高德逆地理响应失败")))?;
    let 内容: Value = serde_json::from_str(&响应文本)
        .map_err(|_| 应用错误::内部错误(anyhow::anyhow!("高德逆地理响应解析失败")))?;
    if 内容["status"].as_str() != Some("1") {
        let 原因 = 内容["info"].as_str().unwrap_or("未知错误");
        return Err(应用错误::服务不可用(
            format!("地址解析失败：{原因}"),
        ));
    }

    let 逆地理 = &内容["regeocode"];
    let 组件 = &逆地理["addressComponent"];
    let 地址 = 逆地理["formatted_address"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_owned();
    if 地址.is_empty() {
        return Err(应用错误::未找到(
            "该位置暂无可读地址，请换个位置试试".into(),
        ));
    }

    Ok(Json(坐标地址视图 {
        经度: 查询.经度,
        纬度: 查询.纬度,
        国: 读取文本(&组件["country"]),
        省: 读取文本(&组件["province"]),
        市: 读取文本(&组件["city"]),
        县: 读取文本(&组件["district"]),
        镇: 读取文本(&组件["township"]),
        // 高德逆地理不返回村级信息，保留字段方便前端统一处理
        村: None,
        地址,
    }))
}

// 周边地址查询参数：地图中心点 + 期望条数 + 可选关键词
#[derive(Deserialize)]
pub(crate) struct 周边地址查询 {
    #[serde(rename = "longitude", alias = "lng")]
    经度: f64,
    #[serde(rename = "latitude", alias = "lat")]
    纬度: f64,
    /// 期望返回条数，不传按 10 条处理
    #[serde(default, rename = "count")]
    数量: Option<u32>,
    /// 关键词：传了就按关键词在中心点周边大范围搜索，不传则只列附近地点
    #[serde(default, rename = "keywords", alias = "keyword")]
    关键词: Option<String>,
}

// 一个可选地址：名称给用户看，经度/纬度用于回填委托位置
#[derive(Serialize)]
pub(crate) struct 周边地址项 {
    编号: String,
    名称: String,
    地址: Option<String>,
    省: Option<String>,
    市: Option<String>,
    县: Option<String>,
    经度: f64,
    纬度: f64,
    距离: f64,
}

#[derive(Serialize)]
pub(crate) struct 周边地址视图 {
    经度: f64,
    纬度: f64,
    /// 实际使用的搜索半径（米），前端据此提示"附近 x 米内"
    半径: u32,
    /// 实际生效的关键词，前端据此区分"附近地址"与"关键词搜索结果"
    关键词: Option<String>,
    地址列表: Vec<周边地址项>,
}

/// 周边地址：列出地图中心点附近（或与关键词匹配）的 POI，按距离由近到远排序。
/// 不带关键词时，密集城区 50 米内往往只有两三个地点，为了不出现空列表，
/// 数量不足时按 50→100→200→500→1000 米逐级放大半径，并把实际半径回传前端。
/// 带关键词时按较大范围搜一次：keywords 不为空时高德的 sortrule 不生效，所以本地按距离重排。
pub(crate) async fn 周边地址(
    State(状态): State<应用状态>,
    查询: Query<周边地址查询>,
) -> Result<Json<周边地址视图>, 应用错误> {
    if !查询.经度.is_finite()
        || !查询.纬度.is_finite()
        || !(-180.0..=180.0).contains(&查询.经度)
        || !(-90.0..=90.0).contains(&查询.纬度)
    {
        return Err(应用错误::请求错误("经纬度超出有效范围".into()));
    }
    let 密钥 = 状态
        .高德服务密钥
        .as_deref()
        .ok_or_else(|| 应用错误::服务不可用("地址解析服务尚未配置".into()))?;

    let 坐标 = format!("{:.6},{:.6}", 查询.经度, 查询.纬度);
    let 中心 = (查询.经度, 查询.纬度);
    let 目标数量 = 查询.数量.unwrap_or(10).clamp(1, 20) as usize;
    let 关键词 = 查询
        .关键词
        .as_deref()
        .map(str::trim)
        .filter(|值| !值.is_empty())
        .map(str::to_owned);

    // 关键词搜索：附近 50 米几乎不可能命中，直接按 50 公里范围搜一次。
    // 高德单次最多返回 25 条，多取一些再本地按距离截断，命中率更高。
    if let Some(词) = 关键词.as_deref() {
        let mut 结果 =
            请求周边地点(&状态, 密钥, &坐标, 中心, Some(词), 50_000, 目标数量.max(25)).await?;
        结果.sort_by(比较距离);
        结果.truncate(目标数量);
        return Ok(Json(周边地址视图 {
            经度: 查询.经度,
            纬度: 查询.纬度,
            半径: 50_000,
            关键词,
            地址列表: 结果,
        }));
    }

    let 搜索半径们 = [50u32, 100, 200, 500, 1000];

    let mut 结果: Vec<周边地址项> = Vec::new();
    let mut 已收录编号: Vec<String> = Vec::new();
    let mut 实际半径 = 0u32;
    for 半径 in 搜索半径们 {
        实际半径 = 半径;
        let 本次列表 = 请求周边地点(&状态, 密钥, &坐标, 中心, None, 半径, 目标数量).await?;
        for 地点 in 本次列表 {
            // 半径放大后同一地点会重复出现，按 POI 编号去重
            if 已收录编号.iter().any(|编号| 编号 == &地点.编号) {
                continue;
            }
            已收录编号.push(地点.编号.clone());
            结果.push(地点);
        }
        if 结果.len() >= 目标数量 {
            break;
        }
    }
    // sortrule=distance 已让高德按距离排序，这里再排一次保证多轮合并后依然有序
    结果.sort_by(比较距离);
    结果.truncate(目标数量);

    Ok(Json(周边地址视图 {
        经度: 查询.经度,
        纬度: 查询.纬度,
        半径: 实际半径,
        关键词: None,
        地址列表: 结果,
    }))
}

/// 距离升序；高德偶发返回 0，排在后面比排前面安全
fn 比较距离(甲: &周边地址项, 乙: &周边地址项) -> std::cmp::Ordering {
    甲.距离
        .partial_cmp(&乙.距离)
        .unwrap_or(std::cmp::Ordering::Equal)
}

/// 单次调用高德周边搜索，只取前端要用的字段
async fn 请求周边地点(
    状态: &应用状态,
    密钥: &str,
    坐标: &str,
    中心: (f64, f64),
    关键词: Option<&str>,
    半径: u32,
    数量: usize,
) -> Result<Vec<周边地址项>, 应用错误> {
    let 半径文本 = 半径.to_string();
    // 高德单页最多 25 条，超出会被静默截断
    let 数量文本 = 数量.min(25).to_string();
    let mut 请求 = 状态
        .高德客户端
        .get("https://restapi.amap.com/v3/place/around")
        .query(&[
            ("key", 密钥),
            ("location", 坐标),
            ("radius", 半径文本.as_str()),
            ("offset", 数量文本.as_str()),
            ("page", "1"),
            ("sortrule", "distance"),
            ("extensions", "base"),
        ]);
    if let Some(词) = 关键词 {
        请求 = 请求.query(&[("keywords", 词)]);
    }
    let 响应 = 请求
        .send()
        .await
        .map_err(|_| 应用错误::内部错误(anyhow::anyhow!("高德周边搜索请求失败")))?;

    if !响应.status().is_success() {
        return Err(应用错误::服务不可用(format!(
            "周边搜索服务返回异常（{}）",
            响应.status().as_u16()
        )));
    }
    let 响应文本 = 响应
        .text()
        .await
        .map_err(|_| 应用错误::内部错误(anyhow::anyhow!("读取高德周边搜索响应失败")))?;
    // 上游状态码不可靠（高德业务失败也返回 200），只能看 status 字段
    let 内容: Value = serde_json::from_str(&响应文本)
        .map_err(|_| 应用错误::内部错误(anyhow::anyhow!("高德周边搜索响应解析失败")))?;
    if 内容["status"].as_str() != Some("1") {
        let 原因 = 内容["info"].as_str().unwrap_or("未知错误");
        return Err(应用错误::服务不可用(
            format!("周边搜索失败：{原因}"),
        ));
    }

    let mut 列表: Vec<周边地址项> = Vec::new();
    let Some(地点数组) = 内容["pois"].as_array() else {
        return Ok(列表);
    };
    for 地点 in 地点数组 {
        // 名称和坐标缺一不可，缺了就无法回填委托位置
        let Some(编号) = 读取文本(&地点["id"]) else {
            continue;
        };
        let Some(名称) = 读取文本(&地点["name"]) else {
            continue;
        };
        let Some((经度, 纬度)) = 读取坐标(&地点["location"]) else {
            continue;
        };
        列表.push(周边地址项 {
            编号,
            名称,
            地址: 读取文本(&地点["address"]),
            省: 读取文本(&地点["pname"]),
            市: 读取文本(&地点["cityname"]),
            县: 读取文本(&地点["adname"]),
            经度,
            纬度,
            // 关键词搜索时高德可能不返回 distance，退回本地球面距离，保证能排序
            距离: 读取距离(&地点["distance"])
                .unwrap_or_else(|| 球面距离(中心.0, 中心.1, 经度, 纬度)),
        });
    }
    Ok(列表)
}

/// 两点间球面距离（米）：高德 distance 缺失时的兜底
fn 球面距离(经度甲: f64, 纬度甲: f64, 经度乙: f64, 纬度乙: f64) -> f64 {
    const 地球半径: f64 = 6_371_000.0;
    let 纬度差 = (纬度乙 - 纬度甲).to_radians();
    let 经度差 = (经度乙 - 经度甲).to_radians();
    let 纬度甲弧度 = 纬度甲.to_radians();
    let 纬度乙弧度 = 纬度乙.to_radians();
    let 半弦平方 = (纬度差 / 2.0).sin().powi(2)
        + 纬度甲弧度.cos() * 纬度乙弧度.cos() * (经度差 / 2.0).sin().powi(2);
    2.0 * 地球半径 * 半弦平方.sqrt().asin()
}

/// 高德把距离返回成字符串（如 "18"），个别接口是数字，两种都兼容
fn 读取距离(值: &Value) -> Option<f64> {
    if let Some(文本) = 值.as_str() {
        return 文本.trim().parse::<f64>().ok();
    }
    值.as_f64()
}

/// 高德坐标是 "经度,纬度" 字符串，解析失败说明该 POI 不可用
fn 读取坐标(值: &Value) -> Option<(f64, f64)> {
    let 文本 = 值.as_str()?.trim();
    let (经度文本, 纬度文本) = 文本.split_once(',')?;
    Some((
        经度文本.trim().parse::<f64>().ok()?,
        纬度文本.trim().parse::<f64>().ok()?,
    ))
}

/// 高德在直辖市等场景会把 city 返回成空数组，这里统一按"没有值"处理
fn 读取文本(值: &Value) -> Option<String> {
    let 文本 = 值.as_str()?.trim();
    if 文本.is_empty() {
        return None;
    }
    Some(文本.to_owned())
}

#[cfg(test)]
mod 逆地理编码测试 {
    use super::{球面距离, 读取坐标, 读取文本, 读取距离};
    use serde_json::json;

    #[test]
    fn 空数组与空字符串都按没有值处理() {
        assert_eq!(读取文本(&json!([])), None);
        assert_eq!(读取文本(&json!("")), None);
        assert_eq!(读取文本(&json!("   ")), None);
    }

    #[test]
    fn 正常文本会去掉首尾空格() {
        assert_eq!(读取文本(&json!(" 广东省 ")), Some("广东省".to_owned()));
    }

    #[test]
    fn 距离字符串与数字都能解析() {
        assert_eq!(读取距离(&json!("18")), Some(18.0));
        assert_eq!(读取距离(&json!(26)), Some(26.0));
        assert_eq!(读取距离(&json!("")), None);
        assert_eq!(读取距离(&json!([])), None);
    }

    #[test]
    fn 坐标字符串解析成经度纬度() {
        assert_eq!(
            读取坐标(&json!("113.934530,22.540411")),
            Some((113.93453, 22.540411))
        );
        assert_eq!(读取坐标(&json!("113.9")), None);
        assert_eq!(读取坐标(&json!(["113.9,22.5"])), None);
    }

    #[test]
    fn 球面距离按米计算() {
        // 深圳南山：经度相差 0.001°（纬度 22.54 附近约 103 米）
        let 距离 = 球面距离(113.9344, 22.5403, 113.9354, 22.5403);
        assert!((100.0..107.0).contains(&距离), "实际距离 {距离}");
        // 同一点距离为 0
        assert!(球面距离(113.9344, 22.5403, 113.9344, 22.5403) < 1e-6);
    }
}
