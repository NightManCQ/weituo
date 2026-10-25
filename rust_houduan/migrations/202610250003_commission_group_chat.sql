ALTER TABLE "群_群组"
    ADD COLUMN "关联任务编号" uuid REFERENCES "任务" ("任务编号");

CREATE UNIQUE INDEX "群_群组_关联任务唯一_idx"
    ON "群_群组" ("关联任务编号")
    WHERE "关联任务编号" IS NOT NULL;

WITH 已接单任务 AS (
    SELECT t."任务编号", t."发布者编号", t."接单者编号", t."标题"
    FROM "任务" t
    WHERE t."接单者编号" IS NOT NULL
),
新建委托群 AS (
    INSERT INTO "群_群组"
        ("群组编号", "群组名称", "群主编号", "关联任务编号")
    SELECT gen_random_uuid(), left(t."标题", 80), t."发布者编号", t."任务编号"
    FROM 已接单任务 t
    RETURNING "群组编号", "群主编号", "关联任务编号"
)
INSERT INTO "群_成员"
    ("成员编号", "群组编号", "用户编号", "成员角色", "可见起始序号")
SELECT gen_random_uuid(), g."群组编号", g."群主编号", '群主', 1
FROM 新建委托群 g
UNION ALL
SELECT gen_random_uuid(), g."群组编号", t."接单者编号", '成员', 1
FROM 新建委托群 g
JOIN 已接单任务 t ON t."任务编号" = g."关联任务编号";

INSERT INTO "群_消息"
    ("消息编号", "群组编号", "群内序号", "发送者编号",
     "客户端消息编号", "消息正文", "创建时间")
SELECT legacy."消息编号", g."群组编号",
       row_number() OVER (
           PARTITION BY legacy."任务编号"
           ORDER BY legacy."创建时间", legacy."消息编号"
       ),
       legacy."发送者编号", legacy."客户端消息编号",
       legacy."消息正文", legacy."创建时间"
FROM "聊天消息" legacy
JOIN "群_群组" g ON g."关联任务编号" = legacy."任务编号";

UPDATE "群_群组" g
SET "最后消息序号" = COALESCE(messages."最后序号", 0)
FROM (
    SELECT "群组编号", max("群内序号") AS "最后序号"
    FROM "群_消息"
    GROUP BY "群组编号"
) messages
WHERE g."群组编号" = messages."群组编号";

UPDATE "群_成员" member
SET "可见起始序号" = group_chat."最后消息序号" + 1
FROM "群_群组" group_chat
WHERE member."群组编号" = group_chat."群组编号"
  AND group_chat."关联任务编号" IS NOT NULL
  AND member."用户编号" <> group_chat."群主编号"
  AND member."成员状态" = '有效';

COMMENT ON COLUMN "群_群组"."关联任务编号" IS
    '非空时表示该群组专用于对应委托，由发布者和当前接单者组成。';
