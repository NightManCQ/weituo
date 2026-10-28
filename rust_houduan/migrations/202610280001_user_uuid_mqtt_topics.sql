WITH 个人通知收件人 AS (
    SELECT 发件箱."事件编号",
           (发件箱."事件内容"->>'通知用户编号')::uuid AS "用户编号"
    FROM "事务发件箱" AS 发件箱
    WHERE 发件箱."事件类型" IN ('friend.request_created', 'friend.message_created')

    UNION ALL

    SELECT 发件箱."事件编号", 邀请."受邀用户编号"
    FROM "事务发件箱" AS 发件箱
    JOIN "群_邀请" AS 邀请
      ON 邀请."邀请编号"::text = 发件箱."事件内容"->'数据'->>'邀请编号'
    WHERE 发件箱."事件类型" = 'group.invitation_created'

    UNION ALL

    SELECT 发件箱."事件编号", 任务."发布者编号"
    FROM "事务发件箱" AS 发件箱
    JOIN "任务" AS 任务
      ON 任务."任务编号"::text = 发件箱."事件内容"->>'任务编号'
    WHERE 发件箱."事件类型" LIKE 'task.%'
)
UPDATE "事务发件箱" AS 发件箱
SET "主题" = 'u/' || 收件人."用户编号"::text
FROM 个人通知收件人 AS 收件人
WHERE 发件箱."事件编号" = 收件人."事件编号"
  AND 发件箱."主题" LIKE 'u/%';

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM "事务发件箱" AS 发件箱
        WHERE 发件箱."主题" LIKE 'u/%'
          AND (
              (
                  发件箱."事件类型" IN ('friend.request_created', 'friend.message_created')
                  AND 发件箱."主题" <> (
                      'u/' || COALESCE(发件箱."事件内容"->>'通知用户编号', '')
                  )
              )
              OR (
                  发件箱."事件类型" = 'group.invitation_created'
                  AND 发件箱."主题" <> COALESCE((
                      SELECT 'u/' || 邀请."受邀用户编号"::text
                      FROM "群_邀请" AS 邀请
                      WHERE 邀请."邀请编号"::text =
                          (发件箱."事件内容"->'数据'->>'邀请编号')
                  ), '')
              )
              OR (
                  发件箱."事件类型" LIKE 'task.%'
                  AND 发件箱."主题" <> COALESCE((
                      SELECT 'u/' || 任务."发布者编号"::text
                      FROM "任务" AS 任务
                      WHERE 任务."任务编号"::text =
                          (发件箱."事件内容"->>'任务编号')
                  ), '')
              )
              OR 发件箱."事件类型" NOT IN (
                  'friend.request_created',
                  'friend.message_created',
                  'group.invitation_created'
              ) AND 发件箱."事件类型" NOT LIKE 'task.%'
          )
    ) THEN
        RAISE EXCEPTION 'Legacy user MQTT outbox topics could not be resolved to user UUIDs';
    END IF;
END $$;
