import { createRouter, createWebHistory } from 'vue-router'
import LoginView from './views/LoginView.vue'
import SubmitPrint from './views/SubmitPrint.vue'
import QueueView from './views/QueueView.vue'
import SettingsView from './views/SettingsView.vue'
import AdminUsers from './views/AdminUsers.vue'
import ReviewCenter from './views/ReviewCenter.vue'
import SystemSettings from './views/SystemSettings.vue'
import { refreshSession, session } from './session'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/login', component: LoginView },
    { path: '/', redirect: '/submit' },
    { path: '/submit', component: SubmitPrint, meta: { requiresAuth: true } },
    { path: '/queue', component: QueueView, meta: { requiresAuth: true } },
    { path: '/history', redirect: '/queue' },
    { path: '/settings', component: SettingsView, meta: { requiresAuth: true } },
    { path: '/admin/users', component: AdminUsers, meta: { requiresAuth: true, admin: true } },
    { path: '/admin/review', component: ReviewCenter, meta: { requiresAuth: true, admin: true } },
    { path: '/admin/stats', redirect: '/admin/users' },
    { path: '/admin/queue', redirect: '/queue' },
    { path: '/admin/history', redirect: '/queue' },
    { path: '/admin/settings', component: SystemSettings, meta: { requiresAuth: true, admin: true } }
  ]
})

router.beforeEach(async to => {
  if (!session.ready) {
    await refreshSession()
  }

  if (to.path === '/login' && session.user) {
    return session.user.must_change_password ? '/settings' : '/submit'
  }

  if (to.meta.requiresAuth && !session.user) {
    return '/login'
  }

  if (session.user?.must_change_password && to.path !== '/settings') {
    return '/settings'
  }

  if (to.meta.admin && session.user?.role !== 'admin') {
    return '/submit'
  }
})

export default router
