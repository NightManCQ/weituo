CREATE TABLE "好友申请" (
    "申请编号" uuid PRIMARY KEY,
    "申请人编号" uuid NOT NULL REFERENCES "用户" ("用户编号") ON DELETE CASCADE,
    "目标用户编号" uuid NOT NULL REFERENCES "用户" ("用户编号") ON DELETE CASCADE,
    "申请附言" text NOT NULL DEFAULT '' CHECK (char_length("申请附言") <= 200),
    "申请状态" text NOT NULL DEFAULT '待处理'
        CHECK ("申请状态" IN ('待处理', '已同意', '已拒绝', '已撤回')),
    "创建时间" timestamptz NOT NULL DEFAULT now(),
    "处理时间" timestamptz,
    CHECK ("申请人编号" <> "目标用户编号")
);
CREATE UNIQUE INDEX "好友申请_待处理双方_idx"
    ON "好友申请" (
        LEAST("申请人编号", "目标用户编号"),
        GREATEST("申请人编号", "目标用户编号")
    )
    WHERE "申请状态" = '待处理';
CREATE INDEX "好友申请_目标状态时间_idx"
    ON "好友申请" ("目标用户编号", "申请状态", "创建时间" DESC);
CREATE INDEX "好友申请_申请人状态时间_idx"
    ON "好友申请" ("申请人编号", "申请状态", "创建时间" DESC);

CREATE TABLE "好友关系" (
    "用户甲编号" uuid NOT NULL REFERENCES "用户" ("用户编号") ON DELETE CASCADE,
    "用户乙编号" uuid NOT NULL REFERENCES "用户" ("用户编号") ON DELETE CASCADE,
    "建立时间" timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("用户甲编号", "用户乙编号"),
    CHECK ("用户甲编号" < "用户乙编号")
);
CREATE INDEX "好友关系_乙方_idx"
    ON "好友关系" ("用户乙编号", "用户甲编号");

CREATE TABLE "私聊会话" (
    "会话编号" uuid PRIMARY KEY,
    "用户甲编号" uuid NOT NULL REFERENCES "用户" ("用户编号") ON DELETE CASCADE,
    "用户乙编号" uuid NOT NULL REFERENCES "用户" ("用户编号") ON DELETE CASCADE,
    "创建时间" timestamptz NOT NULL DEFAULT now(),
    UNIQUE ("用户甲编号", "用户乙编号"),
    CHECK ("用户甲编号" < "用户乙编号")
);
CREATE INDEX "私聊会话_乙方_idx"
    ON "私聊会话" ("用户乙编号", "用户甲编号");

CREATE TABLE "私聊消息" (
    "消息编号" uuid PRIMARY KEY,
    "会话编号" uuid NOT NULL REFERENCES "私聊会话" ("会话编号") ON DELETE CASCADE,
    "发送者编号" uuid NOT NULL REFERENCES "用户" ("用户编号") ON DELETE CASCADE,
    "客户端消息编号" text NOT NULL,
    "消息正文" text NOT NULL
        CHECK (char_length(btrim("消息正文")) BETWEEN 1 AND 4000),
    "创建时间" timestamptz NOT NULL DEFAULT now(),
    UNIQUE ("会话编号", "发送者编号", "客户端消息编号")
);
CREATE INDEX "私聊消息_会话时间_idx"
    ON "私聊消息" ("会话编号", "创建时间" DESC, "消息编号" DESC);

COMMENT ON TABLE "好友申请" IS '用户之间的好友申请状态；独立于任务委托及任务聊天。';
COMMENT ON TABLE "好友关系" IS '已确认的好友关系，每对用户按 UUID 顺序仅保存一条记录。';
COMMENT ON TABLE "私聊会话" IS '好友之间的独立私聊会话，不关联任务。';
COMMENT ON TABLE "私聊消息" IS '好友私聊消息，客户端消息编号用于重试幂等。';
