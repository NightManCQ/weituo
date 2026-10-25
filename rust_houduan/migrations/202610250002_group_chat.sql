CREATE TABLE "群_群组" (
    "群组编号" uuid PRIMARY KEY,
    "群组名称" text NOT NULL CHECK (char_length(btrim("群组名称")) BETWEEN 1 AND 80),
    "群主编号" uuid NOT NULL REFERENCES "用户" ("用户编号"),
    "群组公告" text NOT NULL DEFAULT '' CHECK (char_length("群组公告") <= 2000),
    "群组状态" text NOT NULL DEFAULT '正常'
        CHECK ("群组状态" IN ('正常', '已解散')),
    "最后消息序号" bigint NOT NULL DEFAULT 0 CHECK ("最后消息序号" >= 0),
    "创建时间" timestamptz NOT NULL DEFAULT now(),
    "更新时间" timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE "群_成员" (
    "成员编号" uuid PRIMARY KEY,
    "群组编号" uuid NOT NULL REFERENCES "群_群组" ("群组编号") ON DELETE CASCADE,
    "用户编号" uuid NOT NULL REFERENCES "用户" ("用户编号"),
    "成员角色" text NOT NULL DEFAULT '成员'
        CHECK ("成员角色" IN ('群主', '管理员', '成员')),
    "成员状态" text NOT NULL DEFAULT '有效'
        CHECK ("成员状态" IN ('有效', '主动退出', '被移除')),
    "邀请人编号" uuid REFERENCES "用户" ("用户编号"),
    "加入时间" timestamptz NOT NULL DEFAULT now(),
    "离开时间" timestamptz,
    "可见起始序号" bigint NOT NULL DEFAULT 1 CHECK ("可见起始序号" >= 1),
    "已读至序号" bigint NOT NULL DEFAULT 0 CHECK ("已读至序号" >= 0),
    CHECK (
        ("成员状态" = '有效' AND "离开时间" IS NULL)
        OR ("成员状态" <> '有效' AND "离开时间" IS NOT NULL)
    )
);
CREATE UNIQUE INDEX "群_成员_有效成员唯一_idx"
    ON "群_成员" ("群组编号", "用户编号")
    WHERE "成员状态" = '有效';
CREATE INDEX "群_成员_用户群组_idx"
    ON "群_成员" ("用户编号", "群组编号")
    WHERE "成员状态" = '有效';
CREATE INDEX "群_成员_群组列表_idx"
    ON "群_成员" ("群组编号", "加入时间")
    WHERE "成员状态" = '有效';

CREATE TABLE "群_邀请" (
    "邀请编号" uuid PRIMARY KEY,
    "群组编号" uuid NOT NULL REFERENCES "群_群组" ("群组编号") ON DELETE CASCADE,
    "邀请人编号" uuid NOT NULL REFERENCES "用户" ("用户编号"),
    "受邀用户编号" uuid NOT NULL REFERENCES "用户" ("用户编号"),
    "邀请状态" text NOT NULL DEFAULT '待接受'
        CHECK ("邀请状态" IN ('待接受', '已接受', '已拒绝', '已撤销', '已过期')),
    "创建时间" timestamptz NOT NULL DEFAULT now(),
    "过期时间" timestamptz NOT NULL DEFAULT (now() + interval '7 days'),
    "处理时间" timestamptz,
    CHECK ("邀请人编号" <> "受邀用户编号")
);
CREATE UNIQUE INDEX "群_邀请_待处理唯一_idx"
    ON "群_邀请" ("群组编号", "受邀用户编号")
    WHERE "邀请状态" = '待接受';
CREATE INDEX "群_邀请_受邀用户状态_idx"
    ON "群_邀请" ("受邀用户编号", "邀请状态", "创建时间" DESC);

CREATE TABLE "群_消息" (
    "消息编号" uuid PRIMARY KEY,
    "群组编号" uuid NOT NULL REFERENCES "群_群组" ("群组编号") ON DELETE CASCADE,
    "群内序号" bigint NOT NULL CHECK ("群内序号" > 0),
    "发送者编号" uuid NOT NULL REFERENCES "用户" ("用户编号"),
    "客户端消息编号" text NOT NULL,
    "消息类型" text NOT NULL DEFAULT '文本' CHECK ("消息类型" IN ('文本')),
    "消息正文" text NOT NULL
        CHECK (char_length(btrim("消息正文")) BETWEEN 1 AND 4000),
    "创建时间" timestamptz NOT NULL DEFAULT now(),
    UNIQUE ("群组编号", "群内序号"),
    UNIQUE ("群组编号", "发送者编号", "客户端消息编号")
);
CREATE INDEX "群_消息_群组序号_idx"
    ON "群_消息" ("群组编号", "群内序号" DESC);

COMMENT ON TABLE "群_群组" IS '群聊的基础信息及单调递增的群内消息序号。';
COMMENT ON TABLE "群_成员" IS '群聊成员及其有效状态、可见消息范围和已读游标。';
COMMENT ON TABLE "群_邀请" IS '群聊成员邀请；接受邀请后才创建有效成员关系。';
COMMENT ON TABLE "群_消息" IS '与任务聊天和好友私聊分离的群聊消息。';
