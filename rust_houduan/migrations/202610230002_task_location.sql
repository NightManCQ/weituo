ALTER TABLE "任务"
    ADD COLUMN "经纬" point,
    ADD COLUMN "国" text,
    ADD COLUMN "省" text,
    ADD COLUMN "市" text,
    ADD COLUMN "县" text,
    ADD COLUMN "镇" text,
    ADD COLUMN "村" text,
    ADD COLUMN "地址" text;

CREATE INDEX "任务_经纬_空间_idx"
    ON "任务" USING GIST ("经纬")
    WHERE "经纬" IS NOT NULL;

COMMENT ON COLUMN "任务"."经纬" IS
    '高德 GCJ-02 坐标，X 为经度、Y 为纬度；GiST 索引用于范围预筛选，距离应使用地理距离计算精筛。';
