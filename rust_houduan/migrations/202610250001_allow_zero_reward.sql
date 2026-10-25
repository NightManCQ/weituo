ALTER TABLE "任务"
    DROP CONSTRAINT IF EXISTS "任务_报酬积分_check",
    ADD CONSTRAINT "任务_报酬积分_check"
        CHECK ("报酬积分" IS NULL OR "报酬积分" BETWEEN 0 AND 1000000000);

ALTER TABLE "委托报酬"
    DROP CONSTRAINT IF EXISTS "委托报酬_积分数量_check",
    ADD CONSTRAINT "委托报酬_积分数量_check"
        CHECK ("积分数量" BETWEEN 0 AND 1000000000);

COMMENT ON COLUMN "任务"."报酬积分" IS
    '委托方发布时设置的整数积分报酬；0 表示无报酬，历史委托保持 NULL。';

COMMENT ON COLUMN "委托报酬"."积分数量" IS
    '委托报酬托管与结算数量；0 表示无积分转账的委托。';
