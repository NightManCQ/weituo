DELETE FROM "事务发件箱"
WHERE "事件类型" = 'chat.message_created'
  AND "主题" LIKE 'renwu/%/messages';

DROP TABLE IF EXISTS "聊天消息";
