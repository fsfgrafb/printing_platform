<script setup>
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { Download, KeyRound, Plus, Send, Trash2, Upload } from '@lucide/vue'
import { api, unwrapError } from '../api'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import { refreshSession, session } from '../session'

const users = ref([])
const total = ref(0)
const file = ref(null)
const result = ref(null)
const pendingAction = ref(null)
const transferStudentId = ref('')
const showAddUser = ref(false)
const newStudentId = ref('')
const busy = ref(false)
const loaded = ref(false)
const error = ref('')
const router = useRouter()
const importExtensions = new Set(['xlsx', 'xls', 'xlsm', 'csv', 'txt'])

onMounted(load)

async function load() {
  try {
    const { data } = await api.get('/admin/users', { params: { page: 1, per_page: 200 } })
    users.value = data.items
    total.value = data.total
    loaded.value = true
  } catch (err) {
    error.value = unwrapError(err)
    loaded.value = true
  }
}

async function importUsers() {
  if (!file.value) return
  error.value = ''
  try {
    const formData = new FormData()
    formData.append('file', file.value)
    const { data } = await api.post('/admin/users/import', formData)
    result.value = data
    file.value = null
    await load()
  } catch (err) {
    error.value = unwrapError(err)
  }
}

function selectImportFile(event) {
  const selected = event.target.files?.[0] || null
  event.target.value = ''
  if (!selected) return
  const extension = selected.name.split('.').pop()?.toLowerCase()
  if (!extension || !importExtensions.has(extension)) {
    file.value = null
    error.value = '不支持该用户导入文件格式，仅支持 XLSX、XLS、XLSM、CSV 和 TXT'
    return
  }
  error.value = ''
  file.value = selected
}

function exportCsv() {
  window.open('/api/admin/stats.csv', '_blank')
}

async function createUser() {
  const studentId = newStudentId.value.trim()
  if (!studentId) return
  busy.value = true
  error.value = ''
  try {
    await api.post('/admin/users', { student_id: studentId })
    newStudentId.value = ''
    showAddUser.value = false
    await load()
  } catch (err) {
    error.value = unwrapError(err)
  } finally {
    busy.value = false
  }
}

function requestAction(type, user) {
  transferStudentId.value = ''
  pendingAction.value = { type, user }
}

async function confirmAction() {
  if (!pendingAction.value) return
  const { type, user } = pendingAction.value
  busy.value = true
  error.value = ''
  try {
    if (type === 'reset') {
      await api.post(`/admin/users/${user.id}/reset-password`, {})
      if (user.id === session.user?.id) {
        await refreshSession()
        pendingAction.value = null
        await router.replace('/settings')
        return
      }
    } else if (type === 'delete') {
      await api.delete(`/admin/users/${user.id}`)
    } else if (type === 'transfer') {
      await api.post('/admin/transfer', { new_admin_student_id: transferStudentId.value.trim() })
      await refreshSession()
      pendingAction.value = null
      await router.replace('/queue')
      return
    }
    pendingAction.value = null
    await load()
  } catch (err) {
    error.value = unwrapError(err)
  } finally {
    busy.value = false
  }
}

function roleLabel(user) {
  if (user.must_change_password) return '待首次登录'
  return user.role === 'admin' ? '管理员' : '用户'
}
</script>

<template>
  <section class="page">
    <header class="page-header">
      <div>
        <h1>用户管理</h1>
        <p v-if="loaded">共 {{ total }} 个账号</p>
      </div>
      <div class="button-row">
        <button class="ghost-button" type="button" @click="showAddUser = !showAddUser">
          <Plus :size="18" />
          <span>新增用户</span>
        </button>
        <button class="ghost-button" type="button" @click="exportCsv">
          <Download :size="18" />
          <span>导出统计</span>
        </button>
        <label class="file-button">
          <Upload :size="18" />
          <span>{{ file ? file.name : '选择导入文件' }}</span>
          <input type="file" accept=".xlsx,.xls,.xlsm,.csv,.txt" hidden @change="selectImportFile" />
        </label>
        <button class="primary-button" type="button" :disabled="!file" @click="importUsers">导入</button>
      </div>
    </header>

    <p v-if="result" class="ok-text">已新增 {{ result.created.length }} 人，跳过 {{ result.skipped.length }} 人</p>

    <p v-if="!loaded" class="loading-state">正在加载用户</p>

    <section v-if="loaded && showAddUser" class="panel form-grid">
      <label>
        学号
        <input v-model.trim="newStudentId" autocomplete="off" @keyup.enter="createUser" />
      </label>
      <div class="button-row">
        <button class="primary-button" type="button" :disabled="busy || !newStudentId.trim()" @click="createUser">
          <Plus :size="18" />
          <span>新增用户</span>
        </button>
        <button class="ghost-button" type="button" :disabled="busy" @click="showAddUser = false">取消</button>
      </div>
    </section>

    <div v-if="loaded" class="table-scroll">
      <table class="data-table user-table">
        <thead>
          <tr><th>学号</th><th>角色</th><th>QQ</th><th>累计页数</th><th>完成任务</th><th>注册时间</th><th>最后登录</th><th></th></tr>
        </thead>
        <tbody>
          <tr v-for="user in users" :key="user.id">
            <td>{{ user.student_id }}</td>
            <td>{{ roleLabel(user) }}</td>
            <td>{{ user.qq || '-' }}</td>
            <td>{{ user.total_pages }}</td>
            <td>{{ user.total_tasks }}</td>
            <td>{{ user.created_at || '-' }}</td>
            <td>{{ user.last_login_at || '-' }}</td>
            <td class="row-actions">
              <button class="icon-button" type="button" title="重置为学号密码" @click="requestAction('reset', user)">
                <KeyRound :size="18" />
              </button>
              <button
                v-if="user.id === session.user?.id"
                class="icon-button"
                type="button"
                title="转让管理员"
                @click="requestAction('transfer', user)"
              >
                <Send :size="18" />
              </button>
              <button v-else class="icon-button danger-button" type="button" title="删除用户" @click="requestAction('delete', user)">
                <Trash2 :size="18" />
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <ConfirmDialog
      v-if="pendingAction"
      :title="pendingAction.type === 'reset' ? '重置密码' : pendingAction.type === 'delete' ? '删除用户' : '转让管理员'"
      :message="pendingAction.type === 'reset'
        ? `将 ${pendingAction.user.student_id} 的密码重置为其学号，并要求下次登录修改。`
        : pendingAction.type === 'delete'
          ? `将永久删除 ${pendingAction.user.student_id} 及其打印记录和文件。`
          : '只能转让给已有用户。转让后，当前账号将变为普通用户。'"
      :confirm-text="pendingAction.type === 'reset' ? '确认重置' : pendingAction.type === 'delete' ? '确认删除' : '确认转让'"
      :danger="pendingAction.type === 'delete'"
      :input-label="pendingAction.type === 'transfer' ? '新管理员学号' : ''"
      :input-required="pendingAction.type === 'transfer'"
      :input-value="transferStudentId"
      :busy="busy"
      @update:input-value="transferStudentId = $event"
      @cancel="pendingAction = null"
      @confirm="confirmAction"
    />

    <ConfirmDialog
      v-if="error"
      title="操作失败"
      :message="error"
      confirm-text="确定"
      :show-cancel="false"
      @confirm="error = ''"
    />
  </section>
</template>
