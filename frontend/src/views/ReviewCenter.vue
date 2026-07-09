<script setup>
import { onMounted, ref } from 'vue'
import { Check, X } from '@lucide/vue'
import { api } from '../api'

const tasks = ref([])

onMounted(load)

async function load() {
  const { data } = await api.get('/admin/review')
  tasks.value = data
}

async function approve(task) {
  await api.post(`/admin/review/${task.id}/approve`)
  await load()
}

async function reject(task) {
  const reason = window.prompt('拒绝原因', '')
  await api.post(`/admin/review/${task.id}/reject`, { reason })
  await load()
}
</script>

<template>
  <section class="page">
    <header class="page-header">
      <div>
        <h1>审核中心</h1>
        <p>{{ tasks.length }} 个待审核任务</p>
      </div>
    </header>

    <div class="task-grid">
      <article v-for="task in tasks" :key="task.id" class="task-card review-card">
        <div class="task-top">
          <strong>#{{ task.id }}</strong>
          <span>{{ task.owner_name }}</span>
        </div>
        <h2>{{ task.file_name }}</h2>
        <p>{{ task.page_count }} 页 · {{ task.odd_even }}</p>
        <div class="button-row">
          <button class="primary-button" type="button" @click="approve(task)">
            <Check :size="18" />
            <span>同意</span>
          </button>
          <button class="ghost-button danger-text" type="button" @click="reject(task)">
            <X :size="18" />
            <span>拒绝</span>
          </button>
        </div>
      </article>
    </div>
  </section>
</template>
