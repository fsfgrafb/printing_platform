<script setup>
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { ChevronLeft, ChevronRight, Eye, Pause, Play, RefreshCw, Search, X } from '@lucide/vue'
import { api, unwrapError } from '../api'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import { session } from '../session'

const tasks = ref([])
const page = ref(1)
const perPage = 50
const total = ref(0)
const mineOnly = ref(false)
const studentId = ref('')
const paused = ref(false)
const loaded = ref(false)
const printer = ref({ status: 'starting', queue_name: '', available: false, blocked: false, blocking_reasons: [], warnings: [] })
const previewTask = ref(null)
const pendingAction = ref(null)
const actionValue = ref('')
const actionBusy = ref(false)
const error = ref('')
let timer = null
let socket = null
let reconnectTimer = null
let stopped = false

const isAdmin = computed(() => session.user?.role === 'admin')
const printerDisplay = computed(() => {
  const raw = String(printer.value.status || '').toLowerCase()
  if (paused.value) return { label: 'Paused', text: '队列已暂停', tone: 'paused' }
  if (!printer.value.available || raw.includes('unavailable') || raw.includes('offline')) {
    return { label: 'Offline', text: '打印机不可用', tone: 'error' }
  }
  if (printer.value.blocked || raw.includes('error') || raw.includes('paper') || raw.includes('jam')) {
    return { label: 'Error', text: '打印机需要处理', tone: 'error' }
  }
  if (raw.includes('printing')) return { label: 'Printing', text: '正在打印', tone: 'printing' }
  if (raw.includes('initializing') || raw.includes('processing') || raw.includes('busy')) {
    return { label: 'Running', text: '打印机处理中', tone: 'running' }
  }
  return { label: 'Ready', text: '打印机就绪', tone: 'ready' }
})

onMounted(() => {
  stopped = false
  load()
  timer = window.setInterval(load, 5000)
  connectSocket()
  window.addEventListener('keydown', closeOnEscape)
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
  window.removeEventListener('keydown', closeOnEscape)
})

async function load() {
  try {
    const { data } = await api.get('/queue', {
      params: {
        page: page.value,
        per_page: perPage,
        mine_only: mineOnly.value,
        student_id: isAdmin.value && !mineOnly.value ? studentId.value || undefined : undefined
      }
    })
    tasks.value = data.tasks
    total.value = data.total
    paused.value = data.paused
    printer.value = data.printer
    loaded.value = true
    error.value = ''
  } catch (err) {
    error.value = unwrapError(err)
    loaded.value = true
  }
}

async function toggleMine() {
  page.value = 1
  await load()
}

async function searchStudent() {
  page.value = 1
  await load()
}

async function move(delta) {
  page.value = Math.max(1, page.value + delta)
  await load()
}

function requestAction(type, task = null) {
  actionValue.value = ''
  pendingAction.value = { type, task }
}

async function runAction() {
  const action = pendingAction.value
  if (!action) return
  actionBusy.value = true
  error.value = ''
  try {
    if (action.type === 'cancel') {
      if (isAdmin.value) {
        await api.delete(`/admin/tasks/${action.task.id}`, { data: { reason: actionValue.value || null } })
      } else {
        await api.delete(`/print/tasks/${action.task.id}`)
      }
    }
    pendingAction.value = null
    await load()
  } catch (err) {
    error.value = unwrapError(err)
  } finally {
    actionBusy.value = false
  }
}

async function pauseQueue() {
  await api.post('/admin/queue/pause')
  await load()
}

async function resumeQueue() {
  await api.post('/admin/queue/resume')
  await load()
}

async function acknowledgeToner() {
  await api.post('/admin/printer/ack-toner')
  await load()
}

function closeOnEscape(event) {
  if (event.key !== 'Escape') return
  previewTask.value = null
  if (!actionBusy.value) pendingAction.value = null
}

function statusLabel(status) {
  return {
    queued: '排队中', printing: '打印中', pending_review: '待审核', done: '已结束', cancelled: '已取消'
  }[status] || status
}

function rangeLabel(range) {
  return { all: '全部页', odd: '奇数页', even: '偶数页' }[range] || range
}
</script>

