<script setup>
import { onMounted, ref } from 'vue'
import { Download } from '@lucide/vue'
import { api } from '../api'

const rows = ref([])

onMounted(async () => {
  const { data } = await api.get('/admin/stats')
  rows.value = data
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
        <p>按总页数降序</p>
      </div>
      <button class="primary-button" type="button" @click="exportCsv">
        <Download :size="18" />
        <span>导出 CSV</span>
      </button>
    </header>

    <table class="data-table">
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
