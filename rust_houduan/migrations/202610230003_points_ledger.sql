ALTER TABLE "任务"
    ADD COLUMN "报酬积分" bigint
        CHECK ("报酬积分" IS NULL OR "报酬积分" BETWEEN 1 AND 1000000000);

ALTER TABLE "任务"
    DROP CONSTRAINT IF EXISTS "任务_任务状态_check",
    ADD CONSTRAINT "任务_任务状态_check" CHECK (
        "任务状态" IN ('待接单', '已接单', '进行中', '待验收', '已完成', '已取消', '已过期')
    );

COMMENT ON COLUMN "任务"."报酬积分" IS
    '委托方发布时托管的整数积分；历史委托保持 NULL，不可接单直到补充报酬。';

CREATE TABLE "积分账户" (
    "账户编号" uuid PRIMARY KEY,
    "用户编号" uuid UNIQUE REFERENCES "用户" ("用户编号"),
    "账户类型" text NOT NULL CHECK ("账户类型" IN ('用户', '发行')),
    "可用积分" bigint NOT NULL DEFAULT 0,
    "冻结积分" bigint NOT NULL DEFAULT 0,
    "创建时间" timestamptz NOT NULL DEFAULT now(),
    "更新时间" timestamptz NOT NULL DEFAULT now(),
    CHECK (
        ("账户类型" = '用户' AND "用户编号" IS NOT NULL
            AND "可用积分" >= 0 AND "冻结积分" >= 0)
        OR ("账户类型" = '发行' AND "用户编号" IS NULL AND "冻结积分" = 0)
    )
);

INSERT INTO "积分账户" ("账户编号", "用户编号", "账户类型")
SELECT gen_random_uuid(), "用户编号", '用户' FROM "用户"
ON CONFLICT ("用户编号") DO NOTHING;

INSERT INTO "积分账户" ("账户编号", "账户类型")
VALUES ('20000000-0000-4000-8000-000000000001', '发行');

CREATE TABLE "积分交易" (
    "交易编号" uuid PRIMARY KEY,
    "交易类型" text NOT NULL CHECK (
        "交易类型" IN ('系统发放', '委托托管', '报酬发放', '委托退款')
    ),
    "来源账户编号" uuid NOT NULL REFERENCES "积分账户" ("账户编号"),
    "目标账户编号" uuid NOT NULL REFERENCES "积分账户" ("账户编号"),
    "来源余额类型" text NOT NULL CHECK ("来源余额类型" IN ('可用', '冻结')),
    "目标余额类型" text NOT NULL CHECK ("目标余额类型" IN ('可用', '冻结')),
    "积分数量" bigint NOT NULL CHECK ("积分数量" BETWEEN 1 AND 1000000000),
    "委托编号" uuid REFERENCES "任务" ("任务编号"),
    "操作者编号" uuid REFERENCES "用户" ("用户编号"),
    "请求幂等键" text,
    "说明" text NOT NULL DEFAULT '' CHECK (char_length("说明") <= 500),
    "创建时间" timestamptz NOT NULL DEFAULT now(),
    CHECK (
        "来源账户编号" <> "目标账户编号"
        OR "来源余额类型" <> "目标余额类型"
    )
);

CREATE UNIQUE INDEX "积分交易_操作者_幂等键_idx"
    ON "积分交易" ("操作者编号", "请求幂等键")
    WHERE "操作者编号" IS NOT NULL AND "请求幂等键" IS NOT NULL;

CREATE INDEX "积分交易_委托_时间_idx"
    ON "积分交易" ("委托编号", "创建时间" DESC)
    WHERE "委托编号" IS NOT NULL;

CREATE TABLE "积分流水" (
    "流水编号" uuid PRIMARY KEY,
    "交易编号" uuid NOT NULL REFERENCES "积分交易" ("交易编号"),
    "账户编号" uuid NOT NULL REFERENCES "积分账户" ("账户编号"),
    "可用变动" bigint NOT NULL,
    "冻结变动" bigint NOT NULL,
    "可用余额" bigint NOT NULL,
    "冻结余额" bigint NOT NULL,
    "创建时间" timestamptz NOT NULL DEFAULT now(),
    UNIQUE ("交易编号", "账户编号"),
    CHECK ("可用变动" <> 0 OR "冻结变动" <> 0)
);

CREATE INDEX "积分流水_账户_时间_idx"
    ON "积分流水" ("账户编号", "创建时间" DESC, "流水编号" DESC);

CREATE FUNCTION "拒绝修改积分账本"() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION '积分交易和流水为追加式账本，不允许修改或删除';
END;
$$;

CREATE TRIGGER "积分交易_追加式"
    BEFORE UPDATE OR DELETE ON "积分交易"
    FOR EACH ROW EXECUTE FUNCTION "拒绝修改积分账本"();

CREATE TRIGGER "积分流水_追加式"
    BEFORE UPDATE OR DELETE ON "积分流水"
    FOR EACH ROW EXECUTE FUNCTION "拒绝修改积分账本"();

CREATE TABLE "委托报酬" (
    "委托编号" uuid PRIMARY KEY REFERENCES "任务" ("任务编号"),
    "发布者编号" uuid NOT NULL REFERENCES "用户" ("用户编号"),
    "接单者编号" uuid REFERENCES "用户" ("用户编号"),
    "积分数量" bigint NOT NULL CHECK ("积分数量" BETWEEN 1 AND 1000000000),
    "结算状态" text NOT NULL CHECK (
        "结算状态" IN ('托管中', '待验收', '已发放', '已退款')
    ),
    "创建时间" timestamptz NOT NULL DEFAULT now(),
    "更新时间" timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE "积分账户" IS '用户积分可用与冻结余额；发行账户用于审计系统发放的积分。';
COMMENT ON TABLE "积分交易" IS '积分账户间的一次不可变转账记录，余额变动在同一 PostgreSQL 事务内完成。';
COMMENT ON TABLE "积分流水" IS '积分交易对各账户可用/冻结余额的不可变变动及交易后余额快照。';
COMMENT ON TABLE "委托报酬" IS '每个委托唯一的托管与结算快照；报酬在验收完成后发放。';