<template>
  <section class="page queue-page">
    <header class="page-header">
      <div>
        <h1>打印队列</h1>
        <p>当前记录与近一年的打印历史</p>
      </div>
      <div class="button-row queue-toolbar">
        <div v-if="isAdmin && !mineOnly" class="queue-search">
          <input v-model.trim="studentId" placeholder="按学号筛选" @keyup.enter="searchStudent" />
          <button class="icon-button" type="button" title="查询" @click="searchStudent"><Search :size="18" /></button>
        </div>
        <label class="check-control">
          <input v-model="mineOnly" type="checkbox" @change="toggleMine" />
          <span>只看我的打印</span>
        </label>
        <button class="ghost-button" type="button" @click="load">
          <RefreshCw :size="18" />
          <span>刷新</span>
        </button>
        <template v-if="isAdmin">
          <button v-if="!paused" class="ghost-button" type="button" @click="pauseQueue">
            <Pause :size="18" /><span>暂停队列</span>
          </button>
          <button v-else class="primary-button" type="button" @click="resumeQueue">
            <Play :size="18" /><span>继续队列</span>
          </button>
        </template>
      </div>
    </header>

    <section v-if="loaded" class="printer-card">
      <div class="printer-identity">
        <span class="printer-state-pill" :class="printerDisplay.tone">{{ printerDisplay.label }}</span>
        <div>
          <strong>{{ printerDisplay.text }}</strong>
          <span>{{ printer.queue_name || '正在读取打印机名称' }}</span>
        </div>
      </div>
      <span class="printer-raw-status">驱动状态：{{ printer.status || '-' }}</span>
    </section>

    <div v-else class="printer-card">
      <div class="printer-identity">
        <span class="printer-state-pill paused">Loading</span>
        <div>
          <strong>正在读取打印机状态</strong>
          <span>请稍候</span>
        </div>
      </div>
      <span class="printer-raw-status">驱动状态：-</span>
    </div>

    <div v-if="loaded && printer.blocked" class="alert-banner danger">
      打印机暂时阻塞：{{ printer.blocking_reasons.join('；') }}。故障清除后会自动继续。
    </div>
    <div v-if="loaded && printer.warnings?.length && (!printer.toner_alert_acknowledged || !isAdmin)" class="alert-banner warning">
      <span>{{ printer.warnings.join('；') }}</span>
      <button v-if="isAdmin" class="ghost-button" type="button" @click="acknowledgeToner">确认提示</button>
    </div>

    <div class="task-list">
      <article v-for="task in tasks" :key="task.id" class="task-card queue-task-card" :class="{ mine: task.mine }">
        <div class="task-main">
          <div class="task-top">
            <div class="task-number">
              <strong>#{{ task.id }}</strong>
              <span v-if="task.mine" class="mine-badge">我的打印</span>
            </div>
            <span class="status-pill" :class="task.status">{{ statusLabel(task.status) }}</span>
          </div>
          <h2>{{ task.file_name || `打印任务 ${task.id}` }}</h2>
          <p>
            {{ task.page_count }} 页 · {{ rangeLabel(task.odd_even) }}
            <template v-if="task.owner_name"> · {{ task.owner_name }}</template>
          </p>
          <p class="task-time">提交：{{ task.submitted_at }}<template v-if="task.completed_at"> · 结束：{{ task.completed_at }}</template></p>
          <p v-if="task.status_detail || task.review_reason">{{ task.status_detail || task.review_reason }}</p>
          <p v-if="task.submitted_ip && isAdmin">提交 IP：{{ task.submitted_ip }}</p>
        </div>
        <div class="queue-task-actions">
          <button v-if="task.preview_url" class="ghost-button" type="button" @click="previewTask = task">
            <Eye :size="18" /><span>预览最终 PDF</span>
          </button>
          <button
            v-if="(isAdmin && ['queued', 'printing', 'pending_review'].includes(task.status)) || (task.mine && ['queued', 'pending_review'].includes(task.status))"
            class="ghost-button danger-text"
            type="button"
            @click="requestAction('cancel', task)"
          >
            <X :size="18" /><span>取消任务</span>
          </button>
        </div>
      </article>
    </div>

    <p v-if="!tasks.length" class="empty-state">没有符合条件的打印记录</p>
    <footer v-if="total > perPage" class="pagination-bar">
      <span>共 {{ total }} 条</span>
      <div class="button-row">
        <button class="icon-button" type="button" title="上一页" :disabled="page <= 1" @click="move(-1)"><ChevronLeft :size="18" /></button>
        <span>第 {{ page }} 页</span>
        <button class="icon-button" type="button" title="下一页" :disabled="page * perPage >= total" @click="move(1)"><ChevronRight :size="18" /></button>
      </div>
    </footer>
    <p v-if="error" class="error-text">{{ error }}</p>

    <div v-if="previewTask" class="preview-modal" role="dialog" aria-modal="true" @click.self="previewTask = null">
      <section class="preview-dialog">
        <header>
          <strong>{{ previewTask.file_name }}</strong>
          <button class="icon-button" type="button" title="关闭预览" @click="previewTask = null"><X :size="18" /></button>
        </header>
        <iframe :src="`${previewTask.preview_url}#zoom=100`" title="最终打印 PDF 预览"></iframe>
      </section>
    </div>

    <ConfirmDialog
      v-if="pendingAction"
      title="取消打印任务"
      :message="`任务 #${pendingAction.task.id} · ${pendingAction.task.file_name || '未公开文件名'}`"
      confirm-text="确认取消"
      :danger="true"
      :input-label="isAdmin ? '取消原因（可选）' : ''"
      :input-value="actionValue"
      :busy="actionBusy"
      @update:input-value="actionValue = $event"
      @cancel="pendingAction = null"
      @confirm="runAction"
    />
  </section>
</template>
