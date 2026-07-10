<script setup>
import { onMounted, onUnmounted, ref } from 'vue'
import { RefreshCw, X } from '@lucide/vue'
import { api, unwrapError } from '../api'

const tasks = ref([])
const paused = ref(false)
const printer = ref({ blocked: false, blocking_reasons: [], warnings: [] })
const error = ref('')
let timer = null
let socket = null
let reconnectTimer = null
let stopped = false

onMounted(() => {
  stopped = false
  load()
  timer = window.setInterval(load, 5000)
  connectSocket()
})

function connectSocket() {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  socket = new WebSocket(`${protocol}//${window.location.host}/api/ws/queue`)
  socket.onmessage = () => load()
  socket.onerror = () => socket?.close()
  socket.onclose = () => {
    socket = null
    if (!stopped) reconnectTimer = window.setTimeout(connectSocket, 2000)
  }
}

onUnmounted(() => {
  stopped = true
  if (timer) window.clearInterval(timer)
  if (reconnectTimer) window.clearTimeout(reconnectTimer)
  if (socket) socket.close()
})

async function load() {
  try {
    const { data } = await api.get('/queue')
    tasks.value = data.tasks
    paused.value = data.paused
    printer.value = data.printer
    error.value = ''
  } catch (err) {
    error.value = unwrapError(err)
  }
}

async function cancel(task) {
  await api.delete(`/print/tasks/${task.id}`)
  await load()
}
</script>

<template>
  <section class="page">
    <header class="page-header">
      <div>
        <h1>打印队列</h1>
        <p>{{ paused ? '队列已暂停' : '按提交时间自动打印' }}</p>
      </div>
      <button class="ghost-button" type="button" @click="load">
        <RefreshCw :size="18" />
        <span>刷新</span>
      </button>
    </header>

    <div v-if="printer.blocked" class="alert-banner danger">
      打印机暂时阻塞：{{ printer.blocking_reasons.join('；') }}。故障清除后队列会自动继续。
    </div>
    <div v-else-if="printer.warnings?.length" class="alert-banner warning">
      {{ printer.warnings.join('；') }}
    </div>

    <div class="task-grid">
      <article v-for="task in tasks" :key="task.id" class="task-card" :class="{ mine: task.mine }">
        <div class="task-top">
          <strong>#{{ task.id }}</strong>
          <span class="status-pill" :class="task.status">{{ task.status }}</span>
        </div>
        <h2>{{ task.file_name || `任务 ${task.id}` }}</h2>
        <p>{{ task.page_count }} 页 · {{ task.odd_even }} · {{ task.owner_name || '其他用户' }}</p>
        <p v-if="task.status_detail && task.mine">{{ task.status_detail }}</p>
        <button
          v-if="task.mine && ['queued', 'pending_review'].includes(task.status)"
          class="icon-button danger-button"
          type="button"
          title="取消任务"
          @click="cancel(task)"
        >
          <X :size="18" />
        </button>
      </article>
    </div>

    <p v-if="!tasks.length" class="empty-state">当前没有待处理任务</p>
    <p v-if="error" class="error-text">{{ error }}</p>
  </section>
</template>
