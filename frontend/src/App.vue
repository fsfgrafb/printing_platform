<script setup>
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { RouterLink, RouterView, useRoute, useRouter } from 'vue-router'
import {
  ClipboardList,
  LogOut,
  Settings,
  ShieldCheck,
  SlidersHorizontal,
  UploadCloud,
  Users
} from '@lucide/vue'
import { api, unwrapError } from './api'
import ConfirmDialog from './components/ConfirmDialog.vue'
import { confirmNotice, notification, showError } from './notification'
import { session, signOut } from './session'

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
  { to: '/admin/settings', label: '系统设置', icon: SlidersHorizontal }
])

onMounted(() => {
  updateReviewPolling()
})

onUnmounted(stopReviewPolling)

watch([isAdmin, mustChangePassword], updateReviewPolling)

async function logout() {
  stopReviewPolling()
  try {
    await signOut()
    reviewCount.value = 0
    router.replace('/login')
  } catch (error) {
    showError(unwrapError(error), { title: '退出失败' })
    updateReviewPolling()
  }
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
  <div v-if="!session.ready" class="app-boot" aria-live="polite" aria-label="正在恢复登录状态">
    <img src="/favicon.svg" alt="" />
    <span>正在加载</span>
  </div>

  <template v-else>
    <RouterView v-if="isLogin" v-slot="{ Component, route: currentRoute }">
      <Transition name="page" mode="out-in">
        <div class="route-frame" :key="currentRoute.fullPath">
          <component :is="Component" />
        </div>
      </Transition>
    </RouterView>

    <div v-else-if="session.user" class="app-shell">
      <aside class="sidebar">
        <div class="brand">
          <img class="brand-icon" src="/favicon.svg" alt="" />
          <div>
            <strong>ACM Print</strong>
            <span>{{ session.user.student_id }}</span>
          </div>
        </div>

        <nav v-if="!mustChangePassword" class="nav-group">
          <RouterLink v-for="item in nav" :key="item.to" :to="item.to" class="nav-link" draggable="false" @dragstart.prevent>
            <component :is="item.icon" :size="18" />
            <span>{{ item.label }}</span>
          </RouterLink>
        </nav>

        <nav v-if="isAdmin && !mustChangePassword" class="nav-group admin-nav">
          <RouterLink v-for="item in adminNav" :key="item.to" :to="item.to" class="nav-link" draggable="false" @dragstart.prevent>
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
        <RouterView v-slot="{ Component, route: currentRoute }">
          <Transition name="page" mode="out-in">
            <div class="route-frame" :key="currentRoute.fullPath">
              <component :is="Component" />
            </div>
          </Transition>
        </RouterView>
      </main>
    </div>

    <div v-else class="app-boot" aria-live="polite">
      <img src="/favicon.svg" alt="" />
      <span>正在返回登录页</span>
    </div>

    <ConfirmDialog
      v-if="notification.visible"
      :title="notification.title"
      :message="notification.message"
      :confirm-text="notification.confirmText"
      :danger="notification.danger"
      :show-cancel="false"
      @confirm="confirmNotice"
    />
  </template>
</template>
