<script setup>
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { CircleAlert, Eye, LoaderCircle, Send, UploadCloud, X } from '@lucide/vue'
import { useRouter } from 'vue-router'
import { api, unwrapError } from '../api'
import ConfirmDialog from '../components/ConfirmDialog.vue'

const router = useRouter()
const quota = ref({ used_today: 0, reserved: 0, limit: 50, remaining: 50 })
const quotaLoaded = ref(false)
const adminContact = ref({ student_id: '', qq: '' })
const uploads = ref([])
const previewItem = ref(null)
const submitting = ref(false)
const dragging = ref(false)
const message = ref('')
const error = ref('')
const showOverLimitConfirm = ref(false)
let localId = 0

const readyUploads = computed(() => uploads.value.filter(item => item.status === 'ready'))
const isConverting = computed(() => uploads.value.some(item => item.status === 'loading'))
const projectedPages = computed(() =>
  readyUploads.value.reduce((sum, item) => sum + selectedPages(item), 0)
)
const willOverLimit = computed(() => projectedPages.value > quota.value.remaining)
const canSubmit = computed(() => readyUploads.value.length > 0 && !isConverting.value && !submitting.value)
const previewUrl = computed(() => previewItem.value ? `${previewItem.value.preview_url}#zoom=100` : '')
const quotaGreenWidth = computed(() => {
  if (quota.value.limit <= 0) return 0
  return Math.max(0, quota.value.remaining - projectedPages.value) / quota.value.limit * 100
})
const quotaPendingWidth = computed(() => {
  if (quota.value.limit <= 0) return 0
  return Math.min(projectedPages.value, quota.value.remaining) / quota.value.limit * 100
})

onMounted(() => {
  load()
  window.addEventListener('keydown', closeOnEscape)
})

onUnmounted(() => {
  window.removeEventListener('keydown', closeOnEscape)
  uploads.value.forEach(item => item.controller?.abort())
})

async function load() {
  try {
    const [quotaRes, contactRes, uploadsRes] = await Promise.all([
      api.get('/user/quota'),
      api.get('/user/admin-contact'),
      api.get('/print/uploads')
    ])
    quota.value = quotaRes.data
    quotaLoaded.value = true
    adminContact.value = contactRes.data
    restoreUploads(uploadsRes.data.files || [])
  } catch (err) {
    error.value = unwrapError(err)
    quotaLoaded.value = true
  }
}

function restoreUploads(files) {
  const loadingItems = uploads.value.filter(item => item.status === 'loading')
  const restored = files.map(file => ({
    ...file,
    local_id: ++localId,
    odd_even: 'all',
    status: 'ready',
    error: '',
    removed: false,
    controller: null
  }))
  uploads.value = [...restored, ...loadingItems]
}

function selectedPages(item) {
  const total = Math.max(item.page_count || 1, 1)
  if (item.odd_even === 'odd') return Math.ceil(total / 2)
  if (item.odd_even === 'even') return Math.floor(total / 2)
  return total
}

function pickFiles(event) {
  addFiles(event.target.files)
  event.target.value = ''
}

function dropFiles(event) {
  dragging.value = false
  addFiles(event.dataTransfer.files)
}

function addFiles(fileList) {
  const files = Array.from(fileList || [])
  if (!files.length) return
  error.value = ''
  message.value = ''
  for (const source of files) {
    const item = {
      local_id: ++localId,
      original_name: source.name,
      odd_even: 'all',
      page_count: 0,
      status: 'loading',
      error: '',
      removed: false,
      controller: new AbortController()
    }
    uploads.value.push(item)
    uploadOne(item, source)
  }
}

function patchUpload(localId, patch) {
  const index = uploads.value.findIndex(candidate => candidate.local_id === localId)
  if (index === -1) return
  uploads.value.splice(index, 1, { ...uploads.value[index], ...patch })
}

async function uploadOne(item, source) {
  const formData = new FormData()
  formData.append('files', source)
  try {
    const { data } = await api.post('/print/upload', formData, {
      signal: item.controller.signal,
      timeout: 190000
    })
    const uploaded = data.files[0]
    if (!uploaded) throw new Error('服务器未返回上传文件')
    if (item.removed) {
      await deleteTemporaryUpload(uploaded.temp_id)
      return
    }
    patchUpload(item.local_id, { ...uploaded, status: 'ready', controller: null })
  } catch (err) {
    if (item.removed || err?.code === 'ERR_CANCELED') return
    patchUpload(item.local_id, {
      status: 'error',
      error: unwrapError(err),
      controller: null
    })
  }
}

async function removeUpload(item) {
  item.removed = true
  item.controller?.abort()
  uploads.value = uploads.value.filter(candidate => candidate.local_id !== item.local_id)
  if (previewItem.value?.local_id === item.local_id) previewItem.value = null
  if (item.temp_id) await deleteTemporaryUpload(item.temp_id)
}

async function deleteTemporaryUpload(tempId) {
  try {
    await api.delete(`/print/uploads/${tempId}`)
  } catch (err) {
    if (err?.response?.status !== 404) error.value = unwrapError(err)
  }
}

function showPreview(item) {
  if (item.status === 'ready') previewItem.value = item
}

function closeOnEscape(event) {
  if (event.key === 'Escape') previewItem.value = null
}

async function submit() {
  if (!canSubmit.value) return
  if (willOverLimit.value) {
    showOverLimitConfirm.value = true
    return
  }

  await performSubmit()
}

