<script setup>
import { onMounted, ref } from 'vue'
import { Check, X } from '@lucide/vue'
import { api, unwrapError } from '../api'
import ConfirmDialog from '../components/ConfirmDialog.vue'

const tasks = ref([])
const rejecting = ref(null)
const reason = ref('')
const loaded = ref(false)
const error = ref('')

onMounted(load)

async function load() {
  try {
    const { data } = await api.get('/admin/review')
    tasks.value = data
    error.value = ''
  } catch (err) {
    error.value = unwrapError(err)
  } finally {
    loaded.value = true
  }
}

async function approve(task) {
  await api.post(`/admin/review/${task.id}/approve`)
  await load()
}

async function reject(task) {
  rejecting.value = task
  reason.value = ''
}

async function confirmReject() {
  await api.post(`/admin/review/${rejecting.value.id}/reject`, { reason: reason.value || null })
  rejecting.value = null
  await load()
}
</script>

<template>
  <section class="page">
    <header class="page-header">
      <div>
        <h1>审核中心</h1>
        <p v-if="loaded">{{ tasks.length }} 个待审核任务</p>
      </div>
    </header>

    <p v-if="!loaded" class="loading-state">正在加载审核任务</p>
    <p v-if="error" class="error-text">{{ error }}</p>

    <div v-if="loaded" class="task-grid">
      <article v-for="task in tasks" :key="task.id" class="task-card review-card">
        <div class="task-top">
          <strong>#{{ task.id }}</strong>
          <span>{{ task.owner_name }}</span>
        </div>
        <h2>{{ task.file_name }}</h2>
        <p>{{ task.page_count }} 页 · {{ task.odd_even }}</p>
        <div class="button-row">
          <button class="primary-button" type="button" @click="approve(task)">
            <Check :size="18" />
            <span>同意</span>
          </button>
          <button class="ghost-button danger-text" type="button" @click="reject(task)">
            <X :size="18" />
            <span>拒绝</span>
          </button>
        </div>
      </article>
    </div>
    <ConfirmDialog
      v-if="rejecting"
      title="拒绝打印请求"
      :message="`任务 #${rejecting.id} · ${rejecting.file_name}`"
      confirm-text="确认拒绝"
      :danger="true"
      input-label="拒绝原因（可选）"
      :input-value="reason"
      @update:input-value="reason = $event"
      @cancel="rejecting = null"
      @confirm="confirmReject"
    />
  </section>
</template>
