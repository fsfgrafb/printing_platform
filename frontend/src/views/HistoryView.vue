<script setup>
import { onMounted, ref } from 'vue'
import { ChevronLeft, ChevronRight } from '@lucide/vue'
import { api } from '../api'

const items = ref([])
const page = ref(1)
const perPage = 20
const total = ref(0)

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
        </tr>
      </thead>
      <tbody>
        <tr v-for="item in items" :key="item.id">
          <td>{{ item.file_name }}</td>
          <td>{{ item.page_count }}</td>
          <td><span class="status-pill" :class="item.status">{{ item.status }}</span></td>
          <td>{{ item.submitted_at }}</td>
          <td>{{ item.completed_at || '-' }}</td>
        </tr>
      </tbody>
    </table>
  </section>
</template>
