ALTER TABLE "任务"
    ADD COLUMN IF NOT EXISTS "接单模式" text NOT NULL DEFAULT '竞标',
    ADD COLUMN IF NOT EXISTS "投标截止时间" timestamptz,
    ADD COLUMN IF NOT EXISTS "中标投标编号" uuid;

ALTER TABLE "任务"
    DROP CONSTRAINT IF EXISTS "任务_任务状态_check";
ALTER TABLE "任务"
    ADD CONSTRAINT "任务_任务状态_check" CHECK (
        "任务状态" IN (
            '待接单', '投标中', '已接单', '进行中',
            '待验收', '已完成', '已取消', '已过期'
        )
    );

CREATE TABLE IF NOT EXISTS "任务_投标" (
    "投标编号" uuid PRIMARY KEY,
    "任务编号" uuid NOT NULL REFERENCES "任务" ("任务编号") ON DELETE CASCADE,
    "投标用户编号" uuid NOT NULL REFERENCES "用户" ("用户编号"),
    "投标状态" text NOT NULL DEFAULT '投标中'
        CHECK ("投标状态" IN ('投标中', '已撤回', '已中标', '已落选')),
    "报价积分" bigint NOT NULL DEFAULT 0 CHECK ("报价积分" >= 0),
    "投标说明" text NOT NULL DEFAULT ''
        CHECK (char_length("投标说明") <= 4000),
    "预计完成时间" timestamptz,
    "创建时间" timestamptz NOT NULL DEFAULT now(),
    "更新时间" timestamptz NOT NULL DEFAULT now(),
    "撤回时间" timestamptz,
    "选定时间" timestamptz,
    "版本号" integer NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS "任务_投标_任务用户唯一_idx"
    ON "任务_投标" ("任务编号", "投标用户编号");

CREATE UNIQUE INDEX IF NOT EXISTS "任务_投标_任务中标唯一_idx"
    ON "任务_投标" ("任务编号")
    WHERE "投标状态" = '已中标';

CREATE INDEX IF NOT EXISTS "任务_投标_任务状态_idx"
    ON "任务_投标" ("任务编号", "投标状态", "创建时间" DESC);

CREATE INDEX IF NOT EXISTS "任务_投标_投标用户_idx"
    ON "任务_投标" ("投标用户编号", "更新时间" DESC);

CREATE TABLE IF NOT EXISTS "任务_投标事件" (
    "事件编号" uuid PRIMARY KEY,
    "任务编号" uuid NOT NULL REFERENCES "任务" ("任务编号") ON DELETE CASCADE,
    "投标编号" uuid REFERENCES "任务_投标" ("投标编号") ON DELETE SET NULL,
    "操作者编号" uuid REFERENCES "用户" ("用户编号"),
    "事件类型" text NOT NULL,
    "事件内容" jsonb NOT NULL DEFAULT '{}'::jsonb,
    "发生时间" timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS "任务_投标事件_任务时间_idx"
    ON "任务_投标事件" ("任务编号", "发生时间" DESC);

CREATE INDEX IF NOT EXISTS "任务_投标截止时间_idx"
    ON "任务" ("投标截止时间")
    WHERE "任务状态" = '投标中';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = '任务_中标投标_fk'
    ) THEN
        ALTER TABLE "任务"
            ADD CONSTRAINT "任务_中标投标_fk"
            FOREIGN KEY ("中标投标编号")
            REFERENCES "任务_投标" ("投标编号");
    END IF;
END $$;
