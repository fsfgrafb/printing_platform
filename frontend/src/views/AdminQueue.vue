<script setup>
import { onMounted, ref } from 'vue'
import { Pause, Play, RefreshCw, X } from '@lucide/vue'
import { api } from '../api'

const tasks = ref([])
const paused = ref(false)
const printer = ref({ blocked: false, blocking_reasons: [], warnings: [] })

onMounted(load)

async function load() {
  const { data } = await api.get('/admin/queue')
  tasks.value = data.tasks
  paused.value = data.paused
  printer.value = data.printer
}

async function pause() {
  await api.post('/admin/queue/pause')
  await load()
}

async function resume() {
  await api.post('/admin/queue/resume')
  await load()
}

async function cancel(task) {
  const reason = window.prompt('取消原因', '')
  await api.delete(`/admin/tasks/${task.id}`, { data: { reason } })
  await load()
}
</script>

<template>
  <section class="page">
    <header class="page-header">
      <div>
        <h1>队列管理</h1>
        <p>{{ paused ? '已暂停' : '运行中' }}</p>
      </div>
      <div class="button-row">
        <button class="ghost-button" type="button" @click="load">
          <RefreshCw :size="18" />
          <span>刷新</span>
        </button>
        <button v-if="!paused" class="ghost-button" type="button" @click="pause">
          <Pause :size="18" />
          <span>暂停</span>
        </button>
        <button v-else class="primary-button" type="button" @click="resume">
          <Play :size="18" />
          <span>继续</span>
        </button>
      </div>
    </header>

    <div v-if="printer.blocked" class="alert-banner danger">
      {{ printer.blocking_reasons.join('；') }}（打印机恢复后自动继续，管理员暂停状态不受影响）
    </div>
    <div v-if="printer.warnings?.length" class="alert-banner warning">{{ printer.warnings.join('；') }}</div>

    <div class="task-grid">
      <article v-for="task in tasks" :key="task.id" class="task-card">
        <div class="task-top">
          <strong>#{{ task.id }}</strong>
          <span class="status-pill" :class="task.status">{{ task.status }}</span>
        </div>
        <h2>{{ task.file_name }}</h2>
        <p>{{ task.owner_name }} · {{ task.page_count }} 页 · {{ task.odd_even }}</p>
        <p v-if="task.status_detail">{{ task.status_detail }}</p>
        <button
          v-if="['queued', 'printing', 'pending_review'].includes(task.status)"
          class="icon-button danger-button"
          type="button"
          title="取消任务"
          @click="cancel(task)"
        >
          <X :size="18" />
        </button>
      </article>
    </div>
  </section>
</template>
