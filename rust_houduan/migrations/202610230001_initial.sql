CREATE TABLE IF NOT EXISTS "用户" (
    "用户编号" uuid PRIMARY KEY,
    "登录名" text NOT NULL UNIQUE,
    "密码摘要" text NOT NULL,
    "昵称" text NOT NULL,
    "账号状态" text NOT NULL DEFAULT '正常'
        CHECK ("账号状态" IN ('正常', '禁用')),
    "创建时间" timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS "角色" (
    "角色编号" uuid PRIMARY KEY,
    "角色名称" text NOT NULL UNIQUE
);
INSERT INTO "角色" ("角色编号", "角色名称") VALUES
    ('10000000-0000-4000-8000-000000000001', '普通用户'),
    ('10000000-0000-4000-8000-000000000002', '运营员'),
    ('10000000-0000-4000-8000-000000000003', '管理员')
ON CONFLICT ("角色名称") DO NOTHING;

CREATE TABLE IF NOT EXISTS "用户角色" (
    "用户编号" uuid NOT NULL REFERENCES "用户" ("用户编号") ON DELETE CASCADE,
    "角色编号" uuid NOT NULL REFERENCES "角色" ("角色编号") ON DELETE CASCADE,
    "授予时间" timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("用户编号", "角色编号")
);

CREATE TABLE IF NOT EXISTS "任务" (
    "任务编号" uuid PRIMARY KEY,
    "发布者编号" uuid NOT NULL REFERENCES "用户" ("用户编号"),
    "接单者编号" uuid REFERENCES "用户" ("用户编号"),
    "标题" text NOT NULL CHECK (char_length("标题") BETWEEN 1 AND 120),
    "描述" text NOT NULL CHECK (char_length("描述") <= 10000),
    "任务状态" text NOT NULL CHECK (
        "任务状态" IN ('待接单', '已接单', '进行中', '已完成', '已取消', '已过期')
    ),
    "截止时间" timestamptz,
    "接单时间" timestamptz,
    "版本号" integer NOT NULL DEFAULT 0,
    "创建时间" timestamptz NOT NULL DEFAULT now(),
    "更新时间" timestamptz NOT NULL DEFAULT now(),
    CHECK ("接单者编号" IS NULL OR "接单者编号" <> "发布者编号")
);
CREATE INDEX IF NOT EXISTS "任务_待接单_创建时间_idx"
    ON "任务" ("创建时间" DESC) WHERE "任务状态" = '待接单';
CREATE INDEX IF NOT EXISTS "任务_发布者_创建时间_idx"
    ON "任务" ("发布者编号", "创建时间" DESC);
CREATE INDEX IF NOT EXISTS "任务_接单者_创建时间_idx"
    ON "任务" ("接单者编号", "创建时间" DESC) WHERE "接单者编号" IS NOT NULL;

CREATE TABLE IF NOT EXISTS "任务事件" (
    "事件编号" uuid PRIMARY KEY,
    "任务编号" uuid NOT NULL REFERENCES "任务" ("任务编号"),
    "操作者编号" uuid REFERENCES "用户" ("用户编号"),
    "事件类型" text NOT NULL,
    "原状态" text,
    "新状态" text NOT NULL,
    "发生时间" timestamptz NOT NULL DEFAULT now(),
    "请求幂等键" text
);
CREATE UNIQUE INDEX IF NOT EXISTS "任务事件_操作者_幂等键_idx"
    ON "任务事件" ("操作者编号", "请求幂等键")
    WHERE "请求幂等键" IS NOT NULL;

CREATE TABLE IF NOT EXISTS "聊天消息" (
    "消息编号" uuid PRIMARY KEY,
    "任务编号" uuid NOT NULL REFERENCES "任务" ("任务编号"),
    "发送者编号" uuid NOT NULL REFERENCES "用户" ("用户编号"),
    "客户端消息编号" text NOT NULL,
    "消息正文" text NOT NULL CHECK (char_length("消息正文") BETWEEN 1 AND 4000),
    "创建时间" timestamptz NOT NULL DEFAULT now(),
    UNIQUE ("任务编号", "发送者编号", "客户端消息编号")
);
CREATE INDEX IF NOT EXISTS "聊天消息_任务_时间_idx"
    ON "聊天消息" ("任务编号", "创建时间", "消息编号");

CREATE TABLE IF NOT EXISTS "事务发件箱" (
    "事件编号" uuid PRIMARY KEY,
    "事件类型" text NOT NULL,
    "主题" text NOT NULL,
    "事件内容" jsonb NOT NULL,
    "投递状态" text NOT NULL DEFAULT '待投递'
        CHECK ("投递状态" IN ('待投递', '已投递')),
    "重试次数" integer NOT NULL DEFAULT 0,
    "下次尝试时间" timestamptz NOT NULL DEFAULT now(),
    "创建时间" timestamptz NOT NULL DEFAULT now(),
    "投递时间" timestamptz
);
CREATE INDEX IF NOT EXISTS "事务发件箱_待投递_idx"
    ON "事务发件箱" ("下次尝试时间", "创建时间")
    WHERE "投递状态" = '待投递';

CREATE TABLE IF NOT EXISTS "操作审计" (
    "审计编号" uuid PRIMARY KEY,
    "操作者编号" uuid NOT NULL REFERENCES "用户" ("用户编号"),
    "目标用户编号" uuid REFERENCES "用户" ("用户编号"),
    "操作类型" text NOT NULL,
    "操作内容" jsonb NOT NULL DEFAULT '{}'::jsonb,
    "发生时间" timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS "操作审计_时间_idx"
    ON "操作审计" ("发生时间" DESC);
