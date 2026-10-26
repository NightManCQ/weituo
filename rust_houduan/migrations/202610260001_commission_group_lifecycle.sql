DO $$
DECLARE
    状态约束名称 text;
BEGIN
    SELECT conname
    INTO 状态约束名称
    FROM pg_constraint
    WHERE conrelid = '"群_群组"'::regclass
      AND contype = 'c'
      AND pg_get_constraintdef(oid) LIKE '%群组状态%';

    IF 状态约束名称 IS NOT NULL THEN
        EXECUTE format('ALTER TABLE "群_群组" DROP CONSTRAINT %I', 状态约束名称);
    END IF;
END $$;

ALTER TABLE "群_群组"
    ADD CONSTRAINT "群_群组_群组状态_check"
        CHECK ("群组状态" IN ('正常', '已解散', '已归档')),
    ADD COLUMN "归档时间" timestamptz;

WITH 缺失关联群的任务 AS (
    SELECT t."任务编号", t."发布者编号", t."标题"
    FROM "任务" t
    WHERE NOT EXISTS (
        SELECT 1
        FROM "群_群组" g
        WHERE g."关联任务编号" = t."任务编号"
    )
)
INSERT INTO "群_群组"
    ("群组编号", "群组名称", "群主编号", "关联任务编号")
SELECT gen_random_uuid(), left("标题", 80), "发布者编号", "任务编号"
FROM 缺失关联群的任务;

INSERT INTO "群_成员"
    ("成员编号", "群组编号", "用户编号", "成员角色", "可见起始序号")
SELECT gen_random_uuid(), g."群组编号", g."群主编号", '群主', 1
FROM "群_群组" g
WHERE g."关联任务编号" IS NOT NULL
  AND NOT EXISTS (
      SELECT 1
      FROM "群_成员" m
      WHERE m."群组编号" = g."群组编号"
        AND m."用户编号" = g."群主编号"
        AND m."成员状态" = '有效'
  );

UPDATE "群_群组" g
SET "群组状态" = '已归档',
    "归档时间" = COALESCE(t."更新时间", now()),
    "更新时间" = now()
FROM "任务" t
WHERE g."关联任务编号" = t."任务编号"
  AND t."任务状态" = '已完成'
  AND g."群组状态" = '正常';

COMMENT ON COLUMN "群_群组"."归档时间" IS
    '委托群完成验收后归档的时间；归档群只允许授权成员读取历史消息。';
