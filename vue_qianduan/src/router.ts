import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router'
import ChatPage from './页面/聊天页.vue'
import ContactsPage from './页面/联系人页.vue'
import CreateGroupPage from './页面/创建群聊页.vue'
import CommissionDetailPage from './页面/委托详情页.vue'
import FriendSearchPage from './页面/好友搜索添加页.vue'
import MessagesPage from './页面/消息页.vue'
import ProfilePage from './页面/个人中心页.vue'
import LocationPickerPage from './页面/位置选择页.vue'
import PublishTaskPage from './页面/发布需求页.vue'
import TaskPage from './页面/任务页.vue'
import TaskSquarePage from './页面/需求广场页.vue'

const routes: RouteRecordRaw[] = [
  { path: '/', redirect: { name: 'tasks' } },
  { path: '/tasks', name: 'tasks', component: TaskPage },
  { path: '/square', name: 'square', component: TaskSquarePage },
  { path: '/publish', name: 'publish', component: PublishTaskPage },
  { path: '/groups', name: 'groups', redirect: { name: 'contacts' } },
  { path: '/groups/create', name: 'group-create', component: CreateGroupPage },
  { path: '/friends/add', name: 'friend-search', component: FriendSearchPage },
  {
    path: '/groups/:groupId/chat',
    name: 'group-chat',
    component: ChatPage,
    props: (route) => ({
      群组编号: typeof route.params.groupId === 'string' ? route.params.groupId : '',
    }),
  },
  { path: '/weituo/:id', name: 'commission-detail', component: CommissionDetailPage },
  { path: '/messages', name: 'messages', component: MessagesPage },
  {
    path: '/chat/:taskId',
    name: 'chat',
    component: ChatPage,
    props: (route) => ({
      任务编号: typeof route.params.taskId === 'string' ? route.params.taskId : '',
    }),
  },
  { path: '/friends/chat/:friendId', name: 'friend-chat', component: ChatPage },
  { path: '/contacts', name: 'contacts', component: ContactsPage },
  { path: '/select-location', name: 'location', component: LocationPickerPage },
  { path: '/profile', name: 'profile', component: ProfilePage },
  { path: '/:pathMatch(.*)*', redirect: { name: 'tasks' } },
]

export const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes,
})