async function performSubmit() {
  showOverLimitConfirm.value = false

  submitting.value = true
  error.value = ''
  try {
    await api.post('/print/submit', {
      files: readyUploads.value.map(file => ({
        temp_id: file.temp_id,
        odd_even: file.odd_even
      }))
    })
    message.value = '任务已提交，可在打印队列中取消尚未开始的任务。'
    uploads.value = []
    previewItem.value = null
    await load()
    router.push('/queue')
  } catch (err) {
    error.value = unwrapError(err)
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <section class="page submit-page">
    <header class="page-header">
      <div>
        <h1>提交打印</h1>
      </div>
      <button class="primary-button" type="button" :disabled="!canSubmit" @click="submit">
        <LoaderCircle v-if="submitting" class="spin" :size="18" />
        <Send v-else :size="18" />
        <span>{{ submitting ? '提交中' : '提交打印' }}</span>
      </button>
    </header>

    <div class="submit-layout">
      <label
        class="dropzone submit-dropzone"
        :class="{ dragging }"
        @drop.prevent="dropFiles"
        @dragenter.prevent="dragging = true"
        @dragleave.prevent="dragging = false"
        @dragover.prevent
      >
        <span class="dropzone-icon"><UploadCloud :size="48" /></span>
        <strong>拖拽文件到这里</strong>
        <span>或点击选择文件</span>
        <small>支持 PDF、Word、Excel、PPT、图片和 TXT，可同时添加多个文件</small>
        <input type="file" multiple hidden @change="pickFiles" />
      </label>

      <aside class="submission-sidebar">
        <div v-if="quotaLoaded" class="quota-progress-card" :class="{ danger: willOverLimit }">
          <div class="quota-progress-heading">
            <span>今日额度</span>
            <strong>{{ quota.remaining }}/{{ quota.limit }} 页</strong>
          </div>
          <div class="quota-track" role="progressbar" :aria-valuenow="quota.remaining" aria-valuemin="0" :aria-valuemax="quota.limit">
            <span class="quota-remaining-segment" :style="{ width: `${quotaGreenWidth}%` }"></span>
            <span class="quota-pending-segment" :style="{ width: `${quotaPendingWidth}%` }"></span>
          </div>
          <div class="quota-legend">
            <span v-if="projectedPages"><i class="pending-dot"></i>本次文件 {{ projectedPages }} 页</span>
          </div>
        </div>
        <div v-else class="quota-progress-card quota-loading">
          <div class="quota-progress-heading">
            <span>今日额度</span>
            <strong>加载中</strong>
          </div>
          <div class="quota-track"></div>
        </div>

        <div class="upload-list">
          <article v-for="file in uploads" :key="file.local_id" class="upload-card">
            <div class="upload-card-heading">
              <button
                class="icon-button preview-button"
                type="button"
                :disabled="file.status !== 'ready'"
                :title="file.status === 'ready' ? '预览' : file.status === 'error' ? file.error : '正在生成预览'"
                @click="showPreview(file)"
              >
                <LoaderCircle v-if="file.status === 'loading'" class="spin" :size="18" />
                <CircleAlert v-else-if="file.status === 'error'" :size="18" />
                <Eye v-else :size="18" />
              </button>
              <div class="file-details">
                <strong :title="file.original_name">{{ file.original_name }}</strong>
                <span v-if="file.status === 'loading'">正在上传并生成预览…</span>
                <span v-else-if="file.status === 'error'" class="danger-text" :title="file.error">{{ file.error }}</span>
                <span v-else>{{ file.page_count }} 页 · 实际打印 {{ selectedPages(file) }} 页</span>
              </div>
              <button class="icon-button remove-button" type="button" title="移出" @click="removeUpload(file)">
                <X :size="18" />
              </button>
            </div>

            <div v-if="file.status === 'ready'" class="page-range-control" :data-selection="file.odd_even">
              <button type="button" :class="{ active: file.odd_even === 'all' }" @click="file.odd_even = 'all'">打印全部</button>
              <button type="button" :class="{ active: file.odd_even === 'odd' }" @click="file.odd_even = 'odd'">仅奇数页</button>
              <button type="button" :disabled="file.page_count < 2" :class="{ active: file.odd_even === 'even' }" @click="file.odd_even = 'even'">仅偶数页</button>
            </div>
          </article>

          <div v-if="!uploads.length" class="empty-upload-list">尚未添加文件</div>
        </div>

        <p v-if="message" class="ok-text">{{ message }}</p>
        <p v-if="error" class="error-text">{{ error }}</p>
      </aside>
    </div>

    <div v-if="previewItem" class="preview-modal" role="dialog" aria-modal="true" @click.self="previewItem = null">
      <section class="preview-dialog">
        <header>
          <strong :title="previewItem.original_name">{{ previewItem.original_name }}</strong>
          <button class="icon-button" type="button" title="关闭预览" @click="previewItem = null"><X :size="18" /></button>
        </header>
        <iframe :src="previewUrl" title="PDF 预览"></iframe>
      </section>
    </div>

    <ConfirmDialog
      v-if="showOverLimitConfirm"
      title="提交超额打印"
      :message="`本次文件会超过今日额度，任务将进入管理员审核。管理员 QQ：${adminContact.qq || '未填写'}，学号：${adminContact.student_id || '未填写'}。`"
      confirm-text="继续提交"
      :busy="submitting"
      @cancel="showOverLimitConfirm = false"
      @confirm="performSubmit"
    />
  </section>
</template>
