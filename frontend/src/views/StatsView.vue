<script setup>
import { onMounted, ref } from 'vue'
import { Download } from '@lucide/vue'
import { api, unwrapError } from '../api'

const rows = ref([])
const loaded = ref(false)
const error = ref('')

onMounted(async () => {
  try {
    const { data } = await api.get('/admin/stats')
    rows.value = data
  } catch (err) {
    error.value = unwrapError(err)
  } finally {
    loaded.value = true
  }
})

function exportCsv() {
  window.open('/api/admin/stats.csv', '_blank')
}
</script>

<template>
  <section class="page">
    <header class="page-header">
      <div>
        <h1>统计中心</h1>
      </div>
      <button class="primary-button" type="button" @click="exportCsv">
        <Download :size="18" />
        <span>导出 CSV</span>
      </button>
    </header>

    <p v-if="!loaded" class="loading-state">正在加载统计数据</p>
    <p v-if="error" class="error-text">{{ error }}</p>

    <table v-if="loaded" class="data-table">
      <thead>
        <tr>
          <th>学号</th>
          <th>总页数</th>
          <th>总任务数</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="row in rows" :key="row.student_id">
          <td>{{ row.student_id }}</td>
          <td>{{ row.total_pages }}</td>
          <td>{{ row.total_tasks }}</td>
        </tr>
      </tbody>
    </table>
  </section>
</template>
