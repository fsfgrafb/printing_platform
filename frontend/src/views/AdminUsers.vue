<script setup>
import { onMounted, ref } from 'vue'
import { KeyRound, Trash2, Upload } from '@lucide/vue'
import { api } from '../api'

const users = ref([])
const total = ref(0)
const file = ref(null)
const result = ref(null)

onMounted(load)

async function load() {
  const { data } = await api.get('/admin/users', { params: { page: 1, per_page: 200 } })
  users.value = data.items
  total.value = data.total
}

async function importUsers() {
  if (!file.value) return
  const formData = new FormData()
  formData.append('file', file.value)
  const { data } = await api.post('/admin/users/import', formData)
  result.value = data
  await load()
}

async function resetPassword(user) {
  await api.post(`/admin/users/${user.id}/reset-password`, {})
  await load()
}

async function deleteUser(user) {
  if (!window.confirm(`删除 ${user.student_id}？`)) return
  await api.delete(`/admin/users/${user.id}`)
  await load()
}
</script>

<template>
  <section class="page">
    <header class="page-header">
      <div>
        <h1>用户管理</h1>
        <p>共 {{ total }} 个账号</p>
      </div>
      <label class="file-button">
        <Upload :size="18" />
        <span>导入</span>
        <input type="file" accept=".xlsx,.xls,.csv,.txt" hidden @change="event => file = event.target.files[0]" />
      </label>
      <button class="primary-button" type="button" :disabled="!file" @click="importUsers">确认导入</button>
    </header>

    <p v-if="result" class="ok-text">新增 {{ result.created.length }} 人，跳过 {{ result.skipped.length }} 人</p>

    <table class="data-table">
      <thead>
        <tr>
          <th>学号</th>
          <th>角色</th>
          <th>QQ</th>
          <th>首次改密</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="user in users" :key="user.id">
          <td>{{ user.student_id }}</td>
          <td>{{ user.role }}</td>
          <td>{{ user.qq || '-' }}</td>
          <td>{{ user.must_change_password ? '是' : '否' }}</td>
          <td class="row-actions">
            <button class="icon-button" type="button" title="重置密码" @click="resetPassword(user)">
              <KeyRound :size="18" />
            </button>
            <button class="icon-button danger-button" type="button" title="删除用户" @click="deleteUser(user)">
              <Trash2 :size="18" />
            </button>
          </td>
        </tr>
      </tbody>
    </table>
  </section>
</template>
