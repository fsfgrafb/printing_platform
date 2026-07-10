<script setup>
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
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
import { api } from './api'
import { refreshSession, session, signOut } from './session'

const route = useRoute()
const router = useRouter()
const isLogin = computed(() => route.path === '/login')
const isAdmin = computed(() => session.user?.role === 'admin')
const mustChangePassword = computed(() => Boolean(session.user?.must_change_password))
const reviewCount = ref(0)
let reviewTimer = null

const nav = computed(() => [
  { to: '/submit', label: '提交打印', icon: UploadCloud },
  { to: '/queue', label: '打印队列', icon: ClipboardList },
  { to: '/settings', label: '个人设置', icon: Settings }
])

const adminNav = computed(() => [
  { to: '/admin/users', label: '用户管理', icon: Users },
  { to: '/admin/review', label: '审核中心', icon: ShieldCheck, badge: reviewCount.value },
  { to: '/admin/stats', label: '统计中心', icon: ClipboardList },
  { to: '/admin/settings', label: '系统设置', icon: SlidersHorizontal }
])

onMounted(async () => {
  await refreshSession()
  if (!session.user && !isLogin.value) router.replace('/login')
  updateReviewPolling()
})

onUnmounted(stopReviewPolling)

watch([isAdmin, mustChangePassword], updateReviewPolling)

async function logout() {
  stopReviewPolling()
  await signOut()
  reviewCount.value = 0
  router.replace('/login')
}

function updateReviewPolling() {
  stopReviewPolling()
  if (!isAdmin.value || mustChangePassword.value) {
    reviewCount.value = 0
    return
  }
  loadReviewCount()
  reviewTimer = window.setInterval(loadReviewCount, 3000)
}

function stopReviewPolling() {
  if (reviewTimer) {
    window.clearInterval(reviewTimer)
    reviewTimer = null
  }
}

async function loadReviewCount() {
  try {
    const { data } = await api.get('/admin/review')
    reviewCount.value = Array.isArray(data) ? data.length : 0
  } catch {
    reviewCount.value = 0
  }
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
          <span v-if="item.badge" class="nav-badge">{{ item.badge }}</span>
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
