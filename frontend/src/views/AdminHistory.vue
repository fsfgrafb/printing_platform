<script setup>
import { onMounted, ref } from 'vue'
import { ChevronLeft, ChevronRight, Search } from '@lucide/vue'
import { api } from '../api'

const items = ref([])
const page = ref(1)
const perPage = 50
const total = ref(0)
const studentId = ref('')

onMounted(load)

async function load() {
  const { data } = await api.get('/admin/history', {
    params: { page: page.value, per_page: perPage, student_id: studentId.value || undefined }
  })
  items.value = data.items
  total.value = data.total
}

async function search() {
  page.value = 1
  await load()
}

async function move(delta) {
  page.value = Math.max(1, page.value + delta)
  await load()
}
</script>

<template>
  <section class="page">
    <header class="page-header">
      <div>
        <h1>全部打印历史</h1>
        <p>共 {{ total }} 条记录</p>
      </div>
      <div class="button-row">
        <input v-model.trim="studentId" placeholder="按学号筛选" @keyup.enter="search" />
        <button class="ghost-button" type="button" @click="search"><Search :size="18" />查询</button>
        <button class="icon-button" :disabled="page <= 1" title="上一页" @click="move(-1)"><ChevronLeft :size="18" /></button>
        <button class="icon-button" :disabled="page * perPage >= total" title="下一页" @click="move(1)"><ChevronRight :size="18" /></button>
      </div>
    </header>

    <table class="data-table">
      <thead><tr><th>学号</th><th>文件</th><th>页数</th><th>状态</th><th>提交时间</th><th>结束时间</th><th>IP</th><th>说明</th></tr></thead>
      <tbody>
        <tr v-for="item in items" :key="item.id">
          <td>{{ item.student_id }}</td><td>{{ item.file_name }}</td><td>{{ item.page_count }}</td>
          <td><span class="status-pill" :class="item.status">{{ item.status }}</span></td>
          <td>{{ item.submitted_at }}</td><td>{{ item.completed_at || '-' }}</td><td>{{ item.submitted_ip || '-' }}</td><td>{{ item.status_detail || '-' }}</td>
        </tr>
      </tbody>
    </table>
  </section>
</template>
