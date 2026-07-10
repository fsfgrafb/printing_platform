<script setup>
import { computed, onMounted } from 'vue'
import { RouterLink, RouterView, useRoute, useRouter } from 'vue-router'
import {
  ClipboardList,
  LogOut,
  Printer,
  Settings,
  ShieldCheck,
  SlidersHorizontal,
  UploadCloud,
  Users
} from '@lucide/vue'
import { refreshSession, session, signOut } from './session'

const route = useRoute()
const router = useRouter()
const isLogin = computed(() => route.path === '/login')
const isAdmin = computed(() => session.user?.role === 'admin')
const mustChangePassword = computed(() => Boolean(session.user?.must_change_password))

const nav = computed(() => [
  { to: '/submit', label: '提交打印', icon: UploadCloud },
  { to: '/queue', label: '打印队列', icon: ClipboardList },
  { to: '/settings', label: '个人设置', icon: Settings }
])

const adminNav = computed(() => [
  { to: '/admin/users', label: '用户管理', icon: Users },
  { to: '/admin/review', label: '审核中心', icon: ShieldCheck },
  { to: '/admin/stats', label: '统计中心', icon: ClipboardList },
  { to: '/admin/settings', label: '系统设置', icon: SlidersHorizontal }
])

onMounted(async () => {
  await refreshSession()
  if (!session.user && !isLogin.value) router.replace('/login')
})

async function logout() {
  await signOut()
  router.replace('/login')
}
</script>

<template>
  <RouterView v-if="isLogin" />

  <div v-else class="app-shell">
    <aside class="sidebar">
      <div class="brand">
        <Printer :size="24" />
        <div>
          <strong>ACM Print</strong>
          <span>{{ session.user?.student_id || '未登录' }}</span>
        </div>
      </div>

      <nav v-if="!mustChangePassword" class="nav-group">
        <RouterLink v-for="item in nav" :key="item.to" :to="item.to" class="nav-link">
          <component :is="item.icon" :size="18" />
          <span>{{ item.label }}</span>
        </RouterLink>
      </nav>

      <nav v-if="isAdmin && !mustChangePassword" class="nav-group admin-nav">
        <RouterLink v-for="item in adminNav" :key="item.to" :to="item.to" class="nav-link">
          <component :is="item.icon" :size="18" />
          <span>{{ item.label }}</span>
        </RouterLink>
      </nav>

      <button class="ghost-button logout-button" type="button" @click="logout">
        <LogOut :size="18" />
        <span>退出登录</span>
      </button>
    </aside>

    <main class="main-panel">
      <RouterView />
    </main>
  </div>
</template>
