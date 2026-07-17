<script setup>
import { onMounted, ref } from 'vue'
import { Check, X } from '@lucide/vue'
import { api, unwrapError } from '../api'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import { showError, showSuccess } from '../notification'

const tasks = ref([])
const rejecting = ref(null)
const reason = ref('')
const loaded = ref(false)
const busy = ref(false)

onMounted(load)

async function load() {
  loaded.value = false
  try {
    const { data } = await api.get('/admin/review')
    tasks.value = data
    loaded.value = true
    return true
  } catch (err) {
    showError(unwrapError(err), {
      title: '审核中心加载失败',
      confirmText: '重试',
      onConfirm: load
    })
    return false
  }
}

async function approve(task) {
  if (busy.value) return
  busy.value = true
  try {
    await api.post(`/admin/review/${task.id}/approve`)
    if (await load()) showSuccess(`任务 #${task.id} 已通过审核。`)
  } catch (err) {
    showError(unwrapError(err), { title: '审核失败' })
  } finally {
    busy.value = false
  }
}

async function reject(task) {
  rejecting.value = task
  reason.value = ''
}

async function confirmReject() {
  if (busy.value) return
  busy.value = true
  try {
    await api.post(`/admin/review/${rejecting.value.id}/reject`, { reason: reason.value || null })
    const taskId = rejecting.value.id
    rejecting.value = null
    if (await load()) showSuccess(`任务 #${taskId} 已拒绝。`)
  } catch (err) {
    showError(unwrapError(err), { title: '拒绝任务失败' })
  } finally {
    busy.value = false
  }
}

function rangeLabel(range) {
  return { all: '全部页', odd: '奇数页', even: '偶数页' }[range] || range
}
</script>

<template>
  <section v-if="!loaded" class="page page-loading-shell">
    <p class="loading-state">正在加载审核</p>
  </section>

  <section v-else class="page reveal-page">
    <header class="page-header">
      <div>
        <h1>审核中心</h1>
        <p>{{ tasks.length }} 个待审核任务</p>
      </div>
    </header>

    <div class="task-grid">
      <article v-for="task in tasks" :key="task.id" class="task-card review-card">
        <div class="task-top">
          <strong>#{{ task.id }}</strong>
          <span>{{ task.owner_name }}</span>
        </div>
        <h2>{{ task.file_name }}</h2>
        <p>{{ task.page_count }} 页 · {{ rangeLabel(task.odd_even) }}</p>
        <div class="button-row">
          <button class="primary-button" type="button" :disabled="busy" @click="approve(task)">
            <Check :size="18" />
            <span>通过</span>
          </button>
          <button class="ghost-button danger-text" type="button" :disabled="busy" @click="reject(task)">
            <X :size="18" />
            <span>拒绝</span>
          </button>
        </div>
      </article>
    </div>
    <p v-if="!tasks.length" class="empty-state">当前没有待审核任务</p>
    <ConfirmDialog
      v-if="rejecting"
      title="拒绝打印请求"
      :message="`任务 #${rejecting.id} · ${rejecting.file_name}`"
      confirm-text="确认拒绝"
      :danger="true"
      input-label="拒绝原因（可选）"
      :input-value="reason"
      :busy="busy"
      @update:input-value="reason = $event"
      @cancel="rejecting = null"
      @confirm="confirmReject"
    />
  </section>
</template>
