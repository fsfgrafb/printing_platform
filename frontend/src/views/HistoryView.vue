<script setup>
import { onMounted, ref } from 'vue'
import { ChevronLeft, ChevronRight, X } from '@lucide/vue'
import { api, unwrapError } from '../api'

const items = ref([])
const page = ref(1)
const perPage = 20
const total = ref(0)
const error = ref('')

onMounted(load)

async function load() {
  const { data } = await api.get('/user/history', { params: { page: page.value, per_page: perPage } })
  items.value = data.items
  total.value = data.total
}

async function next(delta) {
  page.value = Math.max(1, page.value + delta)
  await load()
}

async function cancel(item) {
  if (!window.confirm(`取消“${item.file_name}”的打印任务？`)) return
  error.value = ''
  try {
    await api.delete(`/print/tasks/${item.id}`)
    await load()
  } catch (err) {
    error.value = unwrapError(err)
  }
}
</script>

<template>
  <section class="page">
    <header class="page-header">
      <div>
        <h1>我的打印</h1>
        <p>共 {{ total }} 条记录</p>
      </div>
      <div class="button-row">
        <button class="icon-button" type="button" title="上一页" :disabled="page <= 1" @click="next(-1)">
          <ChevronLeft :size="18" />
        </button>
        <button class="icon-button" type="button" title="下一页" :disabled="page * perPage >= total" @click="next(1)">
          <ChevronRight :size="18" />
        </button>
      </div>
    </header>

    <table class="data-table">
      <thead>
        <tr>
          <th>文件</th>
          <th>页数</th>
          <th>状态</th>
          <th>提交时间</th>
          <th>完成时间</th>
          <th>说明</th>
          <th>提交 IP</th>
          <th>操作</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="item in items" :key="item.id">
          <td>{{ item.file_name }}</td>
          <td>{{ item.page_count }}</td>
          <td><span class="status-pill" :class="item.status">{{ item.status }}</span></td>
          <td>{{ item.submitted_at }}</td>
          <td>{{ item.completed_at || '-' }}</td>
          <td>{{ item.status_detail || item.review_reason || '-' }}</td>
          <td>{{ item.submitted_ip || '-' }}</td>
          <td>
            <button
              v-if="['queued', 'pending_review'].includes(item.status)"
              class="icon-button danger-button"
              type="button"
              title="取消任务"
              @click="cancel(item)"
            >
              <X :size="18" />
            </button>
            <span v-else>-</span>
          </td>
        </tr>
      </tbody>
    </table>
    <p v-if="error" class="error-text">{{ error }}</p>
  </section>
</template>
