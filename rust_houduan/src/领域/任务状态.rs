use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum 任务状态错误 {
    #[error("{0}")]
    禁止操作(&'static str),
    #[error("当前状态“{0}”不允许此操作")]
    状态不允许(String),
}

pub(crate) fn 计算任务状态(
    操作: &str,
    原状态: &str,
    发布者: Uuid,
    接单者: Option<Uuid>,
    操作者: Uuid,
) -> Result<(&'static str, Option<Uuid>, &'static str), 任务状态错误> {
    match 操作 {
        "claim" if 原状态 == "待接单" && 发布者 != 操作者 => {
            Ok(("已接单", Some(操作者), "task.claimed"))
        }
        "start" if 原状态 == "已接单" && 接单者 == Some(操作者) => {
            Ok(("进行中", 接单者, "task.started"))
        }
        "complete" if 原状态 == "进行中" && 接单者 == Some(操作者) => {
            Ok(("待验收", 接单者, "task.completion_submitted"))
        }
        "accept-completion" if 原状态 == "待验收" && 发布者 == 操作者 => {
            Ok(("已完成", 接单者, "task.completed"))
        }
        "cancel" if 发布者 == 操作者 && ["待接单", "已接单"].contains(&原状态) => {
            Ok(("已取消", 接单者, "task.cancelled"))
        }
        "release" if 原状态 == "已接单" && 接单者 == Some(操作者) => {
            Ok(("待接单", None, "task.released"))
        }
        "claim" if 发布者 == 操作者 => {
            Err(任务状态错误::禁止操作("不能接自己发布的任务"))
        }
        "start" | "complete" | "release" if 接单者 != Some(操作者) => Err(
            任务状态错误::禁止操作("只有当前接单者可以执行此操作"),
        ),
        "accept-completion" if 发布者 != 操作者 => {
            Err(任务状态错误::禁止操作("只有发布者可以验收委托"))
        }
        "cancel" if 发布者 != 操作者 => {
            Err(任务状态错误::禁止操作("只有发布者可以取消任务"))
        }
        _ => Err(任务状态错误::状态不允许(原状态.to_owned())),
    }
}

#[cfg(test)]
mod 测试 {
    use super::{任务状态错误, 计算任务状态};
    use uuid::Uuid;

    #[test]
    fn 接单者唯一且不能接自己的任务() {
        let 发布者 = Uuid::new_v4();
        let 接单者 = Uuid::new_v4();
        assert_eq!(
            计算任务状态("claim", "待接单", 发布者, None, 接单者).unwrap(),
            ("已接单", Some(接单者), "task.claimed")
        );
        assert_eq!(
            计算任务状态("claim", "待接单", 发布者, None, 发布者),
            Err(任务状态错误::禁止操作("不能接自己发布的任务"))
        );
        assert!(matches!(
            计算任务状态("claim", "已接单", 发布者, Some(接单者), Uuid::new_v4()),
            Err(任务状态错误::状态不允许(_))
        ));
    }

    #[test]
    fn 只有接单者可以开始和完成任务() {
        let 发布者 = Uuid::new_v4();
        let 接单者 = Uuid::new_v4();
        let 外人 = Uuid::new_v4();
        assert_eq!(
            计算任务状态("start", "已接单", 发布者, Some(接单者), 接单者).unwrap(),
            ("进行中", Some(接单者), "task.started")
        );
        assert_eq!(
            计算任务状态("start", "已接单", 发布者, Some(接单者), 外人),
            Err(任务状态错误::禁止操作(
                "只有当前接单者可以执行此操作"
            ))
        );
        assert_eq!(
            计算任务状态("complete", "进行中", 发布者, Some(接单者), 接单者).unwrap(),
            ("待验收", Some(接单者), "task.completion_submitted")
        );
        assert_eq!(
            计算任务状态("accept-completion", "待验收", 发布者, Some(接单者), 发布者).unwrap(),
            ("已完成", Some(接单者), "task.completed")
        );
    }

    #[test]
    fn 取消和释放规则有效() {
        let 发布者 = Uuid::new_v4();
        let 接单者 = Uuid::new_v4();
        assert_eq!(
            计算任务状态("cancel", "已接单", 发布者, Some(接单者), 发布者).unwrap(),
            ("已取消", Some(接单者), "task.cancelled")
        );
        assert_eq!(
            计算任务状态("release", "已接单", 发布者, Some(接单者), 接单者).unwrap(),
            ("待接单", None, "task.released")
        );
        assert!(matches!(
            计算任务状态("cancel", "进行中", 发布者, Some(接单者), 发布者),
            Err(任务状态错误::状态不允许(_))
        ));
        assert_eq!(
            计算任务状态("accept-completion", "待验收", 发布者, Some(接单者), 接单者),
            Err(任务状态错误::禁止操作("只有发布者可以验收委托"))
        );
    }
}
