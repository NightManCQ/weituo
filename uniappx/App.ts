import { 初始化会话列表和实时消息 } from "./pages/xiaoxi/会话列表";
import { 刷新联系人列表 } from "./pages/lianxiren/联系人列表";
import { 准备数据库 } from "./common/sqllite";

export async function 应用初始化() {
  初始化会话列表和实时消息();
  void 刷新联系人列表();

  // 本地数据库是增强能力：失败不能拖垮启动（原来在 App.uvue 里也是 try/catch）
  try {
    const 数据库结果 = await 准备数据库();
    console.log("本地数据库已就绪", 数据库结果);
  } catch (原因) {
    console.error("本地数据库初始化失败", 原因);
  }
}
