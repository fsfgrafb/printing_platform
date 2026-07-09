<script setup>
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { FilePlus2, Send, UploadCloud } from '@lucide/vue'
import { api, unwrapError } from '../api'

const router = useRouter()
const quota = ref({ used_today: 0, limit: 50, remaining: 50 })
const adminContact = ref({ student_id: '', qq: '' })
const uploads = ref([])
const selectedPreview = ref('')
const busy = ref(false)
const message = ref('')
const error = ref('')

const projectedPages = computed(() =>
  uploads.value.reduce((sum, item) => sum + selectedPages(item), 0)
)
const willOverLimit = computed(() => quota.value.used_today + projectedPages.value > quota.value.limit)

onMounted(load)

async function load() {
  const [quotaRes, contactRes] = await Promise.all([
    api.get('/user/quota'),
    api.get('/user/admin-contact')
  ])
  quota.value = quotaRes.data
  adminContact.value = contactRes.data
}

function selectedPages(item) {
  const total = Math.max(item.page_count || 1, 1)
  if (item.odd_even === 'odd') return Math.ceil(total / 2)
  if (item.odd_even === 'even') return Math.floor(total / 2)
  return total
}

async function pickFiles(event) {
  await uploadFiles(event.target.files)
  event.target.value = ''
}

async function dropFiles(event) {
  await uploadFiles(event.dataTransfer.files)
}

async function uploadFiles(fileList) {
  const files = Array.from(fileList || [])
  if (!files.length) return
  busy.value = true
  error.value = ''
  const formData = new FormData()
  files.forEach(file => formData.append('files', file))
  try {
    const { data } = await api.post('/print/upload', formData, {
      headers: { 'Content-Type': 'multipart/form-data' }
    })
    uploads.value.push(...data.files.map(file => ({ ...file, odd_even: 'all' })))
    selectedPreview.value = uploads.value[0]?.preview_url || ''
  } catch (err) {
    error.value = unwrapError(err)
  } finally {
    busy.value = false
  }
}

async function submit() {
  if (!uploads.value.length) return
  if (willOverLimit.value) {
    const ok = window.confirm(
      `本次提交会超过今日限额。任务将进入审核，管理员 QQ：${adminContact.value.qq || '未填写'}，学号：${adminContact.value.student_id || '未填写'}。是否继续？`
    )
    if (!ok) return
  }

  busy.value = true
  error.value = ''
  try {
    await api.post('/print/submit', {
      files: uploads.value.map(file => ({
        temp_id: file.temp_id,
        odd_even: file.odd_even
      }))
    })
    message.value = '任务已提交'
    uploads.value = []
    await router.push('/history')
  } catch (err) {
    error.value = unwrapError(err)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <section class="page">
    <header class="page-header">
      <div>
        <h1>提交打印</h1>
        <p>今日已用 {{ quota.used_today }} / {{ quota.limit }} 页，剩余 {{ quota.remaining }} 页</p>
      </div>
      <button class="primary-button" type="button" :disabled="!uploads.length || busy" @click="submit">
        <Send :size="18" />
        <span>提交</span>
      </button>
    </header>

    <div class="split-layout">
      <section class="panel">
        <label class="dropzone" @drop.prevent="dropFiles" @dragover.prevent>
          <UploadCloud :size="34" />
          <strong>{{ busy ? '处理中' : '拖拽或点击上传' }}</strong>
          <span>PDF、Word、Excel、PPT、图片均可加入队列</span>
          <input type="file" multiple hidden @change="pickFiles" />
        </label>

        <div class="quota-line" :class="{ danger: willOverLimit }">
          本次预计 {{ projectedPages }} 页
        </div>

        <article v-for="file in uploads" :key="file.temp_id" class="list-card">
          <button class="icon-button" type="button" title="预览" @click="selectedPreview = file.preview_url">
            <FilePlus2 :size="18" />
          </button>
          <div class="grow">
            <strong>{{ file.original_name }}</strong>
            <span>{{ file.page_count }} 页，实际提交 {{ selectedPages(file) }} 页</span>
          </div>
          <select v-model="file.odd_even">
            <option value="all">全部</option>
            <option value="odd">奇数页</option>
            <option value="even">偶数页</option>
          </select>
        </article>

        <p v-if="message" class="ok-text">{{ message }}</p>
        <p v-if="error" class="error-text">{{ error }}</p>
      </section>

      <section class="preview-panel">
        <iframe v-if="selectedPreview" :src="selectedPreview" title="PDF 预览"></iframe>
        <div v-else class="empty-state">等待上传文件</div>
      </section>
    </div>
  </section>
</template>
