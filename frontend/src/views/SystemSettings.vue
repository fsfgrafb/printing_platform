<script setup>
import { onMounted, ref } from 'vue'
import { Save, Send } from '@lucide/vue'
import { api } from '../api'

const config = ref({
  daily_limit: '50',
  admin_qq: '',
  admin_student_id: '',
  queue_paused: false,
  printer_status: ''
})
const newAdmin = ref('')
const message = ref('')

onMounted(load)

async function load() {
  const { data } = await api.get('/admin/config')
  config.value = data
}

async function save(key, value) {
  await api.put('/admin/config', { key, value })
  message.value = '已保存'
  await load()
}

async function transfer() {
  if (!newAdmin.value.trim()) return
  await api.post('/admin/transfer', { new_admin_student_id: newAdmin.value })
  message.value = '管理员已转让'
  await load()
}
</script>

<template>
  <section class="page narrow-page">
    <header class="page-header">
      <div>
        <h1>系统设置</h1>
        <p>打印机状态：{{ config.printer_status }}</p>
      </div>
    </header>

    <section class="panel form-grid">
      <label>
        每日限额
        <input v-model.trim="config.daily_limit" type="number" min="0" />
      </label>
      <button class="primary-button" type="button" @click="save('daily_limit', config.daily_limit)">
        <Save :size="18" />
        <span>保存限额</span>
      </button>
    </section>

    <section class="panel form-grid">
      <label>
        管理员 QQ
        <input v-model.trim="config.admin_qq" />
      </label>
      <label>
        管理员学号
        <input v-model.trim="config.admin_student_id" />
      </label>
      <button class="primary-button" type="button" @click="save('admin_qq', config.admin_qq)">
        <Save :size="18" />
        <span>保存 QQ</span>
      </button>
      <button class="ghost-button" type="button" @click="save('admin_student_id', config.admin_student_id)">
        <Save :size="18" />
        <span>保存学号</span>
      </button>
    </section>

    <section class="panel form-grid">
      <label>
        新管理员学号
        <input v-model.trim="newAdmin" />
      </label>
      <button class="primary-button" type="button" @click="transfer">
        <Send :size="18" />
        <span>转让管理员</span>
      </button>
    </section>

    <p v-if="message" class="ok-text">{{ message }}</p>
  </section>
</template>
